//! 建表 DDL、初始化与轻量迁移。
//!
//! `init` 负责建表（幂等）与补齐历史库缺失的列；新增列一律通过 `migrate_add_column`，
//! 保证老版本库升级时不丢数据。

use crate::db::*;
use rusqlite::{Connection, params};
use std::path::Path;

pub fn init(db_path: &Path) -> Result<DbConn, DbError> {
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| DbError::Init(format!("创建数据库目录失败: {e}")))?;
    }

    let conn = Connection::open(db_path).map_err(|e| DbError::Init(format!("打开数据库失败: {e}")))?;

    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
        .map_err(|e| DbError::Init(format!("设置 pragma 失败: {e}")))?;

    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS ch_projects (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL DEFAULT '',
            name TEXT NOT NULL,
            parent_id TEXT,
            sort_order INTEGER NOT NULL DEFAULT 0,
            expanded INTEGER NOT NULL DEFAULT 1,
            create_time TEXT NOT NULL DEFAULT '',
            create_by TEXT NOT NULL DEFAULT '',
            update_time TEXT NOT NULL DEFAULT '',
            update_by TEXT NOT NULL DEFAULT '',
            deleted INTEGER NOT NULL DEFAULT 0,
            data_mode TEXT NOT NULL DEFAULT 'online'
        );

        CREATE TABLE IF NOT EXISTS ch_history (
            id TEXT PRIMARY KEY,
            method TEXT NOT NULL,
            url TEXT NOT NULL DEFAULT '',
            params TEXT NOT NULL DEFAULT '[]',
            headers TEXT NOT NULL DEFAULT '[]',
            body_type TEXT NOT NULL DEFAULT 'none',
            body TEXT NOT NULL DEFAULT '',
            form_body TEXT NOT NULL DEFAULT '[]',
            pre_script TEXT NOT NULL DEFAULT '',
            post_script TEXT NOT NULL DEFAULT '',
            category_id TEXT,
            project_id TEXT,
            status INTEGER,
            create_time TEXT NOT NULL,
            data_mode TEXT NOT NULL DEFAULT 'online'
        );

        CREATE INDEX IF NOT EXISTS idx_history_create_time ON ch_history(create_time DESC);
        CREATE INDEX IF NOT EXISTS idx_categories_parent ON ch_projects(parent_id);

        CREATE TABLE IF NOT EXISTS ch_categories (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            name TEXT NOT NULL,
            parent_id TEXT,
            sort_order INTEGER NOT NULL DEFAULT 0,
            expanded INTEGER NOT NULL DEFAULT 1,
            create_time TEXT NOT NULL DEFAULT '',
            update_time TEXT NOT NULL DEFAULT '',
            create_by TEXT NOT NULL DEFAULT '',
            update_by TEXT NOT NULL DEFAULT '',
            deleted INTEGER NOT NULL DEFAULT 0,
            data_mode TEXT NOT NULL DEFAULT 'online'
        );

        CREATE INDEX IF NOT EXISTS idx_categories_project ON ch_categories(project_id);
        CREATE INDEX IF NOT EXISTS idx_categories_parent ON ch_categories(parent_id);

        CREATE TABLE IF NOT EXISTS ch_saved_requests (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL DEFAULT '',
            name TEXT NOT NULL,
            method TEXT NOT NULL,
            url TEXT NOT NULL DEFAULT '',
            params TEXT NOT NULL DEFAULT '[]',
            headers TEXT NOT NULL DEFAULT '[]',
            body_type TEXT NOT NULL DEFAULT 'none',
            body TEXT NOT NULL DEFAULT '',
            form_body TEXT NOT NULL DEFAULT '[]',
            category_id TEXT,
            project_id TEXT,
            pre_script TEXT NOT NULL DEFAULT '',
            post_script TEXT NOT NULL DEFAULT '',
            sort_order INTEGER NOT NULL DEFAULT 0,
            create_time TEXT NOT NULL,
            create_by TEXT NOT NULL DEFAULT '',
            update_time TEXT NOT NULL,
            update_by TEXT NOT NULL DEFAULT '',
            version INTEGER NOT NULL DEFAULT 1,
            deleted INTEGER NOT NULL DEFAULT 0,
            data_mode TEXT NOT NULL DEFAULT 'online'
        );

        CREATE INDEX IF NOT EXISTS idx_saved_req_category ON ch_saved_requests(category_id);
        CREATE INDEX IF NOT EXISTS idx_saved_req_project ON ch_saved_requests(project_id);

        CREATE TABLE IF NOT EXISTS ch_request_versions (
            id TEXT PRIMARY KEY,
            request_id TEXT NOT NULL,
            version INTEGER NOT NULL,
            snapshot TEXT NOT NULL,
            -- 这两列刻意**不给默认值**：快照写入必须显式提供时间，
            -- 漏写即立刻报错（见 requests.rs 的回归测试），而不是静默写入空串。
            create_time TEXT NOT NULL,
            update_time TEXT NOT NULL,
            deleted INTEGER NOT NULL DEFAULT 0,
            data_mode TEXT NOT NULL DEFAULT 'online'
        );

        CREATE INDEX IF NOT EXISTS idx_req_versions_req ON ch_request_versions(request_id, version DESC);

        CREATE TABLE IF NOT EXISTS ch_environment_groups (
            id TEXT PRIMARY KEY,
            project_id TEXT,
            name TEXT NOT NULL,
            sort_order INTEGER NOT NULL DEFAULT 0,
            deleted INTEGER NOT NULL DEFAULT 0,
            expanded INTEGER NOT NULL DEFAULT 1,
            create_time TEXT NOT NULL DEFAULT '',
            create_by TEXT NOT NULL DEFAULT '',
            update_time TEXT NOT NULL DEFAULT '',
            update_by TEXT NOT NULL DEFAULT '',
            data_mode TEXT NOT NULL DEFAULT 'online',
            server_update_time TEXT NOT NULL DEFAULT ''
        );

        CREATE TABLE IF NOT EXISTS ch_environments (
            id TEXT PRIMARY KEY,
            project_id TEXT,
            name TEXT NOT NULL,
            group_id TEXT,
            is_active INTEGER NOT NULL DEFAULT 0,
            create_time TEXT NOT NULL DEFAULT '',
            create_by TEXT NOT NULL DEFAULT '',
            update_time TEXT NOT NULL DEFAULT '',
            update_by TEXT NOT NULL DEFAULT '',
            deleted INTEGER NOT NULL DEFAULT 0,
            data_mode TEXT NOT NULL DEFAULT 'online',
            server_update_time TEXT NOT NULL DEFAULT ''
        );

        CREATE TABLE IF NOT EXISTS ch_teams (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT NOT NULL DEFAULT '',
            owner_id TEXT NOT NULL DEFAULT '',
            user_id TEXT NOT NULL DEFAULT '',
            create_time TEXT NOT NULL DEFAULT '',
            create_by TEXT NOT NULL DEFAULT '',
            update_time TEXT NOT NULL DEFAULT '',
            update_by TEXT NOT NULL DEFAULT '',
            deleted INTEGER NOT NULL DEFAULT 0,
            data_mode TEXT NOT NULL DEFAULT 'online'
        );

        CREATE TABLE IF NOT EXISTS ch_team_members (
            id TEXT PRIMARY KEY,
            team_id TEXT NOT NULL,
            user_id TEXT NOT NULL,
            role TEXT NOT NULL DEFAULT 'readwrite',
            create_time TEXT NOT NULL DEFAULT '',
            create_by TEXT NOT NULL DEFAULT '',
            update_time TEXT NOT NULL DEFAULT '',
            update_by TEXT NOT NULL DEFAULT '',
            deleted INTEGER NOT NULL DEFAULT 0,
            data_mode TEXT NOT NULL DEFAULT 'online'
        );
        CREATE INDEX IF NOT EXISTS idx_tm_team ON ch_team_members(team_id);

        CREATE TABLE IF NOT EXISTS ch_project_members (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            member_type TEXT NOT NULL DEFAULT 'user',
            member_id TEXT NOT NULL,
            member_name TEXT NOT NULL DEFAULT '',
            role TEXT NOT NULL DEFAULT 'readwrite',
            create_time TEXT NOT NULL DEFAULT '',
            create_by TEXT NOT NULL DEFAULT '',
            update_time TEXT NOT NULL DEFAULT '',
            update_by TEXT NOT NULL DEFAULT '',
            deleted INTEGER NOT NULL DEFAULT 0,
            data_mode TEXT NOT NULL DEFAULT 'online'
        );
        CREATE INDEX IF NOT EXISTS idx_pm_project ON ch_project_members(project_id);
        CREATE INDEX IF NOT EXISTS idx_pm_member ON ch_project_members(member_type, member_id);

        CREATE TABLE IF NOT EXISTS ch_environment_variables (
            id TEXT PRIMARY KEY,
            environment_id TEXT NOT NULL,
            key TEXT NOT NULL,
            value TEXT NOT NULL DEFAULT '',
            enabled INTEGER NOT NULL DEFAULT 1,
            sort_order INTEGER NOT NULL DEFAULT 0,
            deleted INTEGER NOT NULL DEFAULT 0,
            create_time TEXT NOT NULL DEFAULT '',
            create_by TEXT NOT NULL DEFAULT '',
            update_time TEXT NOT NULL DEFAULT '',
            update_by TEXT NOT NULL DEFAULT '',
            data_mode TEXT NOT NULL DEFAULT 'online'
        );

        CREATE INDEX IF NOT EXISTS idx_env_vars_env ON ch_environment_variables(environment_id);",
    )
    .map_err(|e| DbError::Init(format!("初始化表结构失败: {e}")))?;

    // 轻量迁移：为旧版本数据库补充后增列（已存在则跳过）
    migrate_add_column(&conn, "ch_projects", "user_id", "TEXT NOT NULL DEFAULT ''")?;
    migrate_add_column(&conn, "ch_projects", "create_time", "TEXT NOT NULL DEFAULT ''")?;
    migrate_add_column(&conn, "ch_projects", "create_by", "TEXT NOT NULL DEFAULT ''")?;
    migrate_add_column(&conn, "ch_projects", "update_time", "TEXT NOT NULL DEFAULT ''")?;
    migrate_add_column(&conn, "ch_projects", "update_by", "TEXT NOT NULL DEFAULT ''")?;
    // 项目也纳入软删体系：与其余同步实体一致（删除走 deleted = 1，读路径统一过滤）
    migrate_add_column(&conn, "ch_projects", "deleted", "INTEGER NOT NULL DEFAULT 0")?;
    migrate_add_column(&conn, "ch_saved_requests", "user_id", "TEXT NOT NULL DEFAULT ''")?;
    migrate_add_column(&conn, "ch_saved_requests", "pre_script", "TEXT NOT NULL DEFAULT ''")?;
    migrate_add_column(&conn, "ch_saved_requests", "post_script", "TEXT NOT NULL DEFAULT ''")?;
    migrate_add_column(&conn, "ch_saved_requests", "create_by", "TEXT NOT NULL DEFAULT ''")?;
    migrate_add_column(&conn, "ch_saved_requests", "update_by", "TEXT NOT NULL DEFAULT ''")?;
    // 历史条目补齐脚本列与项目列（修复「脚本落库即丢」「历史无法按项目隔离」）
    migrate_add_column(&conn, "ch_history", "pre_script", "TEXT NOT NULL DEFAULT ''")?;
    migrate_add_column(&conn, "ch_history", "post_script", "TEXT NOT NULL DEFAULT ''")?;
    migrate_add_column(&conn, "ch_history", "project_id", "TEXT")?;

    // 项目成员名称快照（服务端 ch_project_members.member_name 下发）：
    // 本地无用户表，名称只能靠落库快照，否则离线成员列表会退回显示 member_id（UUID）。
    migrate_add_column(&conn, "ch_project_members", "member_name", "TEXT NOT NULL DEFAULT ''")?;

    // 环境/分组/变量表补齐 create_time/create_by/update_time/update_by（INSERT 依赖这些列）
    migrate_add_column(&conn, "ch_environment_groups", "create_time", "TEXT NOT NULL DEFAULT ''")?;
    migrate_add_column(&conn, "ch_environment_groups", "create_by", "TEXT NOT NULL DEFAULT ''")?;
    migrate_add_column(&conn, "ch_environment_groups", "update_time", "TEXT NOT NULL DEFAULT ''")?;
    migrate_add_column(&conn, "ch_environment_groups", "update_by", "TEXT NOT NULL DEFAULT ''")?;
    migrate_add_column(&conn, "ch_environments", "create_time", "TEXT NOT NULL DEFAULT ''")?;
    migrate_add_column(&conn, "ch_environments", "create_by", "TEXT NOT NULL DEFAULT ''")?;
    migrate_add_column(&conn, "ch_environments", "update_time", "TEXT NOT NULL DEFAULT ''")?;
    migrate_add_column(&conn, "ch_environments", "update_by", "TEXT NOT NULL DEFAULT ''")?;
    migrate_add_column(&conn, "ch_environment_variables", "create_time", "TEXT NOT NULL DEFAULT ''")?;
    migrate_add_column(&conn, "ch_environment_variables", "create_by", "TEXT NOT NULL DEFAULT ''")?;
    migrate_add_column(&conn, "ch_environment_variables", "update_time", "TEXT NOT NULL DEFAULT ''")?;
    migrate_add_column(&conn, "ch_environment_variables", "update_by", "TEXT NOT NULL DEFAULT ''")?;

    // 服务端数据修改时间快照（用于编辑冲突检测）
    migrate_add_column(&conn, "ch_categories", "server_update_time", "TEXT NOT NULL DEFAULT ''")?;
    migrate_add_column(&conn, "ch_saved_requests", "server_update_time", "TEXT NOT NULL DEFAULT ''")?;
    migrate_add_column(&conn, "ch_environment_variables", "server_update_time", "TEXT NOT NULL DEFAULT ''")?;
    migrate_add_column(&conn, "ch_environments", "server_update_time", "TEXT NOT NULL DEFAULT ''")?;
    migrate_add_column(&conn, "ch_environment_groups", "server_update_time", "TEXT NOT NULL DEFAULT ''")?;

    // Phase 4 增量同步：
    // - version：服务端版本号快照（服务端变更时自增），本地编辑不改，上传时原样带回；
    // - dirty：本地未上传标记。存量行默认 1（首次同步全量上传一次），
    //   本地写入置 1，服务端数据合并后置 0，同步成功后统一清零。
    for t in SYNC_TABLES {
        migrate_add_column(&conn, t, "sync_version", "INTEGER NOT NULL DEFAULT 1")?;
        migrate_add_column(&conn, t, "dirty", "INTEGER NOT NULL DEFAULT 1")?;
    }

    // 清理历史冗余列：ch_categories / ch_saved_requests / ch_request_versions 曾冗余保存 team_id。
    // 数据按项目归属，团队经项目成员表关联，本地库无需按 team_id 隔离；该列在读写中均未被使用。
    // 老版本库升级时删除，新建库在上面的 CREATE TABLE 中已不含此列（存在性守卫保证幂等）。
    migrate_drop_column(&conn, "ch_categories", "team_id")?;
    migrate_drop_column(&conn, "ch_saved_requests", "team_id")?;
    migrate_drop_column(&conn, "ch_request_versions", "team_id")?;

    Ok(Mutex::new(conn))
    }


/// 轻量迁移：表缺少列时自动 ALTER TABLE ADD COLUMN（仅新增列，不动已有数据）
fn migrate_add_column(conn: &Connection, table: &str, column: &str, ddl: &str) -> Result<(), DbError> {
    let exists: bool = conn
        .prepare_cached("SELECT 1 FROM pragma_table_info(?1) WHERE name = ?2")?
        .exists(params![table, column])?;
    if !exists {
        conn.execute_batch(&format!("ALTER TABLE {table} ADD COLUMN {column} {ddl};"))
            .map_err(|e| DbError::Init(format!("迁移 {table}.{column} 失败: {e}")))?;
    }
    Ok(())
}

/// 轻量迁移：表存在指定列时删除（SQLite ≥ 3.35 的 `ALTER TABLE ... DROP COLUMN`）。
///
/// 仅用于清理不被索引 / 触发器 / 视图引用的冗余列。列不存在则跳过（幂等），
/// 因此无论是「新建库（建表已不含该列）」还是「老库升级」都能安全运行。
fn migrate_drop_column(conn: &Connection, table: &str, column: &str) -> Result<(), DbError> {
    let exists: bool = conn
        .prepare_cached("SELECT 1 FROM pragma_table_info(?1) WHERE name = ?2")?
        .exists(params![table, column])?;
    if exists {
        conn.execute_batch(&format!("ALTER TABLE {table} DROP COLUMN {column};"))
            .map_err(|e| DbError::Init(format!("迁移删除 {table}.{column} 失败: {e}")))?;
    }
    Ok(())
}

// ====== 分类 CRUD ======
