use crate::db::{Project, AppDb};
use crate::db::{
    get_all_projects as db_get_projects,
    get_projects_by_ids as db_get_projects_by_ids,
    save_project as db_save_project,
    update_project_db,
    update_project_expanded,
    delete_project_db,
};
use crate::commands::{resolve_conn, valid};
use crate::infra::{get_server_url_and_token, load_config, post_api_result};
use crate::sync::ApiResult;
use log::{info, warn, error};
use serde::Deserialize;
use tauri::Manager;

#[tauri::command]
pub async fn get_projects(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppDb>,
    data_mode: Option<String>,
) -> Result<ApiResult<Vec<Project>>, String> {
    let (conn, mode) = resolve_conn(&state, data_mode);
    // 在线模式：从当前请求的 token 推导 user_id（JWT sub），不依赖前端传入。
    // 取不到时回退到持久化 SyncConfig.user_id（兼容旧随机 token 透传方案）。
    let uid = if mode == crate::db::DataMode::Online {
        let (server_url, token) = match get_server_url_and_token(&app).data {
            Some(v) => v,
            None => {
                error!("[get_projects] 未登录：取不到 server_url/token");
                return Ok(ApiResult::err(401, "请先登录".to_string()));
            }
        };
        info!("[get_projects] online token 获取成功, server_url={}, token_len={}", server_url, token.len());

        let token_uid = crate::jwt::extract_user_id(&token);
        info!("[get_projects] JWT 解析 userId = {:?}", token_uid);

        let cfg_uid = {
            let app_data_dir = app.path().app_data_dir().ok();
            app_data_dir.map(|d| load_config(&d).user_id)
        };
        info!("[get_projects] SyncConfig.user_id = {:?}", cfg_uid);

        match token_uid.filter(|s| !s.is_empty()) {
            Some(u) => u,
            None => match cfg_uid {
                Some(u) if !u.is_empty() => u,
                _ => {
                    warn!("[get_projects] 在线模式无法解析出 user_id（token 非 JWT 且 SyncConfig.user_id 为空），返回空列表");
                    return Ok(ApiResult::ok(Vec::new()));
                }
            },
        }
    } else {
        String::new()
    };
    info!("[get_projects] 最终用于过滤的 user_id = {:?}, mode = {:?}", uid, mode);
    // 离线模式：不按 user_id 过滤（等同于 "*" 分支）
    if mode == crate::db::DataMode::Offline || uid.is_empty() || uid == "*" {
        return match db_get_projects(&conn, mode, &uid) {
            Ok(v) => {
                info!("[get_projects] 离线/全量查询返回 {} 个项目", v.len());
                Ok(ApiResult::ok(v))
            }
            Err(e) => Ok(ApiResult::from_db_err(e)),
        };
    }
    // 在线模式：先取 owner 项目
    let mut owner = match db_get_projects(&conn, mode, &uid) {
        Ok(v) => {
            info!("[get_projects] owner 项目查询返回 {} 个（user_id={}）", v.len(), uid);
            v
        }
        Err(e) => return Ok(ApiResult::from_db_err(e)),
    };
    for p in owner.iter_mut() {
        p.current_user_role = "owner".to_string();
    }
    // 再补充“作为成员可见”的项目（user 直接成员 / 团队间接成员），来自 server
    if let Some((server_url, token)) = get_server_url_and_token(&app).data {
        // 后端该端点声明为 @PostMapping("/visible")，因此必须用 POST。
        // （此前用 get_api_result 发 GET，方法不匹配会拿不到数据。）
        // 请求体虽无业务参数，但后端 AuthAspect 从 body 的 token 字段鉴权，故仍需携带 token。
        let url = format!("{server_url}/api/v1/project-member/visible");
        let body = serde_json::json!({ "token": token });
        if let Some(visibles) = post_api_result::<Vec<ProjectVisibility>>(&url, body).await.data {
            if !visibles.is_empty() {
                let ids: Vec<String> = visibles.iter().map(|v| v.project_id.clone()).collect();
                if let Ok(member_projects) = db_get_projects_by_ids(&conn, mode, &ids) {
                    let existing: std::collections::HashSet<String> =
                        owner.iter().map(|p| p.id.clone()).collect();
                    for mut p in member_projects {
                        if existing.contains(&p.id) {
                            continue;
                        }
                        if let Some(v) = visibles.iter().find(|v| v.project_id == p.id) {
                            p.current_user_role = v.role.clone();
                        }
                        owner.push(p);
                    }
                }
            }
        }
    }
    Ok(ApiResult::ok(owner))
}

/// server 返回的“项目可见性”条目
/// 注意：server 端 JSON 字段为 camelCase（projectId/role），必须显式声明 rename，
/// 否则 project_id 反序列化为空串，导致后续按 id 查本地项目时查不到，成员看不到项目。
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProjectVisibility {
    project_id: String,
    role: String,
}

#[tauri::command]
pub async fn save_project(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppDb>,
    project: Project,
    data_mode: Option<String>,
) -> Result<ApiResult<()>, String> {
    if let Err(e) = valid::check(&project) {
        return Ok(e);
    }
    let (conn, mode) = resolve_conn(&state, data_mode);
    // 本地写入（作为缓存 / 离线兜底）
    if let Err(e) = db_save_project(&conn, &project, mode) {
        return Ok(ApiResult::from_db_err(e));
    }
    // 在线模式：实时保存到后端
    if mode == crate::db::DataMode::Online {
        let r = server_save_project(&app, &project).await;
        if !r.is_success() {
            return Ok(r);
        }
    }
    Ok(ApiResult::ok(()))
}

#[tauri::command]
pub async fn update_project(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppDb>,
    id: String,
    name: String,
    update_by: Option<String>,
    data_mode: Option<String>,
) -> Result<ApiResult<()>, String> {
    if let Err(e) = valid::required("id", &id) {
        return Ok(e);
    }
    if let Err(e) = valid::required("name", &name) {
        return Ok(e);
    }
    let (conn, mode) = resolve_conn(&state, data_mode);
    let updater = update_by.unwrap_or_else(|| "local".to_string());
    // 本地写入
    if let Err(e) = update_project_db(&conn, &id, &name, &updater, mode) {
        return Ok(ApiResult::from_db_err(e));
    }
    // 在线模式：实时保存到后端
    if mode == crate::db::DataMode::Online {
        let mut proj = Project {
            id: id.clone(),
            name: name.clone(),
            user_id: String::new(),
            parent_id: None,
            sort_order: 0,
            expanded: false,
            create_time: String::new(),
            create_by: String::new(),
            update_time: String::new(),
            update_by: updater.clone(),
            deleted: false,
            current_user_role: String::new(),
        };
        // 重新读取完整项目以同步到后端（项目本地无 description/team_id 字段，仅同步 name 等）
        if let Ok(all) = db_get_projects_by_ids(&conn, mode, &[id.clone()]) {
            if let Some(p) = all.into_iter().next() {
                proj.user_id = p.user_id;
                proj.parent_id = p.parent_id;
                proj.sort_order = p.sort_order;
                proj.expanded = p.expanded;
                proj.create_time = p.create_time;
                proj.create_by = p.create_by;
                proj.update_time = p.update_time;
            }
        }
        let r = server_save_project(&app, &proj).await;
        if !r.is_success() {
            return Ok(r);
        }
    }
    Ok(ApiResult::ok(()))
}

#[tauri::command]
pub async fn set_project_expanded(
    state: tauri::State<'_, AppDb>,
    id: String,
    expanded: bool,
    data_mode: Option<String>,
) -> Result<ApiResult<()>, String> {
    if let Err(e) = valid::required("id", &id) {
        return Ok(e);
    }
    let (conn, mode) = resolve_conn(&state, data_mode);
    match update_project_expanded(&conn, &id, expanded, mode) {
        Ok(_) => Ok(ApiResult::ok(())),
        Err(e) => Ok(ApiResult::from_db_err(e)),
    }
}

#[tauri::command]
pub async fn delete_project(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppDb>,
    id: String,
    data_mode: Option<String>,
) -> Result<ApiResult<()>, String> {
    if let Err(e) = valid::required("id", &id) {
        return Ok(e);
    }
    let (conn, mode) = resolve_conn(&state, data_mode);
    // 本地删除
    if let Err(e) = delete_project_db(&conn, &id, mode) {
        return Ok(ApiResult::from_db_err(e));
    }
    // 在线模式：实时删除后端
    if mode == crate::db::DataMode::Online {
        let r = server_delete_project(&app, &id).await;
        if !r.is_success() {
            return Ok(r);
        }
    }
    Ok(ApiResult::ok(()))
}

/// 实时保存项目到后端（在线模式）。teamId 取当前同步团队，确保项目在同步下载中可见。
async fn server_save_project(app: &tauri::AppHandle, project: &Project) -> ApiResult<()> {
    let (server_url, token) = match get_server_url_and_token(app).data {
        Some(v) => v,
        None => return ApiResult::err(401, "请先登录".to_string()),
    };
    let app_data_dir = match app.path().app_data_dir() {
        Ok(d) => d,
        Err(e) => return ApiResult::err(-1, format!("获取数据目录失败: {e}")),
    };
    let cfg = load_config(&app_data_dir);
    let team_id = cfg.team_id.clone();
    let url = format!("{server_url}/api/v1/project/save");
    let body = serde_json::json!({
        "token": token,
        "id": project.id,
        "name": project.name,
        "teamId": team_id,
    });
    let _ = post_api_result::<serde_json::Value>(&url, body).await;
    ApiResult::ok(())
}

/// 实时删除项目（后端软删除）
async fn server_delete_project(app: &tauri::AppHandle, id: &str) -> ApiResult<()> {
    let (server_url, token) = match get_server_url_and_token(app).data {
        Some(v) => v,
        None => return ApiResult::err(401, "请先登录".to_string()),
    };
    let url = format!("{server_url}/api/v1/project/delete");
    let body = serde_json::json!({
        "token": token,
        "id": id,
    });
    let _ = post_api_result::<serde_json::Value>(&url, body).await;
    ApiResult::ok(())
}
