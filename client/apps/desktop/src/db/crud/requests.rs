//! 已保存接口与历史版本的本地读写实现（由原 `db.rs` 拆分，Phase 8.1）。

use crate::db::*;
use rusqlite::{Connection, params, OptionalExtension};

pub fn get_all_saved_requests(conn: &DbConn, project_ids: &[String], mode: DataMode, only_dirty: bool) -> Result<Vec<SavedRequest>, DbError> {
    if project_ids.is_empty() {
        return Ok(Vec::new());
    }
    let db = lock_db(conn)?;
    let placeholders: Vec<String> = (1..=project_ids.len()).map(|i| format!("?{i}")).collect();
    let sql = format!(
        "SELECT id, user_id, name, method, url, params, headers, body_type, body, form_body,
                category_id, project_id, pre_script, post_script, sort_order, create_time, create_by, update_time, update_by, server_update_time, sync_version
         FROM ch_saved_requests
         WHERE project_id IN ({}) AND data_mode = ?{} AND deleted = 0 AND (?{} = 0 OR dirty = 1) ORDER BY sort_order, name",
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
            let params_str: String = row.get(5)?;
            let headers_str: String = row.get(6)?;
            let form_body_str: String = row.get(9)?;
            Ok(SavedRequest {
                id: row.get(0)?,
                user_id: row.get::<_, String>(1).unwrap_or_default(),
                name: row.get(2)?,
                method: row.get(3)?,
                url: row.get(4)?,
                params: serde_json::from_str(&params_str).unwrap_or(serde_json::Value::Array(vec![])),
                headers: serde_json::from_str(&headers_str).unwrap_or(serde_json::Value::Array(vec![])),
                body_type: row.get(7)?,
                body: row.get(8)?,
                form_body: serde_json::from_str(&form_body_str).unwrap_or(serde_json::Value::Array(vec![])),
                category_id: row.get(10)?,
                project_id: row.get(11)?,
                pre_script: row.get::<_, String>(12).unwrap_or_default(),
                post_script: row.get::<_, String>(13).unwrap_or_default(),
                sort_order: row.get(14)?,
                create_time: row.get(15)?,
                create_by: row.get::<_, String>(16).unwrap_or_default(),
                update_time: row.get(17)?,
                update_by: row.get::<_, String>(18).unwrap_or_default(),
                server_update_time: row.get::<_, String>(19).unwrap_or_default(),
                deleted: false,
                current_user_role: String::new(),
                sync_version: row.get::<_, i32>(20).unwrap_or(1),
            })
        })
        .map_err(DbError::Sql)?;
    let mut items = Vec::new();
    for r in rows {
        items.push(r.map_err(DbError::Sql)?);
    }
    Ok(items)
}


pub fn save_saved_request(conn: &DbConn, req: &SavedRequest, mode: DataMode) -> Result<(), DbError> {
    let db = lock_db(conn)?;
    let params_str = req.params.to_string();
    let headers_str = req.headers.to_string();
    let form_body_str = req.form_body.to_string();
    // 仅当内容相对当前版本发生变化时才生成新版本（见 resolve_version）
    let snapshot = build_snapshot(req);
    let (version, changed) = resolve_version(&db, req, mode, &snapshot)?;
    // 使用 upsert：已存在相同 id 时按更新字段覆盖（保留原始 create_time/create_by），
    // 避免更新接口缓存时因重复 INSERT 触发 UNIQUE 约束冲突。
    db.execute(
        "INSERT INTO ch_saved_requests (id, name, method, url, params, headers, body_type, body, form_body,
         category_id, project_id, pre_script, post_script, sort_order, create_time, create_by, update_time, update_by, server_update_time, version, sync_version, dirty, data_mode)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,1,?22)
         ON CONFLICT(id) DO UPDATE SET
           name=excluded.name, method=excluded.method, url=excluded.url,
           params=excluded.params, headers=excluded.headers, body_type=excluded.body_type, body=excluded.body,
           form_body=excluded.form_body, category_id=excluded.category_id, project_id=excluded.project_id,
           pre_script=excluded.pre_script, post_script=excluded.post_script, sort_order=excluded.sort_order,
           update_time=excluded.update_time, server_update_time=excluded.server_update_time, version=?20, dirty=1",
        params![
            req.id, req.name, req.method, req.url,
            params_str, headers_str, req.body_type,
            req.body, form_body_str, req.category_id, req.project_id,
            req.pre_script, req.post_script,
            req.sort_order, req.create_time, req.create_by, req.update_time, req.update_by,
            req.server_update_time,
            version,
            req.sync_version,
            mode.as_db_value(),
        ],
    )
    .map_err(DbError::Sql)?;
    // 内容未变化时复用当前版本，不产生新快照
    if changed {
        snapshot_request_version(&db, req, version as i32, mode)?;
    }
    Ok(())
}


pub fn update_saved_request(conn: &DbConn, req: &SavedRequest, mode: DataMode) -> Result<(), DbError> {
    let db = lock_db(conn)?;
    let params_str = req.params.to_string();
    let headers_str = req.headers.to_string();
    let form_body_str = req.form_body.to_string();
    // 仅当内容相对当前版本发生变化时才生成新版本（见 resolve_version）
    let snapshot = build_snapshot(req);
    let (version, changed) = resolve_version(&db, req, mode, &snapshot)?;
    db.execute(
        "UPDATE ch_saved_requests SET name=?2, method=?3, url=?4, params=?5, headers=?6,
         body_type=?7, body=?8, form_body=?9, category_id=?10, project_id=?11, pre_script=?12, post_script=?13,
         sort_order=?14, update_time=?15, server_update_time=?16, version=?17, dirty=1
         WHERE id=?1 AND data_mode=?18",
        params![
            req.id, req.name, req.method, req.url,
            params_str, headers_str, req.body_type,
            req.body, form_body_str, req.category_id, req.project_id,
            req.pre_script, req.post_script,
            req.sort_order, req.update_time, req.server_update_time, version,
            mode.as_db_value(),
        ],
    )
    .map_err(DbError::Sql)?;
    // 内容未变化时复用当前版本，不产生新快照
    if changed {
        snapshot_request_version(&db, req, version as i32, mode)?;
    }
    Ok(())
}


/// 快照「内容」字段的序列化：**不含**时间 / 版本 / 同步等元数据，
/// 因此可直接用字符串比较判断内容是否发生变化。
///
/// 键名保持 snake_case（与历史存量快照一致），反序列化走 [`snapshot_to_request`]，
/// 不依赖 `SavedRequest` 对外的 camelCase 契约。
fn build_snapshot(req: &SavedRequest) -> String {
    serde_json::json!({
        "id": req.id,
        "name": req.name,
        "method": req.method,
        "url": req.url,
        "params": req.params,
        "headers": req.headers,
        "body_type": req.body_type,
        "body": req.body,
        "form_body": req.form_body,
        "category_id": req.category_id,
        "pre_script": req.pre_script,
        "post_script": req.post_script,
        "sort_order": req.sort_order,
    })
    .to_string()
}


/// 解析版本快照为 `SavedRequest`。
///
/// 快照键是 snake_case，而 `SavedRequest` 的 serde 契约是 camelCase，
/// 直接 `from_str::<SavedRequest>` 会让 `body_type` / `form_body` / `category_id` /
/// `pre_script` / `post_script` / `sort_order` 静默落回默认值（回退即丢字段）。
/// 这里显式按 snake_case 取键，保证回退与对比拿到的都是真实内容。
fn snapshot_to_request(snapshot: &str) -> Result<SavedRequest, DbError> {
    let v: serde_json::Value = serde_json::from_str(snapshot)
        .map_err(|e| DbError::Migration(format!("快照解析失败: {e}")))?;
    let s = |key: &str| -> String {
        v.get(key).and_then(|x| x.as_str()).unwrap_or_default().to_string()
    };
    Ok(SavedRequest {
        id: s("id"),
        user_id: String::new(),
        name: s("name"),
        method: s("method"),
        url: s("url"),
        params: v.get("params").cloned().unwrap_or_else(|| serde_json::json!([])),
        headers: v.get("headers").cloned().unwrap_or_else(|| serde_json::json!([])),
        body_type: s("body_type"),
        body: s("body"),
        form_body: v.get("form_body").cloned().unwrap_or_else(|| serde_json::json!([])),
        category_id: v
            .get("category_id")
            .and_then(|x| x.as_str())
            .filter(|x| !x.is_empty())
            .map(str::to_string),
        project_id: None,
        pre_script: s("pre_script"),
        post_script: s("post_script"),
        sort_order: v.get("sort_order").and_then(|x| x.as_i64()).unwrap_or(0) as i32,
        create_time: String::new(),
        create_by: String::new(),
        update_time: String::new(),
        update_by: String::new(),
        deleted: false,
        current_user_role: String::new(),
        server_update_time: String::new(),
        sync_version: 1,
    })
}


/// 计算本次写入应使用的版本号，并返回内容是否发生变化。
///
/// 规则（需求：有数据修改才生成版本）：
/// - 内容与**当前版本快照**一致 → 复用当前版本号，且不新增快照；
/// - 内容变化（或首次写入）→ 版本号取快照表 MAX(version) + 1。
///
/// 版本号基准取 `ch_request_versions` 的 MAX(version) 而非 `ch_saved_requests.version`：
/// 回退历史版本会把当前版本号改小（需求：回退不生成新版本），若仍以它为基准，
/// 回退后再编辑会算出与既有历史相同的版本号并覆盖旧快照。
fn resolve_version(
    db: &Connection,
    req: &SavedRequest,
    mode: DataMode,
    snapshot: &str,
) -> Result<(i64, bool), DbError> {
    let current_version: Option<i64> = db
        .query_row(
            "SELECT version FROM ch_saved_requests WHERE id = ?1 AND data_mode = ?2",
            params![req.id, mode.as_db_value()],
            |row| row.get(0),
        )
        .optional()
        .map_err(DbError::Sql)?;
    let max_version: i64 = db
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM ch_request_versions WHERE request_id = ?1 AND data_mode = ?2",
            params![req.id, mode.as_db_value()],
            |row| row.get(0),
        )
        .unwrap_or(0);
    let Some(cv) = current_version else {
        // 首次写入：版本从 1 开始
        return Ok((max_version + 1, true));
    };
    let current_snapshot: Option<String> = db
        .query_row(
            "SELECT snapshot FROM ch_request_versions WHERE request_id = ?1 AND version = ?2 AND data_mode = ?3",
            params![req.id, cv, mode.as_db_value()],
            |row| row.get(0),
        )
        .optional()
        .map_err(DbError::Sql)?;
    if current_snapshot.as_deref() == Some(snapshot) {
        Ok((cv, false))
    } else {
        Ok((max_version + 1, true))
    }
}


/// 写入一条请求版本快照
fn snapshot_request_version(
    db: &Connection,
    req: &SavedRequest,
    version: i32,
    mode: DataMode,
) -> Result<(), DbError> {
    let snapshot = build_snapshot(req);
    let id = format!("{}-v{}", req.id, version);
    db.execute(
        "INSERT OR REPLACE INTO ch_request_versions
           (id, request_id, version, snapshot, create_time, update_time, data_mode)
         VALUES (?1,?2,?3,?4,?5,?6,?7)",
        params![
            id,
            req.id,
            version,
            snapshot,
            // 快照对应的是该次编辑，create_time 沿用请求的 update_time
            req.update_time,
            // `ch_request_versions.update_time` 在 schema 里是 NOT NULL 且**没有默认值**，
            // 必须显式写入；此前漏了这一列，SQLite 会绑 NULL 并报
            // `NOT NULL constraint failed: ch_request_versions.update_time`，
            // 导致「更新接口」整条链路失败。
            now_timestamp(),
            mode.as_db_value(),
        ],
    )
    .map_err(DbError::Sql)?;
    Ok(())
}


/// 查询某请求的所有版本元信息（不含完整快照，列表用）。
///
/// `is_current` 标记该快照是否就是当前生效版本（`ch_saved_requests.version`），
/// 供前端高亮「当前版本」并禁止回退到自身。
pub fn get_request_versions(conn: &DbConn, request_id: &str, mode: DataMode) -> Result<Vec<RequestVersionMeta>, DbError> {
    let db = lock_db(conn)?;
    let current_version: Option<i64> = db
        .query_row(
            "SELECT version FROM ch_saved_requests WHERE id = ?1 AND data_mode = ?2",
            params![request_id, mode.as_db_value()],
            |row| row.get(0),
        )
        .optional()
        .map_err(DbError::Sql)?;
    let mut stmt = db
        .prepare_cached(
            "SELECT id, version, create_time FROM ch_request_versions WHERE request_id = ?1 AND data_mode = ?2 ORDER BY version DESC",
        )?;
    let rows = stmt.query_map(params![request_id, mode.as_db_value()], |row| {
        let version: i32 = row.get(1)?;
        Ok(RequestVersionMeta {
            id: row.get(0)?,
            version,
            create_time: row.get::<_, String>(2).unwrap_or_default(),
            is_current: current_version == Some(version as i64),
        })
    })?;
    let mut items = Vec::new();
    for r in rows {
        items.push(r.map_err(DbError::Sql)?);
    }
    Ok(items)
}


/// 读取指定版本的完整快照（用于版本对比等只读场景）。
pub fn get_request_version_snapshot(
    conn: &DbConn,
    request_id: &str,
    version: i32,
    mode: DataMode,
) -> Result<SavedRequest, DbError> {
    let db = lock_db(conn)?;
    let snapshot: String = db
        .query_row(
            "SELECT snapshot FROM ch_request_versions WHERE request_id = ?1 AND version = ?2 AND data_mode = ?3",
            params![request_id, version, mode.as_db_value()],
            |row| row.get(0),
        )
        .map_err(|_| DbError::Sql(rusqlite::Error::QueryReturnedNoRows))?;
    snapshot_to_request(&snapshot)
}


/// 恢复某请求的指定版本。
///
/// 需求：回退本身**不生成新版本**。实现上把快照内容写回 `ch_saved_requests`，
/// 并把当前版本号直接指向被回退的版本；`ch_request_versions` 保持原样（历史不增不减），
/// 因此回退后列表里该版本即为「当前版本」。
pub fn restore_request_version(conn: &DbConn, request_id: &str, version: i32, mode: DataMode) -> Result<SavedRequest, DbError> {
    let db = lock_db(conn)?;
    let snapshot: String = db
        .query_row(
            "SELECT snapshot FROM ch_request_versions WHERE request_id = ?1 AND version = ?2 AND data_mode = ?3",
            params![request_id, version, mode.as_db_value()],
            |row| row.get(0),
        )
        .map_err(|_| DbError::Sql(rusqlite::Error::QueryReturnedNoRows))?;
    let snap = snapshot_to_request(&snapshot)?;
    db.execute(
        "UPDATE ch_saved_requests SET name=?2, method=?3, url=?4, params=?5, headers=?6,
         body_type=?7, body=?8, form_body=?9, category_id=?10, pre_script=?11, post_script=?12,
         sort_order=?13, update_time=?14, version=?15, dirty=1
         WHERE id=?1 AND data_mode=?16",
        params![
            snap.id, snap.name, snap.method, snap.url,
            snap.params.to_string(), snap.headers.to_string(), snap.body_type,
            snap.body, snap.form_body.to_string(), snap.category_id,
            snap.pre_script, snap.post_script,
            snap.sort_order, now_timestamp(), version,
            mode.as_db_value(),
        ],
    )
    .map_err(DbError::Sql)?;
    Ok(snap)
}


pub fn delete_saved_request_db(conn: &DbConn, id: &str, mode: DataMode) -> Result<(), DbError> {
    let db = lock_db(conn)?;
    db.execute("DELETE FROM ch_saved_requests WHERE id = ?1 AND data_mode = ?2", params![id, mode.as_db_value()])
        .map_err(DbError::Sql)?;
    Ok(())
}

// ====== 环境分组 CRUD ======

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_request() -> SavedRequest {
        SavedRequest {
            id: "req-test-1".to_string(),
            user_id: "u1".to_string(),
            name: "测试接口".to_string(),
            method: "GET".to_string(),
            url: "https://example.com".to_string(),
            params: serde_json::json!([]),
            headers: serde_json::json!([]),
            body_type: "none".to_string(),
            body: String::new(),
            form_body: serde_json::json!([]),
            category_id: None,
            project_id: None,
            pre_script: String::new(),
            post_script: String::new(),
            sort_order: 0,
            create_time: "2026-09-17 16:00:00".to_string(),
            create_by: "u1".to_string(),
            update_time: "2026-09-17 16:00:00".to_string(),
            update_by: "u1".to_string(),
            deleted: false,
            current_user_role: String::new(),
            server_update_time: String::new(),
            sync_version: 1,
        }
    }

    /// 临时库：每个用例独立目录，避免与真实库/其它用例互相干扰。
    fn temp_db(tag: &str) -> (DbConn, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("canghai-test-{tag}"));
        std::fs::create_dir_all(&dir).expect("创建临时目录失败");
        let path = dir.join("test.db");
        let _ = std::fs::remove_file(&path);
        let conn = crate::db::init(&path).expect("初始化测试库失败");
        (conn, path)
    }

    /// 回归测试：保存 / 更新接口必须能写入版本快照。
    ///
    /// `ch_request_versions.update_time` 是 NOT NULL，而快照 INSERT 曾漏写该列，
    /// 导致 SQLite 绑 NULL 并报 `NOT NULL constraint failed: ch_request_versions.update_time`，
    /// 表现为「更新接口」整体失败（错误落到 DB_ERROR 码）。
    #[test]
    fn save_and_update_request_write_version_snapshot() {
        let (conn, path) = temp_db("req-version");
        let req = sample_request();

        // 首次保存 → v1 快照
        save_saved_request(&conn, &req, DataMode::Online).expect("首次保存接口失败");
        let v1 = get_request_versions(&conn, &req.id, DataMode::Online).expect("读取版本列表失败");
        assert_eq!(v1.len(), 1, "首次保存应产生 1 条版本快照");
        assert_eq!(v1[0].version, 1);

        // 内容变化后再次保存（等价于「更新接口」）→ v2 快照
        let mut req2 = sample_request();
        req2.url = "https://example.com/v2".to_string();
        req2.update_time = "2026-09-17 16:05:00".to_string();
        save_saved_request(&conn, &req2, DataMode::Online).expect("更新接口失败");
        let v2 = get_request_versions(&conn, &req.id, DataMode::Online).expect("读取版本列表失败");
        assert_eq!(v2.len(), 2, "内容变化后应产生第 2 条版本快照");

        let _ = std::fs::remove_file(&path);
    }

    /// 需求：有数据修改才生成版本 —— 内容不变时不得新增版本。
    #[test]
    fn unchanged_content_does_not_create_version() {
        let (conn, path) = temp_db("req-version-nochange");
        let req = sample_request();
        save_saved_request(&conn, &req, DataMode::Online).expect("首次保存接口失败");

        // 仅时间戳变化（内容相同）→ 不应产生新版本
        let mut req2 = sample_request();
        req2.update_time = "2026-09-17 16:30:00".to_string();
        update_saved_request(&conn, &req2, DataMode::Online).expect("更新接口失败");
        let list = get_request_versions(&conn, &req.id, DataMode::Online).expect("读取版本列表失败");
        assert_eq!(list.len(), 1, "内容未变化不应产生新版本");

        let _ = std::fs::remove_file(&path);
    }

    /// 需求：回退不生成新版本，且当前版本指向被回退的版本。
    #[test]
    fn restore_does_not_create_version_and_marks_current() {
        let (conn, path) = temp_db("req-version-restore");
        let req = sample_request();
        save_saved_request(&conn, &req, DataMode::Online).expect("首次保存接口失败");

        let mut req2 = sample_request();
        req2.url = "https://example.com/v2".to_string();
        req2.body_type = "json".to_string();
        req2.body = "{\"a\":1}".to_string();
        req2.category_id = Some("cat-1".to_string());
        req2.pre_script = "console.log(1)".to_string();
        save_saved_request(&conn, &req2, DataMode::Online).expect("第二次保存失败");
        assert_eq!(
            get_request_versions(&conn, &req.id, DataMode::Online).unwrap().len(),
            2,
        );

        // 回退到 v1
        let restored = restore_request_version(&conn, &req.id, 1, DataMode::Online).expect("回退失败");
        assert_eq!(restored.url, "https://example.com", "回退后内容应为 v1");

        let list = get_request_versions(&conn, &req.id, DataMode::Online).expect("读取版本列表失败");
        assert_eq!(list.len(), 2, "回退不应新增版本");
        assert!(list.iter().find(|v| v.version == 1).unwrap().is_current, "v1 应被标记为当前版本");
        assert!(!list.iter().find(|v| v.version == 2).unwrap().is_current);

        // 回退后再保存（内容未变）也不应新增版本
        let restored2 = restore_request_version(&conn, &req.id, 1, DataMode::Online).expect("再次回退失败");
        update_saved_request(&conn, &restored2, DataMode::Online).expect("回退后保存失败");
        assert_eq!(
            get_request_versions(&conn, &req.id, DataMode::Online).unwrap().len(),
            2,
            "回退后内容未变不应新增版本",
        );

        // 回退后编辑：版本号应在既有最大版本上递增，而不是覆盖旧快照
        let mut edited = restored2.clone();
        edited.url = "https://example.com/v3".to_string();
        update_saved_request(&conn, &edited, DataMode::Online).expect("回退后编辑失败");
        let list = get_request_versions(&conn, &req.id, DataMode::Online).expect("读取版本列表失败");
        assert_eq!(list.len(), 3, "回退后编辑应生成 v3");
        assert_eq!(list[0].version, 3);

        let _ = std::fs::remove_file(&path);
    }

    /// 快照必须能完整还原字段（此前因 camelCase/snake_case 不匹配会丢字段）。
    #[test]
    fn version_snapshot_round_trips_all_fields() {
        let (conn, path) = temp_db("req-version-roundtrip");
        let mut req = sample_request();
        req.body_type = "json".to_string();
        req.body = "{\"a\":1}".to_string();
        req.category_id = Some("cat-9".to_string());
        req.pre_script = "pre".to_string();
        req.post_script = "post".to_string();
        req.sort_order = 7;
        req.headers = serde_json::json!([{ "key": "X-A", "value": "1" }]);
        save_saved_request(&conn, &req, DataMode::Online).expect("保存失败");

        let snap = get_request_version_snapshot(&conn, &req.id, 1, DataMode::Online).expect("读取快照失败");
        assert_eq!(snap.body_type, "json");
        assert_eq!(snap.body, "{\"a\":1}");
        assert_eq!(snap.category_id.as_deref(), Some("cat-9"));
        assert_eq!(snap.pre_script, "pre");
        assert_eq!(snap.post_script, "post");
        assert_eq!(snap.sort_order, 7);

        let _ = std::fs::remove_file(&path);
    }
}
