//! 分类的本地读写实现（由原 `db.rs` 拆分，Phase 8.1）。

use crate::db::*;
use rusqlite::params;

pub fn get_categories(
    conn: &DbConn,
    project_id: &str,
    mode: DataMode,
) -> Result<Vec<Category>, DbError> {
    let db = lock_db(conn)?;
    let mut stmt = db.prepare_cached(
        "SELECT id, project_id, name, parent_id, sort_order, expanded,
                create_time, update_time, create_by, update_by, deleted, server_update_time, sync_version
         FROM ch_categories
         WHERE project_id = ?1 AND data_mode = ?2 AND deleted = 0
         ORDER BY sort_order, name",
    )?;
    let rows = stmt
        .query_map(params![project_id, mode.as_db_value()], |row| {
            Ok(Category {
                id: row.get(0)?,
                project_id: row.get(1)?,
                name: row.get(2)?,
                parent_id: row.get(3)?,
                sort_order: row.get(4)?,
                expanded: row.get::<_, i32>(5).unwrap_or(1) != 0,
                create_time: row.get(6)?,
                update_time: row.get(7)?,
                create_by: row.get::<_, String>(8).unwrap_or_default(),
                update_by: row.get::<_, String>(9).unwrap_or_default(),
                deleted: row.get::<_, i32>(10).unwrap_or(0) != 0,
                server_update_time: row.get::<_, String>(11).unwrap_or_default(),
                sync_version: row.get::<_, i32>(12).unwrap_or(1),
            })
        })
        .map_err(DbError::Sql)?;
    let mut items = Vec::new();
    for r in rows {
        items.push(r.map_err(DbError::Sql)?);
    }
    Ok(items)
}


pub fn save_category(conn: &DbConn, cat: &Category, mode: DataMode) -> Result<(), DbError> {
    let db = lock_db(conn)?;
    // expanded 是纯本地 UI 状态（折叠/展开），不随服务端缓存写入而被覆盖：
    // 已有行保留本地值，新行才用传入值（默认 true）。只有 set_category_expanded 会改变它。
    let existing_expanded: i32 = db
        .query_row(
            "SELECT COALESCE((SELECT expanded FROM ch_categories WHERE id = ?1 AND data_mode = ?2), -1)",
            params![cat.id, mode.as_db_value()],
            |row| row.get(0),
        )
        .unwrap_or(-1);
    let expanded_to_write = if existing_expanded >= 0 {
        existing_expanded
    } else if cat.expanded {
        1
    } else {
        0
    };
    db.execute(
        "INSERT OR REPLACE INTO ch_categories (id, project_id, name, parent_id, sort_order, expanded,
         create_time, update_time, create_by, update_by, server_update_time, sync_version, dirty, data_mode)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,1,?13)",
        params![
            cat.id, cat.project_id, cat.name, cat.parent_id,
            cat.sort_order, expanded_to_write, cat.create_time, cat.update_time,
            cat.create_by, cat.update_by, cat.server_update_time, cat.sync_version, mode.as_db_value(),
        ],
    )
    .map_err(DbError::Sql)?;
    Ok(())
}


pub fn update_category_db(conn: &DbConn, id: &str, name: &str, mode: DataMode) -> Result<(), DbError> {
    let db = lock_db(conn)?;
    db.execute(
        "UPDATE ch_categories SET name = ?1, server_update_time = ?2, dirty = 1 WHERE id = ?3 AND data_mode = ?4",
        params![name, "", id, mode.as_db_value()],
    )
    .map_err(DbError::Sql)?;
    Ok(())
}


pub fn update_category_expanded(conn: &DbConn, id: &str, expanded: bool, mode: DataMode) -> Result<(), DbError> {
    let db = lock_db(conn)?;
    db.execute(
        "UPDATE ch_categories SET expanded = ?1 WHERE id = ?2 AND data_mode = ?3",
        params![expanded, id, mode.as_db_value()],
    )
    .map_err(DbError::Sql)?;
    Ok(())
}


pub fn delete_category_db(conn: &DbConn, id: &str, mode: DataMode) -> Result<(), DbError> {
    let db = lock_db(conn)?;
    db.execute("PRAGMA foreign_keys=ON;", [])
        .map_err(DbError::Sql)?;
    // 逻辑删除：分类及其子孙分类标记为 deleted，其下接口退为未分类
    db.execute(
        "UPDATE ch_categories SET deleted = 1, dirty = 1 WHERE (id = ?1 OR parent_id = ?1) AND data_mode = ?2",
        params![id, mode.as_db_value()],
    )
    .map_err(DbError::Sql)?;
    db.execute(
        "UPDATE ch_saved_requests SET category_id = NULL WHERE category_id = ?1 AND data_mode = ?2",
        params![id, mode.as_db_value()],
    )
    .map_err(DbError::Sql)?;
    Ok(())
}


/// 返回指定项目集合下所有分类，用于同步时收集（仅在线模式）。
/// `only_dirty = true` 时只返回 dirty=1 的待上传行（Phase 4 增量上传）。
pub fn get_all_categories(conn: &DbConn, project_ids: &[String], mode: DataMode, only_dirty: bool) -> Result<Vec<Category>, DbError> {
    if project_ids.is_empty() {
        return Ok(Vec::new());
    }
    let db = lock_db(conn)?;
    let placeholders: Vec<String> = (1..=project_ids.len()).map(|i| format!("?{i}")).collect();
    let sql = format!(
        "SELECT id, project_id, name, parent_id, sort_order, expanded,
                create_time, update_time, create_by, update_by, deleted, server_update_time, sync_version
         FROM ch_categories
         WHERE project_id IN ({}) AND data_mode = ?{} AND deleted = 0 AND (?{} = 0 OR dirty = 1)
         ORDER BY sort_order, name",
        placeholders.join(","),
        project_ids.len() + 1,
        project_ids.len() + 2,
    );
    let mut stmt = db.prepare_cached(&sql)?;
    let mut sql_params: Vec<rusqlite::types::Value> =
        project_ids.iter().map(|s| rusqlite::types::Value::Text(s.clone())).collect();
    sql_params.push(rusqlite::types::Value::Text(mode.as_db_value().to_string()));
    sql_params.push(rusqlite::types::Value::Integer(if only_dirty { 1 } else { 0 }));
    let rows = stmt
        .query_map(rusqlite::params_from_iter(sql_params), |row| {
            Ok(Category {
                id: row.get(0)?,
                project_id: row.get(1)?,
                name: row.get(2)?,
                parent_id: row.get(3)?,
                sort_order: row.get(4)?,
                expanded: row.get::<_, i32>(5).unwrap_or(1) != 0,
                create_time: row.get(6)?,
                update_time: row.get(7)?,
                create_by: row.get::<_, String>(8).unwrap_or_default(),
                update_by: row.get::<_, String>(9).unwrap_or_default(),
                deleted: row.get::<_, i32>(10).unwrap_or(0) != 0,
                server_update_time: row.get::<_, String>(11).unwrap_or_default(),
                sync_version: row.get::<_, i32>(12).unwrap_or(1),
            })
        })
        .map_err(DbError::Sql)?;
    let mut items = Vec::new();
    for r in rows {
        items.push(r.map_err(DbError::Sql)?);
    }
    Ok(items)
}
