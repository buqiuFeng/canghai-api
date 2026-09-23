use crate::commands::valid;
use crate::infra::{get_server_url_and_token, post_api_result};
use crate::sync::ApiResult;
use serde::{Deserialize, Serialize};

// ==================== 数据模型 ====================

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Team {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub owner_id: Option<String>,
    #[serde(default, alias = "createdAt")]
    pub create_time: Option<String>,
    #[serde(default, alias = "createdBy")]
    pub create_by: Option<String>,
    #[serde(default, alias = "updatedAt")]
    pub update_time: Option<String>,
    #[serde(default, alias = "updatedBy")]
    pub update_by: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MemberInfo {
    pub id: String,
    #[serde(rename = "userId")]
    pub user_id: String,
    pub username: String,
    pub role: String,
    #[serde(rename = "createTime", alias = "createdAt")]
    pub create_time: Option<String>,
}

// ==================== Tauri 命令 ====================
// Phase 6.6：这些命令返回裸 `ApiResult<T>`，校验失败用 `valid::as_fail` 转成失败信封。

/// 获取当前用户的团队列表
#[tauri::command]
pub async fn get_teams(app: tauri::AppHandle) -> ApiResult<Vec<Team>> {
    let (server_url, token) = match get_server_url_and_token(&app).data {
        Some(v) => v,
        None => return ApiResult::err(401, "请先登录".to_string()),
    };
    let url = format!("{server_url}/api/v1/team/mine");
    let body = serde_json::json!({ "token": token });
    post_api_result::<Vec<Team>>(&url, body).await
}

/// 创建团队
#[tauri::command]
pub async fn create_team(app: tauri::AppHandle, name: String, description: String) -> ApiResult<Team> {
    if let Err(e) = valid::required("name", &name) {
        return valid::as_fail(e);
    }
    let (server_url, token) = match get_server_url_and_token(&app).data {
        Some(v) => v,
        None => return ApiResult::err(401, "请先登录".to_string()),
    };
    let url = format!("{server_url}/api/v1/team/create");
    let body = serde_json::json!({
        "token": token,
        "name": name,
        "description": description,
    });
    post_api_result::<Team>(&url, body).await
}

/// 更新团队
#[tauri::command]
pub async fn update_team(app: tauri::AppHandle, id: String, name: String, description: String) -> ApiResult<()> {
    if let Err(e) = valid::required("id", &id) {
        return valid::as_fail(e);
    }
    if let Err(e) = valid::required("name", &name) {
        return valid::as_fail(e);
    }
    let (server_url, token) = match get_server_url_and_token(&app).data {
        Some(v) => v,
        None => return ApiResult::err(401, "请先登录".to_string()),
    };
    let url = format!("{server_url}/api/v1/team/update");
    let body = serde_json::json!({
        "token": token,
        "id": id,
        "name": name,
        "description": description,
    });
    let _ = post_api_result::<()>(&url, body).await;
    ApiResult::ok(())
}

/// 删除团队
#[tauri::command]
pub async fn delete_team(app: tauri::AppHandle, id: String) -> ApiResult<()> {
    if let Err(e) = valid::required("id", &id) {
        return valid::as_fail(e);
    }
    let (server_url, token) = match get_server_url_and_token(&app).data {
        Some(v) => v,
        None => return ApiResult::err(401, "请先登录".to_string()),
    };
    let url = format!("{server_url}/api/v1/team/delete");
    let body = serde_json::json!({ "token": token, "id": id });
    let _ = post_api_result::<()>(&url, body).await;
    ApiResult::ok(())
}

/// 邀请成员加入团队
#[tauri::command]
pub async fn invite_member(app: tauri::AppHandle, team_id: String, username: String, role: String) -> ApiResult<String> {
    if let Err(e) = valid::required("teamId", &team_id) {
        return valid::as_fail(e);
    }
    if let Err(e) = valid::required("username", &username) {
        return valid::as_fail(e);
    }
    let (server_url, token) = match get_server_url_and_token(&app).data {
        Some(v) => v,
        None => return ApiResult::err(401, "请先登录".to_string()),
    };
    let url = format!("{server_url}/api/v1/team/invite");
    let body = serde_json::json!({
        "token": token,
        "teamId": team_id,
        "username": username,
        "role": role,
    });
    post_api_result::<String>(&url, body).await
}

/// 修改成员角色
#[tauri::command]
pub async fn change_member_role(app: tauri::AppHandle, team_id: String, user_id: String, role: String) -> ApiResult<String> {
    if let Err(e) = valid::required("teamId", &team_id) {
        return valid::as_fail(e);
    }
    if let Err(e) = valid::required("userId", &user_id) {
        return valid::as_fail(e);
    }
    if let Err(e) = valid::required("role", &role) {
        return valid::as_fail(e);
    }
    let (server_url, token) = match get_server_url_and_token(&app).data {
        Some(v) => v,
        None => return ApiResult::err(401, "请先登录".to_string()),
    };
    let url = format!("{server_url}/api/v1/team/changeRole");
    let body = serde_json::json!({
        "token": token,
        "teamId": team_id,
        "userId": user_id,
        "role": role,
    });
    post_api_result::<String>(&url, body).await
}

/// 移除成员
#[tauri::command]
pub async fn remove_member(app: tauri::AppHandle, team_id: String, user_id: String) -> ApiResult<String> {
    if let Err(e) = valid::required("teamId", &team_id) {
        return valid::as_fail(e);
    }
    if let Err(e) = valid::required("userId", &user_id) {
        return valid::as_fail(e);
    }
    let (server_url, token) = match get_server_url_and_token(&app).data {
        Some(v) => v,
        None => return ApiResult::err(401, "请先登录".to_string()),
    };
    let url = format!("{server_url}/api/v1/team/removeMember");
    let body = serde_json::json!({
        "token": token,
        "teamId": team_id,
        "userId": user_id,
    });
    post_api_result::<String>(&url, body).await
}

/// 移交团队所有权
#[tauri::command]
pub async fn transfer_team(app: tauri::AppHandle, team_id: String, new_owner_username: String) -> ApiResult<String> {
    if let Err(e) = valid::required("teamId", &team_id) {
        return valid::as_fail(e);
    }
    if let Err(e) = valid::required("newOwnerUsername", &new_owner_username) {
        return valid::as_fail(e);
    }
    let (server_url, token) = match get_server_url_and_token(&app).data {
        Some(v) => v,
        None => return ApiResult::err(401, "请先登录".to_string()),
    };
    let url = format!("{server_url}/api/v1/team/transfer");
    let body = serde_json::json!({
        "token": token,
        "teamId": team_id,
        "newOwnerUsername": new_owner_username,
    });
    post_api_result::<String>(&url, body).await
}

/// 获取团队成员列表
#[tauri::command]
pub async fn get_team_members(app: tauri::AppHandle, team_id: String) -> ApiResult<Vec<MemberInfo>> {
    if let Err(e) = valid::required("teamId", &team_id) {
        return valid::as_fail(e);
    }
    let (server_url, token) = match get_server_url_and_token(&app).data {
        Some(v) => v,
        None => return ApiResult::err(401, "请先登录".to_string()),
    };
    let url = format!("{server_url}/api/v1/team/members");
    let body = serde_json::json!({
        "token": token,
        "id": team_id,
    });
    post_api_result::<Vec<MemberInfo>>(&url, body).await
}
