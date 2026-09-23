use crate::db::{self, AppDb, Category};
use crate::commands::{resolve_conn, valid};
use crate::sync::ApiResult;
use tauri::State;

#[tauri::command]
pub fn get_categories(
    project_id: String,
    data_mode: Option<String>,
    db: State<'_, AppDb>,
) -> Result<ApiResult<Vec<Category>>, String> {
    let (conn, mode) = resolve_conn(&db, data_mode);
    Ok(db::get_categories(&conn, &project_id, mode)
        .map(ApiResult::ok)
        .unwrap_or_else(ApiResult::from_db_err))
}

#[tauri::command]
pub fn save_category(
    category: Category,
    data_mode: Option<String>,
    db: State<'_, AppDb>,
) -> Result<ApiResult<()>, String> {
    if let Err(e) = valid::check(&category) {
        return Ok(e);
    }
    if let Err(e) = valid::required("projectId", &category.project_id) {
        return Ok(e);
    }
    let (conn, mode) = resolve_conn(&db, data_mode);
    Ok(db::save_category(&conn, &category, mode)
        .map(ApiResult::ok)
        .unwrap_or_else(ApiResult::from_db_err))
}

#[tauri::command]
pub fn update_category(
    id: String,
    name: String,
    data_mode: Option<String>,
    db: State<'_, AppDb>,
) -> Result<ApiResult<()>, String> {
    if let Err(e) = valid::required("id", &id) {
        return Ok(e);
    }
    if let Err(e) = valid::required("name", &name) {
        return Ok(e);
    }
    let (conn, mode) = resolve_conn(&db, data_mode);
    Ok(db::update_category_db(&conn, &id, &name, mode)
        .map(ApiResult::ok)
        .unwrap_or_else(ApiResult::from_db_err))
}

#[tauri::command]
pub fn set_category_expanded(
    id: String,
    expanded: bool,
    data_mode: Option<String>,
    db: State<'_, AppDb>,
) -> Result<ApiResult<()>, String> {
    if let Err(e) = valid::required("id", &id) {
        return Ok(e);
    }
    let (conn, mode) = resolve_conn(&db, data_mode);
    Ok(db::update_category_expanded(&conn, &id, expanded, mode)
        .map(ApiResult::ok)
        .unwrap_or_else(ApiResult::from_db_err))
}

#[tauri::command]
pub fn delete_category(
    id: String,
    data_mode: Option<String>,
    db: State<'_, AppDb>,
) -> Result<ApiResult<()>, String> {
    if let Err(e) = valid::required("id", &id) {
        return Ok(e);
    }
    let (conn, mode) = resolve_conn(&db, data_mode);
    Ok(db::delete_category_db(&conn, &id, mode)
        .map(ApiResult::ok)
        .unwrap_or_else(ApiResult::from_db_err))
}
