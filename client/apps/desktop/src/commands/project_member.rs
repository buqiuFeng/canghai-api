use crate::db::{self, AppDb, DataMode, ProjectMemberInfo};
use crate::infra::{get_server_url_and_token, post_api_result};
use crate::sync::ApiResult;

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct RemoteMemberInfo {
    id: String,
    project_id: String,
    member_type: String,
    member_id: String,
    #[serde(default, alias = "memberName")]
    member_name: String,
    role: String,
    #[serde(default, alias = "createTime")]
    create_time: String,
}

/// 查询项目成员（在线模式走服务端 API，离线模式查本地库）
///
/// 注意：因命令输入含 `State` 引用，按 Tauri 约束外层须包 `Result`，
/// 业务统一返回结构 ApiResult<T> 放在内层。
#[tauri::command]
pub async fn get_project_members(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppDb>,
    data_mode: Option<String>,
    project_id: String,
) -> Result<ApiResult<Vec<ProjectMemberInfo>>, String> {
    let (conn, mode) = crate::commands::resolve_conn(&state, data_mode);

    if mode == DataMode::Offline {
        return Ok(db::get_project_members_db(&conn, &project_id, mode)
            .map(ApiResult::ok)
            .unwrap_or_else(ApiResult::from_db_err));
    }

    let (server_url, token) = match get_server_url_and_token(&app).data {
        Some(v) => v,
        None => return Ok(ApiResult::err(401, "未登录".to_string())),
    };
    let url = format!("{server_url}/api/v1/project-member/list");
    let body = serde_json::json!({
        "token": token,
        "projectId": project_id,
    });

    match post_api_result::<Vec<RemoteMemberInfo>>(&url, body).await {
        r if r.is_success() => {
            let list = match r.data {
                Some(l) => l,
                None => return Ok(ApiResult::err(r.code, r.msg)),
            };
            let mapped: Vec<ProjectMemberInfo> = list
                .into_iter()
                .map(|m| ProjectMemberInfo {
                    id: m.id,
                    project_id: m.project_id,
                    member_type: m.member_type,
                    member_id: m.member_id,
                    member_name: m.member_name,
                    role: m.role,
                    create_time: m.create_time,
                })
                .collect();
            for m in &mapped {
                let _ = db::save_project_member_db(&conn, m, mode);
            }
            Ok(ApiResult::ok(mapped))
        }
        r => {
            eprintln!("拉取项目成员失败: {}", r.msg);
            Ok(db::get_project_members_db(&conn, &project_id, mode)
                .map(ApiResult::ok)
                .unwrap_or_else(ApiResult::from_db_err))
        }
    }
}

/// 添加项目成员（仅在线模式）
#[tauri::command]
pub async fn add_project_member(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppDb>,
    data_mode: Option<String>,
    project_id: String,
    member_type: String,
    member_id: String,
    role: String,
) -> Result<ApiResult<()>, String> {
    let (conn, mode) = crate::commands::resolve_conn(&state, data_mode);
    if mode == DataMode::Offline {
        return Ok(ApiResult::err(-1, "离线模式暂不支持项目成员管理".to_string()));
    }

    let (server_url, token) = match get_server_url_and_token(&app).data {
        Some(v) => v,
        None => return Ok(ApiResult::err(401, "未登录".to_string())),
    };
    let url = format!("{server_url}/api/v1/project-member/add");
    // 团队类型成员权限继承自团队本身，不单独配置角色
    let effective_role = if member_type == "team" { "inherit".to_string() } else { role.clone() };
    let body = serde_json::json!({
        "token": token,
        "projectId": project_id,
        "memberType": member_type,
        "memberId": member_id,
        "role": effective_role,
    });

    let r = post_api_result::<serde_json::Value>(&url, body).await;
    if !r.is_success() {
        return Ok(ApiResult::err(r.code, r.msg));
    }

    // 同步写入本地缓存
    // 名称落库：user 类型入参即用户名，team 类型入参是团队 id（名称随后的列表刷新会补全）
    let now = db::now_timestamp();
    let local_name = if member_type == "user" { member_id.clone() } else { String::new() };
    let m = ProjectMemberInfo {
        id: format!("{}_{}_{}", project_id, member_type, member_id),
        project_id,
        member_type,
        member_id,
        member_name: local_name,
        role: effective_role,
        create_time: now,
    };
    let _ = db::save_project_member_db(&conn, &m, mode);
    Ok(ApiResult::ok(()))
}

/// 移除项目成员（仅在线模式）
#[tauri::command]
pub async fn remove_project_member(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppDb>,
    data_mode: Option<String>,
    project_id: String,
    member_type: String,
    member_id: String,
) -> Result<ApiResult<()>, String> {
    let (conn, mode) = crate::commands::resolve_conn(&state, data_mode);
    if mode == DataMode::Offline {
        return Ok(ApiResult::err(-1, "离线模式暂不支持项目成员管理".to_string()));
    }

    let (server_url, token) = match get_server_url_and_token(&app).data {
        Some(v) => v,
        None => return Ok(ApiResult::err(401, "未登录".to_string())),
    };
    let url = format!("{server_url}/api/v1/project-member/remove");
    let body = serde_json::json!({
        "token": token,
        "projectId": project_id,
        "memberType": member_type,
        "memberId": member_id,
    });

    let r = post_api_result::<serde_json::Value>(&url, body).await;
    if !r.is_success() {
        return Ok(ApiResult::err(r.code, r.msg));
    }

    let _ = db::remove_project_member_db(&conn, &project_id, &member_type, &member_id, mode);
    Ok(ApiResult::ok(()))
}
