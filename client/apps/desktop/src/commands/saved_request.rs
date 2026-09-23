use crate::db::{SavedRequest, AppDb};
use crate::db::{
    get_all_saved_requests as db_get_saved_requests,
    save_saved_request as db_save_saved_request,
    update_saved_request as db_update_saved_request,
    delete_saved_request_db,
    get_request_versions as db_get_request_versions,
    get_request_version_snapshot as db_get_request_version_snapshot,
    restore_request_version as db_restore_request_version,
};
use crate::commands::{resolve_conn, valid};
use crate::sync::ApiResult;

#[tauri::command]
pub fn get_saved_requests(state: tauri::State<'_, AppDb>, project_id: Option<String>, data_mode: Option<String>) -> Result<ApiResult<Vec<SavedRequest>>, String> {
    let (conn, mode) = resolve_conn(&state, data_mode);
    let ids: Vec<String> = project_id
        .filter(|p| !p.trim().is_empty())
        .map(|p| vec![p])
        .unwrap_or_default();
    Ok(db_get_saved_requests(&conn, &ids, mode, false)
        .map(ApiResult::ok)
        .unwrap_or_else(ApiResult::from_db_err))
}

#[tauri::command]
pub fn save_saved_request(state: tauri::State<'_, AppDb>, request: SavedRequest, data_mode: Option<String>) -> Result<ApiResult<()>, String> {
    if let Err(e) = valid::check(&request) {
        return Ok(e);
    }
    let (conn, mode) = resolve_conn(&state, data_mode);
    Ok(db_save_saved_request(&conn, &request, mode)
        .map(ApiResult::ok)
        .unwrap_or_else(ApiResult::from_db_err))
}

#[tauri::command]
pub fn update_saved_request(state: tauri::State<'_, AppDb>, request: SavedRequest, data_mode: Option<String>) -> Result<ApiResult<()>, String> {
    if let Err(e) = valid::required("id", &request.id) {
        return Ok(e);
    }
    if let Err(e) = valid::check(&request) {
        return Ok(e);
    }
    let (conn, mode) = resolve_conn(&state, data_mode);
    Ok(db_update_saved_request(&conn, &request, mode)
        .map(ApiResult::ok)
        .unwrap_or_else(ApiResult::from_db_err))
}

#[tauri::command]
pub fn delete_saved_request(state: tauri::State<'_, AppDb>, id: String, data_mode: Option<String>) -> Result<ApiResult<()>, String> {
    if let Err(e) = valid::required("id", &id) {
        return Ok(e);
    }
    let (conn, mode) = resolve_conn(&state, data_mode);
    Ok(delete_saved_request_db(&conn, &id, mode)
        .map(ApiResult::ok)
        .unwrap_or_else(ApiResult::from_db_err))
}

/// 查询某请求的所有版本
#[tauri::command]
pub fn list_request_versions(state: tauri::State<'_, AppDb>, request_id: String, data_mode: Option<String>) -> Result<ApiResult<Vec<crate::db::RequestVersionMeta>>, String> {
    if let Err(e) = valid::required("requestId", &request_id) {
        return Ok(valid::as_fail(e));
    }
    let (conn, mode) = resolve_conn(&state, data_mode);
    Ok(db_get_request_versions(&conn, &request_id, mode)
        .map(ApiResult::ok)
        .unwrap_or_else(ApiResult::from_db_err))
}

/// 读取指定版本的完整快照（版本对比用）
#[tauri::command]
pub fn get_request_version_snapshot(state: tauri::State<'_, AppDb>, request_id: String, version: i32, data_mode: Option<String>) -> Result<ApiResult<SavedRequest>, String> {
    if let Err(e) = valid::required("requestId", &request_id) {
        return Ok(valid::as_fail(e));
    }
    let (conn, mode) = resolve_conn(&state, data_mode);
    Ok(db_get_request_version_snapshot(&conn, &request_id, version, mode)
        .map(ApiResult::ok)
        .unwrap_or_else(ApiResult::from_db_err))
}

/// 恢复到指定版本（不生成新版本，当前版本号直接指向该历史版本）
#[tauri::command]
pub fn restore_request_version(state: tauri::State<'_, AppDb>, request_id: String, version: i32, data_mode: Option<String>) -> Result<ApiResult<SavedRequest>, String> {
    if let Err(e) = valid::required("requestId", &request_id) {
        return Ok(valid::as_fail(e));
    }
    let (conn, mode) = resolve_conn(&state, data_mode);
    Ok(db_restore_request_version(&conn, &request_id, version, mode)
        .map(ApiResult::ok)
        .unwrap_or_else(ApiResult::from_db_err))
}
