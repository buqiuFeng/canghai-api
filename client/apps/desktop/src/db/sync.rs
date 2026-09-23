//! 增量同步支撑：`*_for_merge` 合并写入、脏标记、墓碑删除与事务封装。
//!
//! 客户端侧冲突判定与游标推进在 `sync.rs`（crate 根）中完成，本模块只提供数据动作。

use crate::db::*;
use rusqlite::{Connection, params};

/// Phase 4 增量同步涉及的表：均有 version / server_update_time / dirty 三列。
pub(crate) const SYNC_TABLES: [&str; 5] = [
    "ch_categories",
    "ch_saved_requests",
    "ch_environments",
    "ch_environment_groups",
    "ch_environment_variables",
];


pub fn save_category_for_merge(db: &Connection, cat: &Category, mode: DataMode) -> Result<(), DbError> {
    db.execute(
        "INSERT OR REPLACE INTO ch_categories (id, project_id, name, parent_id, sort_order, expanded,
         create_time, update_time, create_by, update_by, server_update_time, sync_version, dirty, data_mode)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,0,?13)",
        params![
            cat.id, cat.project_id, cat.name, cat.parent_id,
            cat.sort_order, cat.expanded, cat.create_time, cat.update_time,
            cat.create_by, cat.update_by, cat.server_update_time, cat.sync_version, mode.as_db_value(),
        ],
    )
    .map_err(DbError::Sql)?;
    Ok(())
}


pub fn update_category_for_merge(db: &Connection, cat: &Category, mode: DataMode) -> Result<(), DbError> {
    db.execute(
        "UPDATE ch_categories SET project_id=?2, name=?3, parent_id=?4, sort_order=?5,
         expanded=?6, create_time=?7, update_time=?8, create_by=?9, update_by=?10, server_update_time=?11,
         sync_version=?12, dirty=0 WHERE id=?1 AND data_mode=?13",
        params![
            cat.id, cat.project_id, cat.name, cat.parent_id,
            cat.sort_order, cat.expanded, cat.create_time, cat.update_time,
            cat.create_by, cat.update_by, cat.server_update_time, cat.sync_version, mode.as_db_value(),
        ],
    )
    .map_err(DbError::Sql)?;
    Ok(())
}

// ====== 历史 CRUD ======


pub fn save_project_for_merge(db: &Connection, cat: &Project, mode: DataMode) -> Result<(), DbError> {
    db.execute(
        "INSERT OR REPLACE INTO ch_projects (id, user_id, name, parent_id, sort_order, expanded, create_time, create_by, update_time, update_by, deleted, data_mode) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
        params![cat.id, cat.user_id, cat.name, cat.parent_id, cat.sort_order, cat.expanded, cat.create_time, cat.create_by, cat.update_time, cat.update_by, cat.deleted as i32, mode.as_db_value()],
    ).map_err(DbError::Sql)?;
    Ok(())
}


pub fn update_project_for_merge(db: &Connection, cat: &Project, mode: DataMode) -> Result<(), DbError> {
    db.execute(
        "UPDATE ch_projects SET name=?2, parent_id=?3, sort_order=?4, expanded=?5, update_time=?6, update_by=?7, deleted=?8 WHERE id=?1 AND data_mode=?9",
        params![cat.id, cat.name, cat.parent_id, cat.sort_order, cat.expanded, cat.update_time, cat.update_by, cat.deleted as i32, mode.as_db_value()],
    ).map_err(DbError::Sql)?;
    Ok(())
}


pub fn save_team_for_merge(db: &Connection, t: &Team, mode: DataMode) -> Result<(), DbError> {
    db.execute(
        "INSERT OR REPLACE INTO ch_teams (id, name, description, owner_id, user_id, create_time, create_by, update_time, update_by, deleted, data_mode) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
        params![t.id, t.name, t.description, t.owner_id, t.user_id, t.create_time, t.create_by, t.update_time, t.update_by, t.deleted as i32, mode.as_db_value()],
    ).map_err(DbError::Sql)?;
    Ok(())
}


pub fn update_team_for_merge(db: &Connection, t: &Team, mode: DataMode) -> Result<(), DbError> {
    db.execute(
        "UPDATE ch_teams SET name=?2, description=?3, owner_id=?4, user_id=?5, update_time=?6, update_by=?7, deleted=?8 WHERE id=?1 AND data_mode=?9",
        params![t.id, t.name, t.description, t.owner_id, t.user_id, t.update_time, t.update_by, t.deleted as i32, mode.as_db_value()],
    ).map_err(DbError::Sql)?;
    Ok(())
}


pub fn save_team_member_for_merge(db: &Connection, m: &TeamMember, mode: DataMode) -> Result<(), DbError> {
    db.execute(
        "INSERT OR REPLACE INTO ch_team_members (id, team_id, user_id, role, create_time, create_by, update_time, update_by, deleted, data_mode) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,0,?9)",
        params![m.id, m.team_id, m.user_id, m.role, m.create_time, m.create_by, m.update_time, m.update_by, mode.as_db_value()],
    ).map_err(DbError::Sql)?;
    Ok(())
}


pub fn update_team_member_for_merge(db: &Connection, m: &TeamMember, mode: DataMode) -> Result<(), DbError> {
    db.execute(
        "UPDATE ch_team_members SET team_id=?2, user_id=?3, role=?4, update_time=?5, update_by=?6 WHERE id=?1 AND data_mode=?7",
        params![m.id, m.team_id, m.user_id, m.role, m.update_time, m.update_by, mode.as_db_value()],
    ).map_err(DbError::Sql)?;
    Ok(())
}


pub fn save_project_member_for_merge(db: &Connection, m: &ProjectMember, mode: DataMode) -> Result<(), DbError> {
    db.execute(
        "INSERT OR REPLACE INTO ch_project_members (id, project_id, member_type, member_id, member_name, role, create_time, create_by, update_time, update_by, deleted, data_mode) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
        params![m.id, m.project_id, m.member_type, m.member_id, m.member_name, m.role, m.create_time, m.create_by, m.update_time, m.update_by, m.deleted as i32, mode.as_db_value()],
    ).map_err(DbError::Sql)?;
    Ok(())
}


pub fn update_project_member_for_merge(db: &Connection, m: &ProjectMember, mode: DataMode) -> Result<(), DbError> {
    db.execute(
        "UPDATE ch_project_members SET project_id=?2, member_type=?3, member_id=?4, member_name=?5, role=?6, update_time=?7, update_by=?8, deleted=?9 WHERE id=?1 AND data_mode=?10",
        params![m.id, m.project_id, m.member_type, m.member_id, m.member_name, m.role, m.update_time, m.update_by, m.deleted as i32, mode.as_db_value()],
    ).map_err(DbError::Sql)?;
    Ok(())
}


pub fn save_request_for_merge(db: &Connection, req: &SavedRequest, mode: DataMode) -> Result<(), DbError> {
    let params_str = req.params.to_string();
    let headers_str = req.headers.to_string();
    let form_body_str = req.form_body.to_string();
    db.execute(
        "INSERT OR REPLACE INTO ch_saved_requests (id, user_id, name, method, url, params, headers, body_type, body, form_body, category_id, project_id, pre_script, post_script, sort_order, create_time, create_by, update_time, update_by, server_update_time, sync_version, dirty, data_mode) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,0,?22)",
        params![req.id, req.user_id, req.name, req.method, req.url, params_str, headers_str, req.body_type, req.body, form_body_str, req.category_id, req.project_id, req.pre_script, req.post_script, req.sort_order, req.create_time, req.create_by, req.update_time, req.update_by, req.server_update_time, req.sync_version, mode.as_db_value()],
    ).map_err(DbError::Sql)?;
    Ok(())
}


pub fn update_request_for_merge(db: &Connection, req: &SavedRequest, mode: DataMode) -> Result<(), DbError> {
    let params_str = req.params.to_string();
    let headers_str = req.headers.to_string();
    let form_body_str = req.form_body.to_string();
    db.execute(
        "UPDATE ch_saved_requests SET user_id=?2, name=?3, method=?4, url=?5, params=?6, headers=?7, body_type=?8, body=?9, form_body=?10, category_id=?11, project_id=?12, pre_script=?13, post_script=?14, sort_order=?15, create_time=?16, create_by=?17, update_time=?18, update_by=?19, server_update_time=?20, sync_version=?21, dirty=0 WHERE id=?1 AND data_mode=?22",
        params![req.id, req.user_id, req.name, req.method, req.url, params_str, headers_str, req.body_type, req.body, form_body_str, req.category_id, req.project_id, req.pre_script, req.post_script, req.sort_order, req.create_time, req.create_by, req.update_time, req.update_by, req.server_update_time, req.sync_version, mode.as_db_value()],
    ).map_err(DbError::Sql)?;
    Ok(())
}


pub fn save_env_group_for_merge(db: &Connection, group: &EnvironmentGroup, mode: DataMode) -> Result<(), DbError> {
    db.execute(
        "INSERT OR REPLACE INTO ch_environment_groups (id, project_id, name, sort_order, expanded, create_time, create_by, update_time, update_by, server_update_time, sync_version, dirty, data_mode) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,0,?12)",
        params![group.id, group.project_id, group.name, group.sort_order, group.expanded, group.create_time, group.create_by, group.update_time, group.update_by, group.server_update_time, group.sync_version, mode.as_db_value()],
    ).map_err(DbError::Sql)?;
    Ok(())
}


pub fn update_env_group_for_merge(db: &Connection, group: &EnvironmentGroup, mode: DataMode) -> Result<(), DbError> {
    db.execute(
        "UPDATE ch_environment_groups SET project_id=?2, name=?3, sort_order=?4, expanded=?5, update_time=?6, update_by=?7, server_update_time=?8, sync_version=?9, dirty=0 WHERE id=?1 AND data_mode=?10",
        params![group.id, group.project_id, group.name, group.sort_order, group.expanded, group.update_time, group.update_by, group.server_update_time, group.sync_version, mode.as_db_value()],
    ).map_err(DbError::Sql)?;
    Ok(())
}


pub fn save_env_for_merge(db: &Connection, env: &Environment, mode: DataMode) -> Result<(), DbError> {
    db.execute(
        "INSERT OR REPLACE INTO ch_environments (id, project_id, name, group_id, is_active, create_time, create_by, update_time, update_by, server_update_time, sync_version, dirty, data_mode) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,0,?12)",
        params![env.id, env.project_id, env.name, env.group_id, env.is_active, env.create_time, env.create_by, env.update_time, env.update_by, env.server_update_time, env.sync_version, mode.as_db_value()],
    ).map_err(DbError::Sql)?;
    Ok(())
}


pub fn update_env_for_merge(db: &Connection, env: &Environment, mode: DataMode) -> Result<(), DbError> {
    db.execute(
        "UPDATE ch_environments SET project_id=?2, name=?3, group_id=?4, is_active=?5, update_time=?6, server_update_time=?7, sync_version=?8, dirty=0 WHERE id=?1 AND data_mode=?9",
        params![env.id, env.project_id, env.name, env.group_id, env.is_active, env.update_time, env.server_update_time, env.sync_version, mode.as_db_value()],
    ).map_err(DbError::Sql)?;
    Ok(())
}


pub fn save_var_for_merge(db: &Connection, var: &EnvironmentVariable, mode: DataMode) -> Result<(), DbError> {
    db.execute(
        "INSERT OR REPLACE INTO ch_environment_variables (id, environment_id, key, value, enabled, sort_order, create_time, create_by, update_time, update_by, server_update_time, sync_version, dirty, data_mode) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,0,?13)",
        params![var.id, var.environment_id, var.key, var.value, var.enabled, var.sort_order, var.create_time, var.create_by, var.update_time, var.update_by, var.server_update_time, var.sync_version, mode.as_db_value()],
    ).map_err(DbError::Sql)?;
    Ok(())
}


pub fn update_var_for_merge(db: &Connection, var: &EnvironmentVariable, mode: DataMode) -> Result<(), DbError> {
    db.execute(
        "UPDATE ch_environment_variables SET key=?2, value=?3, enabled=?4, sort_order=?5, update_time=?6, server_update_time=?7, sync_version=?8, dirty=0 WHERE id=?1 AND data_mode=?9",
        params![var.id, var.key, var.value, var.enabled, var.sort_order, var.update_time, var.server_update_time, var.sync_version, mode.as_db_value()],
    ).map_err(DbError::Sql)?;
    Ok(())
}

// ====== Phase 4：增量同步辅助（dirty 标记 / 墓碑删除 / 事务合并）======


/// 上传成功后清除本地待上传标记（dirty = 0）。
/// 只处理指定数据模式下的行；之后的下一次上传即为空集，实现真正的增量。
pub fn clear_dirty_flags(conn: &DbConn, mode: DataMode) -> Result<(), DbError> {
    let db = lock_db(conn)?;
    let mode_val = mode.as_db_value();
    for t in SYNC_TABLES {
        db.execute(&format!("UPDATE {t} SET dirty = 0 WHERE data_mode = ?1"), params![mode_val])
            .map_err(DbError::Sql)?;
    }
    Ok(())
}


/// 墓碑批量执行的分片大小：SQLite 的变量数上限（`SQLITE_MAX_VARIABLE_NUMBER`）默认 999，
/// 这里按 200 分片，给 `data_mode` 参数留出余量，避免 id 过多时报 "too many SQL variables"。
const DELETED_ID_CHUNK: usize = 200;

/// 应用服务端墓碑：把服务端已删除的实体在本地清理掉，统一走**软删**。
///
/// - 5 张同步表：`deleted = 1`，并一并清掉 `dirty`，避免这些行再被当作本地变更回传；
/// - `ch_project_members` / `ch_projects`：同样 `deleted = 1`。
///   项目此前因本地无 `deleted` 列而走物理删除，现已补列（见 `schema.rs`），
///   与其它实体统一；软删后的行由读路径（`get_all_projects` / `get_projects_by_ids`）过滤。
///
/// 按表做批量 `IN (...)` 而不是「逐 id 逐表」探测：id 为全局唯一 UUID，
/// 批量 IN 未命中即影响 0 行，结果等价，但 SQL 次数从 O(id 数 × 表数) 降到
/// O(分片数 × 表数)。
pub fn apply_deleted_ids(conn: &DbConn, ids: &[String], mode: DataMode) -> Result<usize, DbError> {
    if ids.is_empty() {
        return Ok(0);
    }
    let db = lock_db(conn)?;
    let mode_val = mode.as_db_value();
    let mut affected = 0usize;

    for chunk in ids.chunks(DELETED_ID_CHUNK) {
        let placeholders: Vec<String> = (0..chunk.len()).map(|i| format!("?{}", i + 2)).collect();
        let in_clause = placeholders.join(",");
        // 第 1 个参数固定为 data_mode，其余为 id
        let mut bind: Vec<rusqlite::types::Value> = Vec::with_capacity(chunk.len() + 1);
        bind.push(rusqlite::types::Value::Text(mode_val.to_string()));
        for id in chunk {
            bind.push(rusqlite::types::Value::Text(id.clone()));
        }

        for t in SYNC_TABLES {
            affected += db
                .execute(
                    &format!("UPDATE {t} SET deleted = 1, dirty = 0 WHERE data_mode = ?1 AND id IN ({in_clause})"),
                    rusqlite::params_from_iter(bind.iter()),
                )
                .map_err(DbError::Sql)?;
        }
        // 项目成员：有 deleted 列 → 软删
        affected += db
            .execute(
                &format!("UPDATE ch_project_members SET deleted = 1 WHERE data_mode = ?1 AND id IN ({in_clause})"),
                rusqlite::params_from_iter(bind.iter()),
            )
            .map_err(DbError::Sql)?;
        // 项目：与其它实体一致走软删，由读路径过滤
        affected += db
            .execute(
                &format!("UPDATE ch_projects SET deleted = 1 WHERE data_mode = ?1 AND id IN ({in_clause})"),
                rusqlite::params_from_iter(bind.iter()),
            )
            .map_err(DbError::Sql)?;
    }
    Ok(affected)
}


/// 在单个事务内执行闭包（用于批量合并，避免逐行自动提交）。
/// 采用 SAVEPOINT 实现，可安全嵌套；失败自动回滚。
pub fn in_transaction<F, T>(conn: &DbConn, f: F) -> Result<T, DbError>
where
    F: FnOnce(&Connection) -> Result<T, DbError>,
{
    let db = lock_db(conn)?;
    db.execute_batch("SAVEPOINT canghai_merge;").map_err(DbError::Sql)?;
    match f(&db) {
        Ok(v) => {
            db.execute_batch("RELEASE SAVEPOINT canghai_merge;").map_err(DbError::Sql)?;
            Ok(v)
        }
        Err(e) => {
            let _ = db.execute_batch("ROLLBACK TO SAVEPOINT canghai_merge; RELEASE SAVEPOINT canghai_merge;");
            Err(e)
        }
    }
}
