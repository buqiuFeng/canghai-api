//! 环境 / 环境分组 / 环境变量的本地读写实现（由原 `db.rs` 拆分，Phase 8.1）。

use crate::db::*;
use rusqlite::params;

pub fn get_all_environment_groups(conn: &DbConn, project_id: Option<&str>, mode: DataMode, only_dirty: bool) -> Result<Vec<EnvironmentGroup>, DbError> {
    let db = lock_db(conn)?;
    let has_pid = project_id.map(|p| !p.is_empty()).unwrap_or(false);
    let dirty_flag: i64 = if only_dirty { 1 } else { 0 };
    let mut stmt = if has_pid {
        db.prepare_cached("SELECT id, project_id, name, sort_order, expanded, create_time, create_by, update_time, update_by, server_update_time, sync_version FROM ch_environment_groups WHERE project_id = ?1 AND data_mode = ?2 AND deleted = 0 AND (?3 = 0 OR dirty = 1) ORDER BY sort_order, name")?
    } else {
        db.prepare_cached("SELECT id, project_id, name, sort_order, expanded, create_time, create_by, update_time, update_by, server_update_time, sync_version FROM ch_environment_groups WHERE (project_id IS NULL OR project_id = '') AND data_mode = ?1 AND deleted = 0 AND (?2 = 0 OR dirty = 1) ORDER BY sort_order, name")?
    };
    let rows = if has_pid {
        stmt.query_map(params![project_id.unwrap(), mode.as_db_value(), dirty_flag], map_env_group)
    } else {
        stmt.query_map(params![mode.as_db_value(), dirty_flag], map_env_group)
    }
    .map_err(DbError::Sql)?;
    let mut items = Vec::new();
    for r in rows {
        items.push(r.map_err(DbError::Sql)?);
    }
    Ok(items)
}


/// 同步专用：一次性取出**全部**环境分组（含无项目归属行与各项目归属行）。
///
/// 原 `collect_all` 按项目逐个查询（N+1），100 个项目即 100+ 次查询。
/// 同步是团队级全量收集，直接一次性取全表即可。
pub fn get_all_environment_groups_all(conn: &DbConn, mode: DataMode, only_dirty: bool) -> Result<Vec<EnvironmentGroup>, DbError> {
    let db = lock_db(conn)?;
    let dirty_flag: i64 = if only_dirty { 1 } else { 0 };
    let mut stmt = db.prepare_cached("SELECT id, project_id, name, sort_order, expanded, create_time, create_by, update_time, update_by, server_update_time, sync_version FROM ch_environment_groups WHERE data_mode = ?1 AND deleted = 0 AND (?2 = 0 OR dirty = 1) ORDER BY sort_order, name")?;
    let rows = stmt
        .query_map(params![mode.as_db_value(), dirty_flag], map_env_group)
        .map_err(DbError::Sql)?;
    let mut items = Vec::new();
    for r in rows {
        items.push(r.map_err(DbError::Sql)?);
    }
    Ok(items)
}


fn map_env_group(row: &rusqlite::Row<'_>) -> Result<EnvironmentGroup, rusqlite::Error> {
    Ok(EnvironmentGroup {
        id: row.get(0)?,
        project_id: row.get(1)?,
        name: row.get(2)?,
        sort_order: row.get(3)?,
        expanded: row.get::<_, bool>(4)?,
        create_time: row.get::<_, String>(5).unwrap_or_default(),
        create_by: row.get::<_, String>(6).unwrap_or_default(),
        update_time: row.get::<_, String>(7).unwrap_or_default(),
        update_by: row.get::<_, String>(8).unwrap_or_default(),
        deleted: false,
        current_user_role: String::new(),
        server_update_time: row.get::<_, String>(9).unwrap_or_default(),
        // 兼容未列出 sync_version 的旧查询（列数不足时回退为默认值 1）
        sync_version: if row.as_ref().column_count() > 10 { row.get::<_, i32>(10).unwrap_or(1) } else { 1 },
    })
}


pub fn save_environment_group(conn: &DbConn, group: &EnvironmentGroup, mode: DataMode) -> Result<(), DbError> {
    let db = lock_db(conn)?;
    // expanded 是纯本地 UI 状态（折叠/展开），不随服务端缓存写入而被覆盖：
    // 已有行保留本地值，新行才用传入值（默认 true）。只有 set_environment_group_expanded 会改变它。
    let existing_expanded: i32 = db
        .query_row(
            "SELECT COALESCE((SELECT expanded FROM ch_environment_groups WHERE id = ?1 AND data_mode = ?2), -1)",
            params![group.id, mode.as_db_value()],
            |row| row.get(0),
        )
        .unwrap_or(-1);
    let expanded_to_write = if existing_expanded >= 0 {
        existing_expanded
    } else if group.expanded {
        1
    } else {
        0
    };
    db.execute(
        "INSERT OR REPLACE INTO ch_environment_groups (id, project_id, name, sort_order, expanded, create_time, create_by, update_time, update_by, data_mode, server_update_time, sync_version, dirty) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 1)",
        params![group.id, group.project_id, group.name, group.sort_order, expanded_to_write, group.create_time, group.create_by, group.update_time, group.update_by, mode.as_db_value(), group.server_update_time, group.sync_version],
    )
    .map_err(DbError::Sql)?;
    Ok(())
}


pub fn update_environment_group_db(conn: &DbConn, group: &EnvironmentGroup, mode: DataMode) -> Result<(), DbError> {
    let db = lock_db(conn)?;
    db.execute(
        "UPDATE ch_environment_groups SET name=?2, sort_order=?3, expanded=?4, update_time=?5, server_update_time=?7, dirty=1 WHERE id=?1 AND data_mode=?6",
        params![group.id, group.name, group.sort_order, group.expanded, group.update_time, mode.as_db_value(), group.server_update_time],
    )
    .map_err(DbError::Sql)?;
    Ok(())
}

/// 纯本地更新分组展开状态：只改 expanded，不置 dirty（不触发下一次同步上传）。
pub fn update_environment_group_expanded(conn: &DbConn, id: &str, expanded: bool, mode: DataMode) -> Result<(), DbError> {
    let db = lock_db(conn)?;
    db.execute(
        "UPDATE ch_environment_groups SET expanded = ?1 WHERE id = ?2 AND data_mode = ?3",
        params![expanded, id, mode.as_db_value()],
    )
    .map_err(DbError::Sql)?;
    Ok(())
}


pub fn delete_environment_group_db(conn: &DbConn, id: &str, mode: DataMode) -> Result<(), DbError> {
    let db = lock_db(conn)?;
    db.execute("PRAGMA foreign_keys=ON;", [])
        .map_err(DbError::Sql)?;
    db.execute("DELETE FROM ch_environment_groups WHERE id = ?1 AND data_mode = ?2", params![id, mode.as_db_value()])
        .map_err(DbError::Sql)?;
    Ok(())
}

// ====== 环境 CRUD ======


pub fn get_all_environments(conn: &DbConn, project_id: Option<&str>, mode: DataMode, only_dirty: bool) -> Result<Vec<Environment>, DbError> {
    let db = lock_db(conn)?;
    let has_pid = project_id.map(|p| !p.is_empty()).unwrap_or(false);
    let dirty_flag: i64 = if only_dirty { 1 } else { 0 };
    let mut stmt = if has_pid {
        db.prepare_cached("SELECT id, project_id, name, group_id, is_active, create_time, create_by, update_time, update_by, server_update_time, sync_version FROM ch_environments WHERE project_id = ?1 AND data_mode = ?2 AND deleted = 0 AND (?3 = 0 OR dirty = 1) ORDER BY create_time")?
    } else {
        db.prepare_cached("SELECT id, project_id, name, group_id, is_active, create_time, create_by, update_time, update_by, server_update_time, sync_version FROM ch_environments WHERE (project_id IS NULL OR project_id = '') AND data_mode = ?1 AND deleted = 0 AND (?2 = 0 OR dirty = 1) ORDER BY create_time")?
    };
    let rows = if has_pid {
        stmt.query_map(params![project_id.unwrap(), mode.as_db_value(), dirty_flag], map_env)
    } else {
        stmt.query_map(params![mode.as_db_value(), dirty_flag], map_env)
    }
    .map_err(DbError::Sql)?;
    let mut items = Vec::new();
    for r in rows {
        items.push(r.map_err(DbError::Sql)?);
    }
    Ok(items)
}


/// 同步专用：一次性取出**全部**环境（含无项目归属行与各项目归属行），消除按项目 N+1 查询。
pub fn get_all_environments_all(conn: &DbConn, mode: DataMode, only_dirty: bool) -> Result<Vec<Environment>, DbError> {
    let db = lock_db(conn)?;
    let dirty_flag: i64 = if only_dirty { 1 } else { 0 };
    let mut stmt = db.prepare_cached("SELECT id, project_id, name, group_id, is_active, create_time, create_by, update_time, update_by, server_update_time, sync_version FROM ch_environments WHERE data_mode = ?1 AND deleted = 0 AND (?2 = 0 OR dirty = 1) ORDER BY create_time")?;
    let rows = stmt
        .query_map(params![mode.as_db_value(), dirty_flag], map_env)
        .map_err(DbError::Sql)?;
    let mut items = Vec::new();
    for r in rows {
        items.push(r.map_err(DbError::Sql)?);
    }
    Ok(items)
}


fn map_env(row: &rusqlite::Row<'_>) -> Result<Environment, rusqlite::Error> {
    Ok(Environment {
        id: row.get(0)?,
        project_id: row.get(1)?,
        name: row.get(2)?,
        group_id: row.get(3)?,
        is_active: row.get::<_, bool>(4)?,
        create_time: row.get(5)?,
        create_by: row.get::<_, String>(6).unwrap_or_default(),
        update_time: row.get(7)?,
        update_by: row.get::<_, String>(8).unwrap_or_default(),
        deleted: false,
        current_user_role: String::new(),
        server_update_time: row.get::<_, String>(9).unwrap_or_default(),
        // 兼容未列出 sync_version 的旧查询（列数不足时回退为默认值 1）
        sync_version: if row.as_ref().column_count() > 10 { row.get::<_, i32>(10).unwrap_or(1) } else { 1 },
    })
}


fn map_active_var(row: &rusqlite::Row<'_>) -> Result<EnvironmentVariable, rusqlite::Error> {
    Ok(EnvironmentVariable {
        id: row.get(0)?,
        environment_id: row.get(1)?,
        key: row.get(2)?,
        value: row.get(3)?,
        enabled: row.get::<_, bool>(4)?,
        sort_order: row.get(5)?,
        create_time: row.get::<_, String>(6).unwrap_or_default(),
        create_by: row.get::<_, String>(7).unwrap_or_default(),
        update_time: row.get::<_, String>(8).unwrap_or_default(),
        update_by: row.get::<_, String>(9).unwrap_or_default(),
        deleted: false,
        current_user_role: String::new(),
        server_update_time: row.get::<_, String>(10).unwrap_or_default(),
        // 兼容未列出 sync_version 的查询（列数不足时回退为默认值 1）
        sync_version: if row.as_ref().column_count() > 11 { row.get::<_, i32>(11).unwrap_or(1) } else { 1 },
    })
}


fn map_env_var(row: &rusqlite::Row<'_>) -> Result<EnvironmentVariable, rusqlite::Error> {
    Ok(EnvironmentVariable {
        id: row.get(0)?,
        environment_id: row.get(1)?,
        key: row.get(2)?,
        value: row.get(3)?,
        enabled: row.get::<_, bool>(4)?,
        sort_order: row.get(5)?,
        create_time: row.get::<_, String>(6).unwrap_or_default(),
        create_by: row.get::<_, String>(7).unwrap_or_default(),
        update_time: row.get::<_, String>(8).unwrap_or_default(),
        update_by: row.get::<_, String>(9).unwrap_or_default(),
        deleted: false,
        current_user_role: String::new(),
        server_update_time: row.get::<_, String>(10).unwrap_or_default(),
        sync_version: row.get::<_, i32>(11).unwrap_or(1),
    })
}


pub fn get_env_variables(conn: &DbConn, env_id: &str, mode: DataMode, only_dirty: bool) -> Result<Vec<EnvironmentVariable>, DbError> {
    let db = lock_db(conn)?;
    let dirty_flag: i64 = if only_dirty { 1 } else { 0 };
    let mut stmt = db
        .prepare_cached("SELECT id, environment_id, key, value, enabled, sort_order, create_time, create_by, update_time, update_by, server_update_time, sync_version FROM ch_environment_variables WHERE environment_id = ?1 AND data_mode = ?2 AND deleted = 0 AND (?3 = 0 OR dirty = 1) ORDER BY sort_order")?;
    let rows = stmt
        .query_map(params![env_id, mode.as_db_value(), dirty_flag], map_env_var)
        .map_err(DbError::Sql)?;
    let mut items = Vec::new();
    for r in rows {
        items.push(r.map_err(DbError::Sql)?);
    }
    Ok(items)
}


/// 同步专用：一次性取出**全部**环境变量，消除按环境逐个查询的 N+1。
pub fn get_all_env_variables_all(conn: &DbConn, mode: DataMode, only_dirty: bool) -> Result<Vec<EnvironmentVariable>, DbError> {
    let db = lock_db(conn)?;
    let dirty_flag: i64 = if only_dirty { 1 } else { 0 };
    let mut stmt = db
        .prepare_cached("SELECT id, environment_id, key, value, enabled, sort_order, create_time, create_by, update_time, update_by, server_update_time, sync_version FROM ch_environment_variables WHERE data_mode = ?1 AND deleted = 0 AND (?2 = 0 OR dirty = 1) ORDER BY sort_order")?;
    let rows = stmt
        .query_map(params![mode.as_db_value(), dirty_flag], map_env_var)
        .map_err(DbError::Sql)?;
    let mut items = Vec::new();
    for r in rows {
        items.push(r.map_err(DbError::Sql)?);
    }
    Ok(items)
}


pub fn get_active_env_variables(conn: &DbConn, project_id: Option<&str>, mode: DataMode) -> Result<Vec<EnvironmentVariable>, DbError> {
    let db = lock_db(conn)?;
    let has_pid = project_id.map(|p| !p.is_empty()).unwrap_or(false);
    let mut stmt = if has_pid {
        db.prepare_cached(
            "SELECT v.id, v.environment_id, v.key, v.value, v.enabled, v.sort_order, v.create_time, v.create_by, v.update_time, v.update_by, v.server_update_time
             FROM ch_environment_variables v
             JOIN ch_environments e ON v.environment_id = e.id
             WHERE e.is_active = 1 AND e.project_id = ?1 AND e.data_mode = ?2
               AND e.deleted = 0 AND v.deleted = 0
             ORDER BY v.sort_order"
        )?
    } else {
        db.prepare_cached(
            "SELECT v.id, v.environment_id, v.key, v.value, v.enabled, v.sort_order, v.create_time, v.create_by, v.update_time, v.update_by, v.server_update_time
             FROM ch_environment_variables v
             JOIN ch_environments e ON v.environment_id = e.id
             WHERE e.is_active = 1 AND (e.project_id IS NULL OR e.project_id = '') AND e.data_mode = ?1
               AND e.deleted = 0 AND v.deleted = 0
             ORDER BY v.sort_order"
        )?
    };
    let rows = if has_pid {
        stmt.query_map(params![project_id.unwrap(), mode.as_db_value()], map_active_var)
    } else {
        stmt.query_map(params![mode.as_db_value()], map_active_var)
    }
    .map_err(DbError::Sql)?;
    let mut items = Vec::new();
    for r in rows {
        items.push(r.map_err(DbError::Sql)?);
    }
    Ok(items)
}


pub fn save_environment(conn: &DbConn, env: &Environment, mode: DataMode) -> Result<(), DbError> {
    let db = lock_db(conn)?;
    db.execute(
        "INSERT INTO ch_environments (id, project_id, name, group_id, is_active, create_time, create_by, update_time, update_by, data_mode, server_update_time, sync_version, dirty) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 1)",
        params![env.id, env.project_id, env.name, env.group_id, env.is_active, env.create_time, env.create_by, env.update_time, env.update_by, mode.as_db_value(), env.server_update_time, env.sync_version],
    )
    .map_err(DbError::Sql)?;
    Ok(())
}


pub fn update_environment(conn: &DbConn, env: &Environment, mode: DataMode) -> Result<(), DbError> {
    let db = lock_db(conn)?;
    db.execute(
        "UPDATE ch_environments SET project_id=?2, name=?3, group_id=?4, is_active=?5, update_time=?6, server_update_time=?8, dirty=1 WHERE id=?1 AND data_mode=?7",
        params![env.id, env.project_id, env.name, env.group_id, env.is_active, env.update_time, mode.as_db_value(), env.server_update_time],
    )
    .map_err(DbError::Sql)?;
    Ok(())
}


pub fn delete_environment_db(conn: &DbConn, id: &str, mode: DataMode) -> Result<(), DbError> {
    let db = lock_db(conn)?;
    db.execute("PRAGMA foreign_keys=ON;", [])
        .map_err(DbError::Sql)?;
    db.execute("DELETE FROM ch_environments WHERE id = ?1 AND data_mode = ?2", params![id, mode.as_db_value()])
        .map_err(DbError::Sql)?;
    Ok(())
}


pub fn set_active_environment(conn: &DbConn, id: &str, project_id: Option<&str>, mode: DataMode) -> Result<(), DbError> {
    let db = lock_db(conn)?;
    // 仅取消同项目下的激活，避免跨项目互相影响；未归项目环境单独处理
    if let Some(pid) = project_id {
        if !pid.is_empty() {
            db.execute("UPDATE ch_environments SET is_active = 0 WHERE project_id = ?1 AND data_mode = ?2", params![pid, mode.as_db_value()])
                .map_err(DbError::Sql)?;
        } else {
            db.execute("UPDATE ch_environments SET is_active = 0 WHERE (project_id IS NULL OR project_id = '') AND data_mode = ?1", params![mode.as_db_value()])
                .map_err(DbError::Sql)?;
        }
    } else {
        db.execute("UPDATE ch_environments SET is_active = 0 WHERE (project_id IS NULL OR project_id = '') AND data_mode = ?1", params![mode.as_db_value()])
            .map_err(DbError::Sql)?;
    }
    db.execute("UPDATE ch_environments SET is_active = 1 WHERE id = ?1 AND data_mode = ?2", params![id, mode.as_db_value()])
        .map_err(DbError::Sql)?;
    Ok(())
}


pub fn save_env_variable(conn: &DbConn, var: &EnvironmentVariable, mode: DataMode) -> Result<(), DbError> {
    let db = lock_db(conn)?;
    db.execute(
        "INSERT INTO ch_environment_variables (id, environment_id, key, value, enabled, sort_order, create_time, create_by, update_time, update_by, data_mode, server_update_time, sync_version, dirty) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,1)",
        params![var.id, var.environment_id, var.key, var.value, var.enabled, var.sort_order, var.create_time, var.create_by, var.update_time, var.update_by, mode.as_db_value(), var.server_update_time, var.sync_version],
    )
    .map_err(DbError::Sql)?;
    Ok(())
}


pub fn update_env_variable(conn: &DbConn, var: &EnvironmentVariable, mode: DataMode) -> Result<(), DbError> {
    let db = lock_db(conn)?;
    db.execute(
        "UPDATE ch_environment_variables SET key=?2, value=?3, enabled=?4, sort_order=?5, update_time=?6, server_update_time=?8, dirty=1 WHERE id=?1 AND data_mode=?7",
        params![var.id, var.key, var.value, var.enabled, var.sort_order, var.update_time, mode.as_db_value(), var.server_update_time],
    )
    .map_err(DbError::Sql)?;
    Ok(())
}


pub fn delete_env_variable_db(conn: &DbConn, id: &str, mode: DataMode) -> Result<(), DbError> {
    let db = lock_db(conn)?;
    db.execute("DELETE FROM ch_environment_variables WHERE id = ?1 AND data_mode = ?2", params![id, mode.as_db_value()])
        .map_err(DbError::Sql)?;
    Ok(())
}

// ====== 同步合并辅助函数（接受 &Connection，调用方已持锁，仅用于 online 模式） ======
