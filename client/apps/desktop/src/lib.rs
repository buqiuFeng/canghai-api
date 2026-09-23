mod db;
mod models;
mod commands;
mod sync;
mod infra;
mod jwt;

use std::error::Error;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> Result<(), Box<dyn Error>> {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("获取应用数据目录失败: {e}"))?;
            let db_path = app_data_dir.join("canghai.db");
            // 单库：在线/离线数据通过 data_mode 列逻辑区分（不再使用独立物理文件）。
            // 存量离线数据（canghai_offline.db）按需求不保留、不合并，直接删除旧文件。
            let offline_path = app_data_dir.join("canghai_offline.db");
            if offline_path.exists() {
                let _ = std::fs::remove_file(&offline_path);
            }
            let conn = db::init(&db_path)
                .map_err(|e| format!("初始化数据库失败: {e}"))?;
            app.manage(db::AppDb { conn });
            app.manage(commands::ws::WsManager::default());
            app.manage(commands::mock::MockServerState::default());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // HTTP
            commands::http::send_http_request,
            commands::http::send_http_request_stream,
            // 项目（扁平，无多级）
            commands::project::get_projects,
            commands::project::save_project,
            commands::project::update_project,
            commands::project::set_project_expanded,
            commands::project::delete_project,
            // 项目成员
            commands::project_member::get_project_members,
            commands::project_member::add_project_member,
            commands::project_member::remove_project_member,
            // 分类（项目下多级）
            commands::category::get_categories,
            commands::category::save_category,
            commands::category::update_category,
            commands::category::set_category_expanded,
            commands::category::delete_category,
            // 历史
            commands::history::get_history,
            commands::history::get_history_limit,
            commands::history::save_history,
            commands::history::delete_history_item,
            commands::history::clear_history,
            // 请求
            commands::saved_request::get_saved_requests,
            commands::saved_request::save_saved_request,
            commands::saved_request::update_saved_request,
            commands::saved_request::delete_saved_request,
            commands::saved_request::list_request_versions,
            commands::saved_request::get_request_version_snapshot,
            commands::saved_request::restore_request_version,
            // 环境
            commands::environment::get_environments,
            commands::environment::get_env_variables,
            commands::environment::get_active_env_variables,
            commands::environment::save_environment,
            commands::environment::update_environment,
            commands::environment::delete_environment,
            commands::environment::activate_environment,
            commands::environment::save_env_variable,
            commands::environment::update_env_variable,
            commands::environment::delete_env_variable,
            commands::environment::get_environment_groups,
            commands::environment::save_environment_group,
            commands::environment::update_environment_group,
            commands::environment::set_environment_group_expanded,
            commands::environment::delete_environment_group,
            // 同步
            commands::sync::get_sync_config,
            commands::sync::set_sync_config,
            commands::sync::sync_data,
            commands::sync::pull_data,
            commands::sync::check_server_connection,
            commands::sync::login,
            commands::sync::register,
            commands::sync::logout,
            commands::sync::get_server_base_url,
            commands::sync::call_server_api,
            commands::import::import_collection,
            // 团队
            commands::team::get_teams,
            commands::team::create_team,
            commands::team::update_team,
            commands::team::delete_team,
            commands::team::invite_member,
            commands::team::change_member_role,
            commands::team::remove_member,
            commands::team::transfer_team,
            commands::team::get_team_members,
            // WebSocket
            commands::ws::ws_connect,
            commands::ws::ws_send,
            commands::ws::ws_disconnect,
            // Mock Server
            commands::mock::start_mock_server,
            commands::mock::stop_mock_server,
            commands::mock::mock_server_status,
        ])
        .run(tauri::generate_context!())
        .map_err(|e| -> Box<dyn Error> { Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())) })?;
    Ok(())
}
