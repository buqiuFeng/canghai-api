//! 本地 SQLite 数据访问层（由原 `db.rs` 拆分而来，Phase 8.1）。
//!
//! 对外路径保持 `crate::db::*` 不变：各子模块的公开条目由本文件 `pub use` 重新导出，
//! 因此调用方（commands / sync / infra）零改动。
//!
//! 模块划分：
//! - `models`：实体与枚举（serde 形状即三端契约）
//! - `schema`：建表 DDL、初始化与轻量迁移
//! - `crud`：按实体拆分的读写（在线/离线通过 `data_mode` 列隔离）
//! - `sync`：增量同步所需的合并/脏标记/事务辅助

mod crud;
mod models;
mod schema;
mod sync;

pub use crud::*;
pub use models::*;
pub use schema::*;
pub use sync::*;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

/// 数据模式：在线（默认，走后端/可同步）与离线（独立本地库，完全隔离）
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DataMode {
    Online,
    Offline,
}

impl Default for DataMode {
    fn default() -> Self {
        DataMode::Online
    }
}

impl DataMode {
    pub fn from_str(s: &str) -> DataMode {
        match s.to_lowercase().as_str() {
            "offline" => DataMode::Offline,
            _ => DataMode::Online,
        }
    }

    /// 落库用的字符串值：'online' / 'offline'。
    pub fn as_db_value(&self) -> &'static str {
        match self {
            DataMode::Online => "online",
            DataMode::Offline => "offline",
        }
    }
}


/// 单库持有：在线/离线数据通过 `data_mode` 列逻辑区分，不再使用独立物理文件。
/// 由 `lib.rs` 在 setup 时注入为 Tauri 托管状态。
pub struct AppDb {
    pub conn: DbConn,
}


/// 数据库层统一错误类型，相比裸 `String` 提供了稳定的错误分类，
/// 便于命令层（commands）按需转换为对用户友好的提示。
#[derive(Debug)]
pub enum DbError {
    /// 锁竞争失败（理论上不应发生，但需兜底）
    LockPoisoned,
    /// SQL 编译/执行失败，携带底层 rusqlite 描述
    Sql(rusqlite::Error),
    /// 文件系统或 pragma 初始化失败
    Init(String),
    /// 迁移脚本执行失败
    Migration(String),
}

impl From<rusqlite::Error> for DbError {
    fn from(e: rusqlite::Error) -> Self {
        DbError::Sql(e)
    }
}

impl DbError {
    /// 映射为统一业务错误码（见 `docs/API_CONTRACT.md` §3）。
    ///
    /// 原先所有 DB 失败都被命令层 `.to_string()` 后压成 `-1`，前端无法区分
    /// 「数据库坏了」与「参数不对」。此处按错误类别给出可判别整数码。
    pub fn code(&self) -> i32 {
        match self {
            // 初始化失败通常意味着运行环境/权限问题，归入服务端内部错误
            DbError::Init(_) => crate::sync::codes::INTERNAL_ERROR,
            // 锁破坏、SQL 执行、迁移失败均属本地数据库错误
            DbError::LockPoisoned | DbError::Sql(_) | DbError::Migration(_) => {
                crate::sync::codes::DB_ERROR
            }
        }
    }
}

impl std::fmt::Display for DbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DbError::LockPoisoned => write!(f, "数据库锁已被破坏，请重启应用"),
            DbError::Sql(e) => write!(f, "数据库操作失败: {e}"),
            DbError::Init(e) => write!(f, "{e}"),
            DbError::Migration(e) => write!(f, "{e}"),
        }
    }
}


/// 取锁的统一入口：所有 DB 访问都需先拿到互斥锁，集中处理锁破坏这一
/// 边界情况，避免在十几个函数里重复写同样的 map_err。
pub(crate) fn lock_db<'a>(conn: &'a DbConn) -> Result<std::sync::MutexGuard<'a, Connection>, DbError> {
    conn.lock().map_err(|_| DbError::LockPoisoned)
}


pub type DbConn = Mutex<Connection>;


/// 历史记录保留上限。原逻辑硬编码在 `save_history_item` 的清理 SQL 中，
/// 提取为常量避免散落魔法数字。
pub const MAX_HISTORY: usize = 20;


/// 应用统一时区：Asia/Shanghai（UTC+8，中国不使用夏令时，固定偏移）。
///
/// 刻意使用**固定偏移**而不是 `chrono::Local`：`Local` 取的是宿主机系统时区，
/// 而时间基准必须与 Java 端 / MySQL 会话时区一致，不能随运行环境漂移
/// （跨时区协作、CI 容器、用户手动改系统时区都会让当地时区不再是 +8）。
const APP_TZ_OFFSET_SECONDS: i32 = 8 * 60 * 60;

/// 统一的当前时间字符串（**Asia/Shanghai / UTC+8**，`YYYY-MM-DD HH:MM:SS`，秒级）。
///
/// 这是 Rust 端**唯一**的时间生成点（全仓仅此一处格式化时间，也从不解析时间字符串），
/// 时区契约固定为北京时间：
/// - Java `BaseService.now()`、前端 `utils.now()` 同样输出北京时间；
/// - MySQL 连接会话时区被钉在 Asia/Shanghai，`server_update_time`
///   （`CURRENT_TIMESTAMP(3)`）与同步游标 `NOW(3)` 出自同一时钟；
/// - 三端的时间戳只做**字符串字典序**比较（同步合并、冲突判定），因此任何一端混入
///   别的时区都会产生固定偏移（UTC 与北京时间相差 8 小时），表现为「本地永远赢 / 永远输」。
///
/// 契约由本文件末尾的 `now_timestamp_is_shanghai` 测试固化，请勿改成宿主机本地时区。
pub fn now_timestamp() -> String {
    let tz = chrono::FixedOffset::east_opt(APP_TZ_OFFSET_SECONDS)
        .expect("UTC+8 是合法偏移");
    chrono::Utc::now()
        .with_timezone(&tz)
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::{now_timestamp, APP_TZ_OFFSET_SECONDS};

    /// 时区契约回归测试：`now_timestamp()` 必须输出 **Asia/Shanghai（UTC+8）**。
    ///
    /// 三端统一以北京时间存时间字符串，同步合并 / 冲突判定都基于字符串比较；
    /// 若有人把它改回 UTC 或宿主机本地时区，同一行数据会凭空多出/少掉一个时区偏移
    /// （8 小时），而症状（同步结果异常）很难反推到原因。
    #[test]
    fn now_timestamp_is_shanghai() {
        let s = now_timestamp();
        let parsed = chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S")
            .unwrap_or_else(|e| panic!("时间格式应为 YYYY-MM-DD HH:MM:SS，实际 `{s}`：{e}"));

        // 必须与 UTC+8 的当前时间一致（允许跨秒的调度抖动）
        let expect = chrono::Utc::now() + chrono::Duration::seconds(APP_TZ_OFFSET_SECONDS as i64);
        let delta = (expect.naive_utc() - parsed).num_seconds().abs();
        assert!(delta <= 2, "now_timestamp() 偏离 UTC+8 共 {delta} 秒");
    }
}
