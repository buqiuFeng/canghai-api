pub mod http;
pub mod valid;
pub mod project;
pub mod project_member;
pub mod category;
pub mod history;
pub mod saved_request;
pub mod environment;
pub mod sync;
pub mod import;
pub mod team;
pub mod ws;
pub mod mock;

use crate::db::{AppDb, DataMode, DbConn};
use tauri::State;

/// 从命令参数解析数据模式（前端不传时默认在线），返回单库连接与当前模式。
/// 所有数据类命令统一通过此函数取连接；单库下在线/离线通过 data_mode 列逻辑区分。
pub(crate) fn resolve_conn<'a>(
    db: &'a State<'a, AppDb>,
    data_mode: Option<String>,
) -> (&'a DbConn, DataMode) {
    let mode = data_mode
        .as_deref()
        .map(DataMode::from_str)
        .unwrap_or_default();
    (&db.conn, mode)
}

