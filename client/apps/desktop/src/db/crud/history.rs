//! 请求历史的本地读写实现（由原 `db.rs` 拆分，Phase 8.1）。

use crate::db::*;
use rusqlite::params;

pub fn get_all_history(conn: &DbConn, limit: i32, mode: DataMode) -> Result<Vec<HistoryItem>, DbError> {
    let db = lock_db(conn)?;
    let mut stmt = db
        .prepare_cached(
            "SELECT id, method, url, params, headers, body_type, body, form_body, category_id, status, create_time, pre_script, post_script, project_id
             FROM ch_history WHERE data_mode = ?2 ORDER BY create_time DESC LIMIT ?1",
        )?;
    let rows = stmt
        .query_map(params![limit, mode.as_db_value()], |row| {
            let params_str: String = row.get(3)?;
            let headers_str: String = row.get(4)?;
            let form_body_str: String = row.get(7)?;
            Ok(HistoryItem {
                id: row.get(0)?,
                method: row.get(1)?,
                url: row.get(2)?,
                params: serde_json::from_str(&params_str).unwrap_or(serde_json::Value::Array(vec![])),
                headers: serde_json::from_str(&headers_str).unwrap_or(serde_json::Value::Array(vec![])),
                body_type: row.get(5)?,
                body: row.get(6)?,
                form_body: serde_json::from_str(&form_body_str).unwrap_or(serde_json::Value::Array(vec![])),
                category_id: row.get(8)?,
                status: row.get(9)?,
                create_time: row.get(10)?,
                pre_script: row.get::<_, String>(11).unwrap_or_default(),
                post_script: row.get::<_, String>(12).unwrap_or_default(),
                project_id: row.get(13).unwrap_or_default(),
            })
        })
        .map_err(DbError::Sql)?;
    let mut items = Vec::new();
    for r in rows {
        items.push(r.map_err(DbError::Sql)?);
    }
    Ok(items)
}


pub fn save_history_item(conn: &DbConn, item: &HistoryItem, mode: DataMode) -> Result<(), DbError> {
    let db = lock_db(conn)?;
    let params_str = item.params.to_string();
    let headers_str = item.headers.to_string();
    let form_body_str = item.form_body.to_string();
    db.execute(
        "INSERT INTO ch_history (id, method, url, params, headers, body_type, body, form_body, category_id, status, create_time, pre_script, post_script, project_id, data_mode)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
        params![
            item.id, item.method, item.url,
            params_str, headers_str, item.body_type,
            item.body, form_body_str, item.category_id,
            item.status, item.create_time,
            item.pre_script, item.post_script, item.project_id,
            mode.as_db_value(),
        ],
    )
    .map_err(DbError::Sql)?;
    // Keep only the latest MAX_HISTORY items of the same mode
    db.execute(
        "DELETE FROM ch_history WHERE data_mode = ?1 AND id NOT IN (SELECT id FROM ch_history WHERE data_mode = ?1 ORDER BY create_time DESC LIMIT ?2)",
        params![mode.as_db_value(), MAX_HISTORY as i32],
    )
    .map_err(DbError::Sql)?;
    Ok(())
}


pub fn delete_history_item_db(conn: &DbConn, id: &str, mode: DataMode) -> Result<(), DbError> {
    let db = lock_db(conn)?;
    db.execute("DELETE FROM ch_history WHERE id = ?1 AND data_mode = ?2", params![id, mode.as_db_value()])
        .map_err(DbError::Sql)?;
    Ok(())
}


pub fn clear_all_history(conn: &DbConn, mode: DataMode) -> Result<(), DbError> {
    let db = lock_db(conn)?;
    db.execute("DELETE FROM ch_history WHERE data_mode = ?1", params![mode.as_db_value()])
        .map_err(DbError::Sql)?;
    Ok(())
}

// ====== 保存的请求 CRUD ======
