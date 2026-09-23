use crate::db::{HistoryItem, AppDb, MAX_HISTORY};
use crate::db::{
    get_all_history as db_get_history,
    save_history_item as db_save_history,
    delete_history_item_db,
    clear_all_history as db_clear_history,
};
use crate::commands::resolve_conn;
use crate::sync::ApiResult;

#[tauri::command]
pub fn get_history(state: tauri::State<'_, AppDb>, data_mode: Option<String>) -> Result<ApiResult<Vec<HistoryItem>>, String> {
    let (conn, mode) = resolve_conn(&state, data_mode);
    Ok(db_get_history(&conn, MAX_HISTORY as i32, mode)
        .map(ApiResult::ok)
        .unwrap_or_else(ApiResult::from_db_err))
}

/// 历史上限（后端为单一事实源）。
///
/// 前端启动时拉取一次并据此裁剪本地列表，避免「Rust 常量 20 / 前端 MAX_HISTORY=20」
/// 两处硬编码各自漂移。返回以 `ApiResult` 包裹，与其它命令保持一致的解包约定。
#[tauri::command]
pub fn get_history_limit() -> Result<ApiResult<usize>, String> {
    Ok(ApiResult::ok(MAX_HISTORY))
}

#[tauri::command]
pub fn save_history(state: tauri::State<'_, AppDb>, item: HistoryItem, data_mode: Option<String>) -> Result<ApiResult<()>, String> {
    let (conn, mode) = resolve_conn(&state, data_mode);
    Ok(db_save_history(&conn, &item, mode)
        .map(ApiResult::ok)
        .unwrap_or_else(ApiResult::from_db_err))
}

#[tauri::command]
pub fn delete_history_item(state: tauri::State<'_, AppDb>, id: String, data_mode: Option<String>) -> Result<ApiResult<()>, String> {
    let (conn, mode) = resolve_conn(&state, data_mode);
    Ok(delete_history_item_db(&conn, &id, mode)
        .map(ApiResult::ok)
        .unwrap_or_else(ApiResult::from_db_err))
}

#[tauri::command]
pub fn clear_history(state: tauri::State<'_, AppDb>, data_mode: Option<String>) -> Result<ApiResult<()>, String> {
    let (conn, mode) = resolve_conn(&state, data_mode);
    Ok(db_clear_history(&conn, mode)
        .map(ApiResult::ok)
        .unwrap_or_else(ApiResult::from_db_err))
}
