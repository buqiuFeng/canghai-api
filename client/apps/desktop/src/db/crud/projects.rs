//! 项目与项目成员的本地读写实现（由原 `db.rs` 拆分，Phase 8.1）。

use crate::db::*;
use rusqlite::params;

/// 获取项目列表。
/// - offline 模式（本地数据）不按用户过滤，保持原有行为。
/// - online 模式：
///   - user_id 为空 → 无有效用户身份（未登录/已登出），返回空集，看不到任何在线项目；
///   - user_id == "*" → 全量（用于同步内部收集，绕过用户过滤）；
///   - 其他非空值 → 按该 userId 过滤（项目绑定用户）。
pub fn get_all_projects(conn: &DbConn, mode: DataMode, user_id: &str) -> Result<Vec<Project>, DbError> {
    let db = lock_db(conn)?;
    let mode_val: &'static str = mode.as_db_value();
    if mode == DataMode::Online && user_id.is_empty() {
        return Ok(Vec::new());
    }
    let (sql, params): (&str, Vec<&dyn rusqlite::ToSql>) = if mode == DataMode::Online && user_id != "*" {
        (
            "SELECT id, user_id, name, parent_id, sort_order, expanded, create_time, create_by, update_time, update_by FROM ch_projects WHERE data_mode = ?1 AND user_id = ?2 AND deleted = 0 ORDER BY sort_order",
            vec![&mode_val as &dyn rusqlite::ToSql, &user_id as &dyn rusqlite::ToSql],
        )
    } else {
        (
            "SELECT id, user_id, name, parent_id, sort_order, expanded, create_time, create_by, update_time, update_by FROM ch_projects WHERE data_mode = ?1 AND deleted = 0 ORDER BY sort_order",
            vec![&mode_val as &dyn rusqlite::ToSql],
        )
    };
    let mut stmt = db.prepare_cached(sql)?;
    let rows = stmt
        .query_map(rusqlite::params_from_iter(params), |row| {
            Ok(Project {
                id: row.get(0)?,
                user_id: row.get::<_, String>(1).unwrap_or_default(),
                name: row.get(2)?,
                parent_id: row.get(3)?,
                sort_order: row.get(4)?,
                expanded: row.get::<_, bool>(5)?,
                create_time: row.get::<_, String>(6).unwrap_or_default(),
                create_by: row.get::<_, String>(7).unwrap_or_default(),
                update_time: row.get::<_, String>(8).unwrap_or_default(),
                update_by: row.get::<_, String>(9).unwrap_or_default(),
                deleted: false,
                current_user_role: String::new(),
            })
        })
        .map_err(DbError::Sql)?;
    let mut cats = Vec::new();
    for r in rows {
        cats.push(r.map_err(DbError::Sql)?);
    }
    Ok(cats)
}


/// 按 id 列表查询指定模式下的项目（用于补充“作为成员可见”的项目）。
pub fn get_projects_by_ids(conn: &DbConn, mode: DataMode, ids: &[String]) -> Result<Vec<Project>, DbError> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let db = lock_db(conn)?;
    let mode_val: &'static str = mode.as_db_value();
    // 用参数占位符拼接 IN 列表
    let placeholders: Vec<String> = (1..=ids.len()).map(|i| format!("?{}", i + 1)).collect();
    let sql = format!(
        "SELECT id, user_id, name, parent_id, sort_order, expanded, create_time, create_by, update_time, update_by FROM ch_projects WHERE data_mode = ?1 AND deleted = 0 AND id IN ({}) ORDER BY sort_order",
        placeholders.join(", ")
    );
    let mut params: Vec<&dyn rusqlite::ToSql> = vec![&mode_val as &dyn rusqlite::ToSql];
    for id in ids {
        params.push(id as &dyn rusqlite::ToSql);
    }
    let mut stmt = db.prepare_cached(&sql)?;
    let rows = stmt
        .query_map(rusqlite::params_from_iter(params), |row| {
            Ok(Project {
                id: row.get(0)?,
                user_id: row.get::<_, String>(1).unwrap_or_default(),
                name: row.get(2)?,
                parent_id: row.get(3)?,
                sort_order: row.get(4)?,
                expanded: row.get::<_, bool>(5)?,
                create_time: row.get::<_, String>(6).unwrap_or_default(),
                create_by: row.get::<_, String>(7).unwrap_or_default(),
                update_time: row.get::<_, String>(8).unwrap_or_default(),
                update_by: row.get::<_, String>(9).unwrap_or_default(),
                deleted: false,
                current_user_role: String::new(),
            })
        })
        .map_err(DbError::Sql)?;
    let mut result = Vec::new();
    for r in rows {
        result.push(r.map_err(DbError::Sql)?);
    }
    Ok(result)
}


pub fn save_project(conn: &DbConn, cat: &Project, mode: DataMode) -> Result<(), DbError> {
    let db = lock_db(conn)?;
    // upsert：项目改为软删后，同一 id 可能以 deleted = 1 残留在本地，
    // 直接 INSERT 会撞主键约束；这里覆盖已存在行并把 deleted 重置为 0（保存即视为恢复）。
    db.execute(
        "INSERT INTO ch_projects (id, user_id, name, parent_id, sort_order, expanded, create_time, create_by, update_time, update_by, deleted, data_mode)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 0, ?11)
         ON CONFLICT(id) DO UPDATE SET
           user_id=excluded.user_id, name=excluded.name, parent_id=excluded.parent_id,
           sort_order=excluded.sort_order, expanded=excluded.expanded,
           update_time=excluded.update_time, update_by=excluded.update_by,
           deleted=0, data_mode=excluded.data_mode",
        params![cat.id, cat.user_id, cat.name, cat.parent_id, cat.sort_order, cat.expanded, cat.create_time, cat.create_by, cat.update_time, cat.update_by, mode.as_db_value()],
    )
    .map_err(DbError::Sql)?;
    Ok(())
}


pub fn update_project_db(conn: &DbConn, id: &str, name: &str, update_by: &str, mode: DataMode) -> Result<(), DbError> {
    let db = lock_db(conn)?;
    let now = now_timestamp();
    db.execute(
        "UPDATE ch_projects SET name = ?1, update_time = ?2, update_by = ?3 WHERE id = ?4 AND data_mode = ?5",
        params![name, now, update_by, id, mode.as_db_value()],
    )
    .map_err(DbError::Sql)?;
    Ok(())
}


pub fn update_project_expanded(conn: &DbConn, id: &str, expanded: bool, mode: DataMode) -> Result<(), DbError> {
    let db = lock_db(conn)?;
    db.execute(
        "UPDATE ch_projects SET expanded = ?1 WHERE id = ?2 AND data_mode = ?3",
        params![expanded, id, mode.as_db_value()],
    )
    .map_err(DbError::Sql)?;
    Ok(())
}


/// 删除项目：改为**软删**（`deleted = 1`），与分类等实体保持一致，
/// 读路径（`get_all_projects` / `get_projects_by_ids`）会过滤掉软删行。
/// 不再物理删除，因此也不需要再打开 foreign_keys。
pub fn delete_project_db(conn: &DbConn, id: &str, mode: DataMode) -> Result<(), DbError> {
    let db = lock_db(conn)?;
    db.execute(
        "UPDATE ch_projects SET deleted = 1 WHERE id = ?1 AND data_mode = ?2",
        params![id, mode.as_db_value()],
    )
    .map_err(DbError::Sql)?;
    Ok(())
}

// ====== 项目成员 CRUD ======


pub fn get_project_members_db(conn: &DbConn, project_id: &str, mode: DataMode) -> Result<Vec<ProjectMemberInfo>, DbError> {
    let db = lock_db(conn)?;
    let mut stmt = db.prepare_cached(
        "SELECT id, project_id, member_type, member_id, role, create_time, member_name FROM ch_project_members WHERE project_id = ?1 AND data_mode = ?2 AND deleted = 0 ORDER BY member_type, create_time"
    )?;
    let rows = stmt
        .query_map(params![project_id, mode.as_db_value()], |row| {
            Ok(ProjectMemberInfo {
                id: row.get::<_, String>(0).unwrap_or_default(),
                project_id: row.get::<_, String>(1).unwrap_or_default(),
                member_type: row.get::<_, String>(2).unwrap_or_default(),
                member_id: row.get::<_, String>(3).unwrap_or_default(),
                role: row.get::<_, String>(4).unwrap_or_default(),
                create_time: row.get::<_, String>(5).unwrap_or_default(),
                member_name: row.get::<_, String>(6).unwrap_or_default(),
            })
        })
        .map_err(DbError::Sql)?;
    let mut list = Vec::new();
    for r in rows {
        list.push(r.map_err(DbError::Sql)?);
    }
    Ok(list)
}


pub fn save_project_member_db(conn: &DbConn, m: &ProjectMemberInfo, mode: DataMode) -> Result<(), DbError> {
    let db = lock_db(conn)?;
    db.execute(
        "INSERT OR REPLACE INTO ch_project_members (id, project_id, member_type, member_id, member_name, role, create_time, create_by, update_time, update_by, deleted, data_mode) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 0, ?11)",
        params![m.id, m.project_id, m.member_type, m.member_id, m.member_name, m.role, m.create_time, "", m.create_time, "", mode.as_db_value()],
    ).map_err(DbError::Sql)?;
    Ok(())
}


pub fn remove_project_member_db(conn: &DbConn, project_id: &str, member_type: &str, member_id: &str, mode: DataMode) -> Result<(), DbError> {
    let db = lock_db(conn)?;
    db.execute(
        "UPDATE ch_project_members SET deleted = 1 WHERE project_id = ?1 AND member_type = ?2 AND member_id = ?3 AND data_mode = ?4",
        params![project_id, member_type, member_id, mode.as_db_value()],
    ).map_err(DbError::Sql)?;
    Ok(())
}

// ====== 分类 CRUD（项目下多级分类）=====
