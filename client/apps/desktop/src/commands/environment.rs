use crate::db::{Environment, EnvironmentGroup, EnvironmentVariable, AppDb};
use crate::db::{
    get_all_environments as db_get_environments,
    get_all_environment_groups as db_get_env_groups,
    get_env_variables as db_get_env_vars,
    get_active_env_variables as db_get_active_vars,
    save_environment as db_save_env,
    update_environment as db_update_env,
    delete_environment_db,
    set_active_environment as db_set_active_env,
    save_env_variable as db_save_var,
    update_env_variable as db_update_var,
    delete_env_variable_db,
    save_environment_group as db_save_env_group,
    update_environment_group_db as db_update_env_group,
    update_environment_group_expanded as db_update_env_group_expanded,
    delete_environment_group_db as db_delete_env_group,
};
use crate::commands::{resolve_conn, valid};
use crate::sync::ApiResult;

#[tauri::command]
pub fn get_environments(
    state: tauri::State<'_, AppDb>,
    project_id: Option<String>,
    data_mode: Option<String>,
) -> Result<ApiResult<Vec<Environment>>, String> {
    let (conn, mode) = resolve_conn(&state, data_mode);
    Ok(db_get_environments(&conn, project_id.as_deref(), mode, false)
        .map(ApiResult::ok)
        .unwrap_or_else(ApiResult::from_db_err))
}

#[tauri::command]
pub fn get_env_variables(
    state: tauri::State<'_, AppDb>,
    environment_id: String,
    data_mode: Option<String>,
) -> Result<ApiResult<Vec<EnvironmentVariable>>, String> {
    let (conn, mode) = resolve_conn(&state, data_mode);
    Ok(db_get_env_vars(&conn, &environment_id, mode, false)
        .map(ApiResult::ok)
        .unwrap_or_else(ApiResult::from_db_err))
}

#[tauri::command]
pub fn get_active_env_variables(
    state: tauri::State<'_, AppDb>,
    project_id: Option<String>,
    data_mode: Option<String>,
) -> Result<ApiResult<Vec<EnvironmentVariable>>, String> {
    let (conn, mode) = resolve_conn(&state, data_mode);
    Ok(db_get_active_vars(&conn, project_id.as_deref(), mode)
        .map(ApiResult::ok)
        .unwrap_or_else(ApiResult::from_db_err))
}

#[tauri::command]
pub fn save_environment(
    state: tauri::State<'_, AppDb>,
    environment: Environment,
    data_mode: Option<String>,
) -> Result<ApiResult<()>, String> {
    if let Err(e) = valid::check(&environment) {
        return Ok(e);
    }
    let (conn, mode) = resolve_conn(&state, data_mode);
    Ok(db_save_env(&conn, &environment, mode)
        .map(ApiResult::ok)
        .unwrap_or_else(ApiResult::from_db_err))
}

#[tauri::command]
pub fn update_environment(
    state: tauri::State<'_, AppDb>,
    environment: Environment,
    data_mode: Option<String>,
) -> Result<ApiResult<()>, String> {
    if let Err(e) = valid::required("id", &environment.id) {
        return Ok(e);
    }
    if let Err(e) = valid::check(&environment) {
        return Ok(e);
    }
    let (conn, mode) = resolve_conn(&state, data_mode);
    Ok(db_update_env(&conn, &environment, mode)
        .map(ApiResult::ok)
        .unwrap_or_else(ApiResult::from_db_err))
}

#[tauri::command]
pub fn delete_environment(
    state: tauri::State<'_, AppDb>,
    id: String,
    data_mode: Option<String>,
) -> Result<ApiResult<()>, String> {
    if let Err(e) = valid::required("id", &id) {
        return Ok(e);
    }
    let (conn, mode) = resolve_conn(&state, data_mode);
    Ok(delete_environment_db(&conn, &id, mode)
        .map(ApiResult::ok)
        .unwrap_or_else(ApiResult::from_db_err))
}

#[tauri::command]
pub fn activate_environment(
    state: tauri::State<'_, AppDb>,
    id: String,
    project_id: Option<String>,
    data_mode: Option<String>,
) -> Result<ApiResult<()>, String> {
    if let Err(e) = valid::required("id", &id) {
        return Ok(e);
    }
    let (conn, mode) = resolve_conn(&state, data_mode);
    Ok(db_set_active_env(&conn, &id, project_id.as_deref(), mode)
        .map(ApiResult::ok)
        .unwrap_or_else(ApiResult::from_db_err))
}

#[tauri::command]
pub fn save_env_variable(
    state: tauri::State<'_, AppDb>,
    variable: EnvironmentVariable,
    data_mode: Option<String>,
) -> Result<ApiResult<()>, String> {
    if let Err(e) = valid::check(&variable) {
        return Ok(e);
    }
    let (conn, mode) = resolve_conn(&state, data_mode);
    Ok(db_save_var(&conn, &variable, mode)
        .map(ApiResult::ok)
        .unwrap_or_else(ApiResult::from_db_err))
}

#[tauri::command]
pub fn update_env_variable(
    state: tauri::State<'_, AppDb>,
    variable: EnvironmentVariable,
    data_mode: Option<String>,
) -> Result<ApiResult<()>, String> {
    if let Err(e) = valid::required("id", &variable.id) {
        return Ok(e);
    }
    if let Err(e) = valid::check(&variable) {
        return Ok(e);
    }
    let (conn, mode) = resolve_conn(&state, data_mode);
    Ok(db_update_var(&conn, &variable, mode)
        .map(ApiResult::ok)
        .unwrap_or_else(ApiResult::from_db_err))
}

#[tauri::command]
pub fn delete_env_variable(
    state: tauri::State<'_, AppDb>,
    id: String,
    data_mode: Option<String>,
) -> Result<ApiResult<()>, String> {
    if let Err(e) = valid::required("id", &id) {
        return Ok(e);
    }
    let (conn, mode) = resolve_conn(&state, data_mode);
    Ok(delete_env_variable_db(&conn, &id, mode)
        .map(ApiResult::ok)
        .unwrap_or_else(ApiResult::from_db_err))
}

#[tauri::command]
pub fn get_environment_groups(
    state: tauri::State<'_, AppDb>,
    project_id: Option<String>,
    data_mode: Option<String>,
) -> Result<ApiResult<Vec<EnvironmentGroup>>, String> {
    let (conn, mode) = resolve_conn(&state, data_mode);
    Ok(db_get_env_groups(&conn, project_id.as_deref(), mode, false)
        .map(ApiResult::ok)
        .unwrap_or_else(ApiResult::from_db_err))
}

#[tauri::command]
pub fn save_environment_group(
    state: tauri::State<'_, AppDb>,
    group: EnvironmentGroup,
    data_mode: Option<String>,
) -> Result<ApiResult<()>, String> {
    if let Err(e) = valid::check(&group) {
        return Ok(e);
    }
    let (conn, mode) = resolve_conn(&state, data_mode);
    Ok(db_save_env_group(&conn, &group, mode)
        .map(ApiResult::ok)
        .unwrap_or_else(ApiResult::from_db_err))
}

#[tauri::command]
pub fn update_environment_group(
    state: tauri::State<'_, AppDb>,
    group: EnvironmentGroup,
    data_mode: Option<String>,
) -> Result<ApiResult<()>, String> {
    if let Err(e) = valid::required("id", &group.id) {
        return Ok(e);
    }
    if let Err(e) = valid::check(&group) {
        return Ok(e);
    }
    let (conn, mode) = resolve_conn(&state, data_mode);
    Ok(db_update_env_group(&conn, &group, mode)
        .map(ApiResult::ok)
        .unwrap_or_else(ApiResult::from_db_err))
}

#[tauri::command]
pub fn delete_environment_group(
    state: tauri::State<'_, AppDb>,
    id: String,
    data_mode: Option<String>,
) -> Result<ApiResult<()>, String> {
    if let Err(e) = valid::required("id", &id) {
        return Ok(e);
    }
    let (conn, mode) = resolve_conn(&state, data_mode);
    Ok(db_delete_env_group(&conn, &id, mode)
        .map(ApiResult::ok)
        .unwrap_or_else(ApiResult::from_db_err))
}

/// 纯本地切换分组展开状态（折叠/展开）。不置 dirty，不会触发下一次同步上传。
#[tauri::command]
pub fn set_environment_group_expanded(
    state: tauri::State<'_, AppDb>,
    id: String,
    expanded: bool,
    data_mode: Option<String>,
) -> Result<ApiResult<()>, String> {
    if let Err(e) = valid::required("id", &id) {
        return Ok(e);
    }
    let (conn, mode) = resolve_conn(&state, data_mode);
    Ok(db_update_env_group_expanded(&conn, &id, expanded, mode)
        .map(ApiResult::ok)
        .unwrap_or_else(ApiResult::from_db_err))
}
