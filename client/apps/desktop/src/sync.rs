use crate::db::{self, DbConn, DbError, DataMode, Project, Category, SavedRequest, Environment, EnvironmentGroup, EnvironmentVariable, Team, TeamMember, ProjectMember};
use serde::{Deserialize, Serialize};
use rusqlite::Connection;

// ==================== 同步数据模型 ====================

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SyncData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    pub categories: Vec<Category>,
    pub requests: Vec<SavedRequest>,
    #[serde(rename = "environmentGroups")]
    pub environment_groups: Vec<EnvironmentGroup>,
    pub environments: Vec<Environment>,
    #[serde(rename = "environmentVariables")]
    pub environment_variables: Vec<EnvironmentVariable>,
    pub last_sync_at: Option<i64>,
    /// 客户端保存的服务端时间游标（serverTime）。非空时服务端只下发该时刻之后的变更。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_sync_time: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SyncResponse {
    /// 列表字段统一容错 null（无可见项目时服务端提前返回，各列表字段为 null）。
    #[serde(default, deserialize_with = "crate::db::null_to_empty_vec")]
    pub projects: Vec<Project>,
    #[serde(default, deserialize_with = "crate::db::null_to_empty_vec")]
    pub categories: Vec<Category>,
    #[serde(default, deserialize_with = "crate::db::null_to_empty_vec")]
    pub teams: Vec<Team>,
    #[serde(rename = "teamMembers", default, deserialize_with = "crate::db::null_to_empty_vec")]
    pub team_members: Vec<TeamMember>,
    #[serde(rename = "projectMembers", default, deserialize_with = "crate::db::null_to_empty_vec")]
    pub project_members: Vec<ProjectMember>,
    #[serde(default, deserialize_with = "crate::db::null_to_empty_vec")]
    pub requests: Vec<SavedRequest>,
    #[serde(rename = "environmentGroups", default, deserialize_with = "crate::db::null_to_empty_vec")]
    pub environment_groups: Vec<EnvironmentGroup>,
    #[serde(default, deserialize_with = "crate::db::null_to_empty_vec")]
    pub environments: Vec<Environment>,
    #[serde(rename = "environmentVariables", default, deserialize_with = "crate::db::null_to_empty_vec")]
    pub environment_variables: Vec<EnvironmentVariable>,
    pub sync_at: i64,
    /// 服务端时间游标（yyyy-MM-dd HH:mm:ss.SSS），客户端需持久化并在下次请求回传
    #[serde(default)]
    pub server_time: Option<String>,
    /// 本次是否为增量下发
    #[serde(default)]
    pub incremental: Option<bool>,
    /// 增量区间内服务端已删除的实体 id（墓碑）
    #[serde(default, deserialize_with = "crate::db::null_to_empty_vec")]
    pub deleted_ids: Vec<String>,
}

// ==================== 服务端统一响应 ====================

/// 与服务端统一响应结构 `{success, code, msg, data}` 保持一致。
/// - `code`: 业务码，后端约定 0 表示成功，非 0 表示失败。
/// - `data`: 使用 `Option<T>`，因为服务端在 `success=false` 时通常不返回 data 字段。
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ApiResult<T> {
    pub success: bool,
    #[serde(default)]
    pub code: i32,
    pub msg: String,
    pub data: Option<T>,
}

/// 本地错误码常量（与 `docs/API_CONTRACT.md` §3 对齐）。
///
/// Java `ErrorCode` 枚举是唯一权威定义，Rust 侧仅作同义常量，避免散用魔法数字 `-1`
/// 导致「所有失败长得一样」、前端无法按码分支处理。
///
/// 常量按契约表全量声明（便于按码对照），并非每个都被当前代码引用，故允许 dead_code。
#[allow(dead_code)]
pub mod codes {
    pub const SUCCESS: i32 = 0;
    /// 参数无效（40000）
    pub const PARAM_INVALID: i32 = 40000;
    /// 缺少必填参数（40001）
    pub const PARAM_MISSING: i32 = 40001;
    /// 未登录 / token 无效（40100）
    pub const UNAUTHORIZED: i32 = 40100;
    /// 无权限（40103）
    pub const FORBIDDEN: i32 = 40103;
    /// 资源不存在（40400）
    pub const NOT_FOUND: i32 = 40400;
    /// 资源冲突（40900）
    pub const CONFLICT: i32 = 40900;
    /// 业务处理失败（42200）
    pub const BUSINESS_ERROR: i32 = 42200;
    /// 服务器内部错误（50000）
    pub const INTERNAL_ERROR: i32 = 50000;
    /// 本地数据库错误（50001）
    pub const DB_ERROR: i32 = 50001;
    /// 下游服务（服务端 / 网络）错误（50002）
    pub const DOWNSTREAM_ERROR: i32 = 50002;
}

impl<T> ApiResult<T> {
    /// 成功响应：code=0，data 承载业务数据。
    pub fn ok(data: T) -> Self {
        ApiResult { success: true, code: codes::SUCCESS, msg: String::new(), data: Some(data) }
    }

    /// 失败响应：success=false，携带业务码与错误信息。
    pub fn err(code: i32, msg: String) -> Self {
        ApiResult { success: false, code, msg, data: None }
    }

    /// 由本地 SQLite 错误构造失败响应：错误码来自 [`DbError::code`] 映射，
    /// 取代原先一律压成 `-1` 的降级处理（丢失错误类型，前端无法区分）。
    pub fn from_db_err(e: crate::db::DbError) -> Self {
        ApiResult::err(e.code(), e.to_string())
    }

    /// 是否成功：success 为真且 code 为 0（与后端约定对齐）。
    pub fn is_success(&self) -> bool {
        self.success && self.code == 0
    }
}

// ==================== 收集本地数据 ====================

/// `only_dirty = true` 时只收集 dirty=1 的待上传行（Phase 4 增量上传）。
pub fn collect_all(conn: &DbConn, mode: DataMode, only_dirty: bool) -> Result<SyncData, DbError> {
    // "*" 为全量哨兵：同步是团队级全量收集，需绕过 userId 过滤。
    // 团队与项目脱钩：数据按项目归属收集，服务端按团队关联的项目集合（allowed）过滤，
    // 因此本地全量收集即可，无需按 team_id 裁剪。
    // 说明：项目/团队不在此上报（项目经 /api/v1/project/save 实时落库，团队仅服务端权威下发）。
    let project_ids: Vec<String> = db::get_all_projects(conn, mode, "*")?.iter().map(|p| p.id.clone()).collect();
    let categories = db::get_all_categories(conn, &project_ids, mode, only_dirty)?;
    let requests = db::get_all_saved_requests(conn, &project_ids, mode, only_dirty)?;

    // 环境分组/环境：同步是团队级全量收集，直接一次性取全表（含无项目归属与各项目归属），
    // 取代原先「按项目逐个查询」的 N+1（100 项目 × 2 类 = 200+ 次查询）。
    let environment_groups = db::get_all_environment_groups_all(conn, mode, only_dirty)?;
    let environments = db::get_all_environments_all(conn, mode, only_dirty)?;

    // 环境变量同理：一次性取全量，取代按环境逐个查询的 N+1。
    let all_vars = db::get_all_env_variables_all(conn, mode, only_dirty)?;

    Ok(SyncData {
        token: None,
        categories,
        requests,
        environment_groups,
        environments,
        environment_variables: all_vars,
        last_sync_at: None,
        last_sync_time: None, // 由调用方从持久化的游标填入
    })
}

// ==================== 同步合并 ====================

/// 单张表的合并策略：表名 + 是否支持版本号 + 删除判定 + 插入/更新回调
struct MergeStrategy<'a, T> {
    table: &'static str,
    /// 该表是否有 `sync_version` 列（5 张同步表有；项目/团队/成员等结构表没有）
    versioned: bool,
    server_list: &'a [T],
    /// 该行在服务端是否已被软删。服务端会把软删行随列表一起下发，命中时只作删除信号，
    /// 不写入本地存活数据（见 `merge_server_data` 的返回值）。
    is_deleted: Box<dyn Fn(&T) -> bool + 'a>,
    save: Box<dyn Fn(&Connection, &T) -> Result<(), DbError> + 'a>,
    update: Box<dyn Fn(&Connection, &T) -> Result<(), DbError> + 'a>,
}

/// 将服务端返回的数据合并到本地 SQLite，以 update_time 最新者为准。
/// 所有实体表的合并逻辑一致（按 id 查本地 update_time，缺失则插入、服务端更新则覆盖），
/// 因此统一收敛到 `merge_entities`，避免逐表重复样板。
///
/// 返回值：**列表中携带的删除信号 id**（服务端下发的 `deleted = true` 行）。
/// 这些行不能写成存活数据，只能当删除处理，因此此处只收集 id，
/// 由调用方在**事务外**统一走 `db::apply_deleted_ids` 清理本地副本
/// （`apply_deleted_ids` 内部会重新加锁，在事务内调用会自锁）。
pub fn merge_server_data(conn: &DbConn, resp: &SyncResponse, mode: DataMode) -> Result<Vec<String>, DbError> {
    // 整批合并放进单个事务：N 行数据只提交一次（原先每行一次隐式事务）
    db::in_transaction(conn, |db| {
    let mut tombstoned: Vec<String> = Vec::new();
    merge_entities(db, MergeStrategy {
        table: "ch_projects",
        versioned: false,
        server_list: &resp.projects,
        is_deleted: Box::new(|e| e.deleted),
        save: Box::new(|c, e| db::save_project_for_merge(c, e, mode)),
        update: Box::new(|c, e| db::update_project_for_merge(c, e, mode)),
    }, &mut tombstoned)?;
    merge_entities(db, MergeStrategy {
        table: "ch_teams",
        versioned: false,
        server_list: &resp.teams,
        is_deleted: Box::new(|e| e.deleted),
        save: Box::new(|c, e| db::save_team_for_merge(c, e, mode)),
        update: Box::new(|c, e| db::update_team_for_merge(c, e, mode)),
    }, &mut tombstoned)?;
    merge_entities(db, MergeStrategy {
        table: "ch_team_members",
        versioned: false,
        server_list: &resp.team_members,
        // TeamMember 本地无 deleted 列，服务端也不下发其软删行：恒视为存活
        is_deleted: Box::new(|_| false),
        save: Box::new(|c, e| db::save_team_member_for_merge(c, e, mode)),
        update: Box::new(|c, e| db::update_team_member_for_merge(c, e, mode)),
    }, &mut tombstoned)?;
    merge_entities(db, MergeStrategy {
        table: "ch_project_members",
        versioned: false,
        server_list: &resp.project_members,
        is_deleted: Box::new(|e| e.deleted),
        save: Box::new(|c, e| db::save_project_member_for_merge(c, e, mode)),
        update: Box::new(|c, e| db::update_project_member_for_merge(c, e, mode)),
    }, &mut tombstoned)?;
    merge_entities(db, MergeStrategy {
        table: "ch_categories",
        versioned: true,
        server_list: &resp.categories,
        is_deleted: Box::new(|e| e.deleted),
        save: Box::new(|c, e| db::save_category_for_merge(c, e, mode)),
        update: Box::new(|c, e| db::update_category_for_merge(c, e, mode)),
    }, &mut tombstoned)?;
    merge_entities(db, MergeStrategy {
        table: "ch_saved_requests",
        versioned: true,
        server_list: &resp.requests,
        is_deleted: Box::new(|e| e.deleted),
        save: Box::new(|c, e| db::save_request_for_merge(c, e, mode)),
        update: Box::new(|c, e| db::update_request_for_merge(c, e, mode)),
    }, &mut tombstoned)?;
    merge_entities(db, MergeStrategy {
        table: "ch_environment_groups",
        versioned: true,
        server_list: &resp.environment_groups,
        is_deleted: Box::new(|e| e.deleted),
        save: Box::new(|c, e| db::save_env_group_for_merge(c, e, mode)),
        update: Box::new(|c, e| db::update_env_group_for_merge(c, e, mode)),
    }, &mut tombstoned)?;
    merge_entities(db, MergeStrategy {
        table: "ch_environments",
        versioned: true,
        server_list: &resp.environments,
        is_deleted: Box::new(|e| e.deleted),
        save: Box::new(|c, e| db::save_env_for_merge(c, e, mode)),
        update: Box::new(|c, e| db::update_env_for_merge(c, e, mode)),
    }, &mut tombstoned)?;
    merge_entities(db, MergeStrategy {
        table: "ch_environment_variables",
        versioned: true,
        server_list: &resp.environment_variables,
        is_deleted: Box::new(|e| e.deleted),
        save: Box::new(|c, e| db::save_var_for_merge(c, e, mode)),
        update: Box::new(|c, e| db::update_var_for_merge(c, e, mode)),
    }, &mut tombstoned)?;

        Ok(tombstoned)
    })
}

/// 通用合并：根据「本地 update_time + sync_version」决定插入或覆盖
///
/// 判定规则与服务端 `SyncService::compareToServer` 保持一致（先比 update_time，
/// 同一秒内的并发修改再比 sync_version），否则两端可能对同一条数据做出相反的决定，
/// 造成「本地永远赢」的静默覆盖。
///
/// `deleted = true` 的行（服务端软删）不参与上面的判定：它只是「这一行已被删除」的信号，
/// 写入本地存活数据毫无意义（必然被紧随其后的墓碑清理覆盖），故直接收集 id 并跳过写入。
fn merge_entities<T>(
    db: &Connection,
    strategy: MergeStrategy<'_, T>,
    tombstoned: &mut Vec<String>,
) -> Result<(), DbError>
where
    T: HasUpdateTime,
{
    let local_map = load_id_update_time_map(db, strategy.table, strategy.versioned)?;

    for item in strategy.server_list {
        let id = item.id();
        if (strategy.is_deleted)(item) {
            // 软删行 → 只作删除信号，交给调用方在事务外清理本地副本
            tombstoned.push(id);
            continue;
        }
        let server_ts = item.update_time();
        match local_map.get(&id) {
            None => {
                // 本地没有 → 插入
                (strategy.save)(db, item)?;
            }
            Some((local_ts, local_ver)) => {
                if server_is_newer(&server_ts, item.sync_version(), local_ts, *local_ver) {
                    // 服务端更新（或同秒但版本更高）→ 覆盖本地
                    (strategy.update)(db, item)?;
                }
                // else: 本地更新 → 保持不变
            }
        }
    }
    Ok(())
}

/// 冲突判定：> 0 表示服务端更新（覆盖本地）。先比 update_time，同秒再比 sync_version。
fn server_is_newer(server_ts: &str, server_ver: i32, local_ts: &str, local_ver: i32) -> bool {
    match server_ts.cmp(local_ts) {
        std::cmp::Ordering::Greater => true,
        std::cmp::Ordering::Less => false,
        std::cmp::Ordering::Equal => server_ver > local_ver,
    }
}

/// 合并实体需提供 id、update_time 与（可选的）sync_version 访问能力
trait HasUpdateTime {
    fn id(&self) -> String;
    fn update_time(&self) -> String;
    /// 服务端版本号；不支持版本列的表返回 0（退化为纯时间戳比较）
    fn sync_version(&self) -> i32 {
        0
    }
}

impl HasUpdateTime for Project {
    fn id(&self) -> String { self.id.clone() }
    fn update_time(&self) -> String { self.update_time.clone() }
}
impl HasUpdateTime for SavedRequest {
    fn id(&self) -> String { self.id.clone() }
    fn update_time(&self) -> String { self.update_time.clone() }
    fn sync_version(&self) -> i32 { self.sync_version }
}
impl HasUpdateTime for EnvironmentGroup {
    fn id(&self) -> String { self.id.clone() }
    fn update_time(&self) -> String { self.update_time.clone() }
    fn sync_version(&self) -> i32 { self.sync_version }
}
impl HasUpdateTime for Environment {
    fn id(&self) -> String { self.id.clone() }
    fn update_time(&self) -> String { self.update_time.clone() }
    fn sync_version(&self) -> i32 { self.sync_version }
}
impl HasUpdateTime for EnvironmentVariable {
    fn id(&self) -> String { self.id.clone() }
    fn update_time(&self) -> String { self.update_time.clone() }
    fn sync_version(&self) -> i32 { self.sync_version }
}
impl HasUpdateTime for Category {
    fn id(&self) -> String { self.id.clone() }
    fn update_time(&self) -> String { self.update_time.clone() }
    fn sync_version(&self) -> i32 { self.sync_version }
}
impl HasUpdateTime for Team {
    fn id(&self) -> String { self.id.clone() }
    fn update_time(&self) -> String { self.update_time.clone() }
}
impl HasUpdateTime for TeamMember {
    fn id(&self) -> String { self.id.clone() }
    fn update_time(&self) -> String { self.update_time.clone() }
}
impl HasUpdateTime for ProjectMember {
    fn id(&self) -> String { self.id.clone() }
    fn update_time(&self) -> String { self.update_time.clone() }
}

// ==================== 辅助函数 ====================

/// 加载某个表的 (id → (update_time, sync_version)) 映射。
/// `versioned=false` 的表（项目/团队/成员）没有版本列，以常量 0 代替，
/// 这样同一条 SQL 形状可复用，避免为无版本表单独写一套合并逻辑。
fn load_id_update_time_map(
    db: &Connection,
    table: &str,
    versioned: bool,
) -> Result<std::collections::HashMap<String, (String, i32)>, DbError> {
    let sql = if versioned {
        format!("SELECT id, update_time, sync_version FROM {table}")
    } else {
        format!("SELECT id, update_time, 0 FROM {table}")
    };
    let mut stmt = db.prepare(&sql).map_err(DbError::Sql)?;
    let rows = stmt
        .query_map([], |row| {
            let id: String = row.get(0)?;
            let ts: String = row.get(1)?;
            let ver: i32 = row.get(2)?;
            Ok((id, ts, ver))
        })
        .map_err(DbError::Sql)?;

    let mut map = std::collections::HashMap::new();
    for r in rows {
        let (id, ts, ver) = r.map_err(DbError::Sql)?;
        map.insert(id, (ts, ver));
    }
    Ok(map)
}

// ==================== 同步编排（核心业务层） ====================

use crate::infra;
use crate::db::lock_db;

/// 把服务端返回数据合并到本地，是 run_sync / run_pull 共用的核心步骤。
///
/// 1. 注入当前登录用户 id 到项目/接口：服务端数据不携带 userId（通过 ch_project_members 关联），
///    合并前注入使本地可按用户过滤（登出后查不到）。
/// 2. 认领历史遗留的在线模式孤儿数据（user_id 为空的旧数据），使其对当前用户可见。
pub fn apply_server_response(
    conn: &DbConn,
    server_data: &mut SyncResponse,
    mode: DataMode,
    uid: &str,
) -> Result<(), String> {
    for p in server_data.projects.iter_mut() {
        p.user_id = uid.to_string();
    }
    for r in server_data.requests.iter_mut() {
        r.user_id = uid.to_string();
    }
    // 合并返回「列表中携带的删除信号 id」：服务端下发的 deleted = true 行
    let payload_tombstones = merge_server_data(conn, server_data, mode).map_err(|e| e.to_string())?;

    // 统一清理删除：列表内的软删行信号 + deletedIds 墓碑。
    // 两条通道内容通常重叠（同一个软删行既在列表里、又在 deletedIds 里），
    // 这里合并去重以减少 SQL 次数；`apply_deleted_ids` 本身也是幂等的。
    if !payload_tombstones.is_empty() || !server_data.deleted_ids.is_empty() {
        let mut seen = std::collections::HashSet::new();
        let to_delete: Vec<String> = payload_tombstones
            .into_iter()
            .chain(server_data.deleted_ids.iter().cloned())
            .filter(|id| seen.insert(id.clone()))
            .collect();
        db::apply_deleted_ids(conn, &to_delete, mode).map_err(|e| e.to_string())?;
    }

    // 仅在线模式、且当前用户已登录时认领孤儿数据
    if !uid.is_empty() && mode == DataMode::Online {
        let db = lock_db(conn).map_err(|e| e.to_string())?;
        db.execute(
            "UPDATE ch_projects SET user_id = ?1 WHERE data_mode = 'online' AND deleted = 0 AND (user_id IS NULL OR user_id = '')",
            rusqlite::params![uid.to_string()],
        ).map_err(|e| format!("认领历史项目失败: {e}"))?;
        db.execute(
            "UPDATE ch_saved_requests SET user_id = ?1 WHERE data_mode = 'online' AND (user_id IS NULL OR user_id = '')",
            rusqlite::params![uid.to_string()],
        ).map_err(|e| format!("认领历史接口失败: {e}"))?;
    }
    Ok(())
}

// ==================== 同步互斥与重试 ====================

/// 同步进行中标记。同步是「读全量 → 上传 → 合并 → 清 dirty」的多步过程，
/// 并发执行会互相覆盖（后完成者基于过期快照写回），因此必须串行化。
static SYNC_RUNNING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// RAII 守卫：无论同步成功还是提前返回，退出时都会释放锁，避免异常路径死锁。
struct SyncGuard;

impl Drop for SyncGuard {
    fn drop(&mut self) {
        SYNC_RUNNING.store(false, std::sync::atomic::Ordering::SeqCst);
    }
}

/// 尝试获取同步锁；已被占用返回 None（调用方应提示「同步进行中」而非排队等待）。
fn try_lock_sync() -> Option<SyncGuard> {
    SYNC_RUNNING
        .compare_exchange(
            false,
            true,
            std::sync::atomic::Ordering::SeqCst,
            std::sync::atomic::Ordering::SeqCst,
        )
        .ok()
        .map(|_| SyncGuard)
}

/// 网络层错误的重试次数（首次请求 + 2 次重试，退避 300ms / 600ms）。
const NETWORK_RETRY_ATTEMPTS: u32 = 3;

/// 带指数退避的 POST。
///
/// 仅对**传输层错误**重试（连接失败、超时、TLS 抖动）：业务错误（4xx/5xx 响应体）
/// 已经到达服务端，重试可能造成重复写入，故直接返回由上层处理。
async fn post_json_with_retry(
    url: &str,
    payload: &serde_json::Value,
    max_attempts: u32,
) -> Result<(reqwest::StatusCode, String), String> {
    let attempts = max_attempts.max(1);
    let mut last_err = String::from("未知网络错误");
    for attempt in 0..attempts {
        if attempt > 0 {
            // 指数退避：300ms → 600ms → 1200ms
            let delay_ms = 300u64 << (attempt - 1).min(3);
            tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
        }
        let client = infra::http_client()?;
        match client
            .post(url)
            .header("Content-Type", "application/json; charset=utf-8")
            .json(payload)
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status();
                return match resp.text().await {
                    Ok(t) => Ok((status, t)),
                    Err(e) => Err(format!("读取响应失败: {e}")),
                };
            }
            Err(e) => {
                last_err = format!("请求服务端失败[{url}]: {e}");
            }
        }
    }
    Err(last_err)
}

/// 执行一次同步：收集本地全量数据 -> POST 到服务端 -> 合并返回数据。
/// 命令层仅负责取连接与配置，业务编排下沉于此（瘦命令 / 厚服务）。
pub async fn run_sync(
    conn: &DbConn,
    server_url: &str,
    config: &mut crate::commands::sync::SyncConfig,
    mode: DataMode,
) -> ApiResult<String> {
    // 0. 并发保护：已有同步在跑则直接拒绝，避免双向覆盖
    let _guard = match try_lock_sync() {
        Some(g) => g,
        None => return ApiResult::err(codes::CONFLICT, "同步正在进行中，请稍后再试".to_string()),
    };

    // 埋点基线：同步端到端耗时（收集 -> 上传 -> 合并 -> 清 dirty）
    let started = std::time::Instant::now();

    // 1. 收集本地待上传数据（仅 dirty=1 的行；存量行首次默认为 1，故首次仍是全量）
    let mut payload = match collect_all(conn, mode, true) {
        Ok(p) => p,
        Err(e) => return ApiResult::err(codes::DB_ERROR, format!("收集本地数据失败: {e}")),
    };
    payload.token = Some(config.auth_token.clone());
    // 带上本地游标，服务端据此只回传增量（不传则全量）
    payload.last_sync_time = config.last_sync_time.clone();

    // 2. POST 到服务端（网络层错误自动退避重试）
    let url = format!("{}/api/v1/sync/upload", server_url);
    let (status, body_text) = match post_json_with_retry(
        &url,
        &serde_json::to_value(&payload).unwrap_or(serde_json::Value::Null),
        NETWORK_RETRY_ATTEMPTS,
    )
    .await
    {
        Ok(r) => r,
        Err(e) => return ApiResult::err(codes::DOWNSTREAM_ERROR, e),
    };

    if !status.is_success() {
        return ApiResult::err(status.as_u16() as i32, format!("服务端返回错误{}: {}", status.as_u16(), body_text));
    }

    let api_result: ApiResult<SyncResponse> = match serde_json::from_str(&body_text) {
        Ok(r) => r,
        Err(e) => return ApiResult::err(codes::DOWNSTREAM_ERROR, format!("解析响应 JSON 失败: {e}\n响应内容: {body_text}")),
    };

    if !api_result.is_success() {
        return ApiResult::err(api_result.code, format!("服务端业务错误: {}", api_result.msg));
    }

    let mut server_data = match api_result.data {
        Some(d) => d,
        None => return ApiResult::err(api_result.code, format!("服务端未返回数据（msg: {}）", api_result.msg)),
    };

    // 3. 合并服务端返回数据到本地（注入 user_id + 认领历史孤儿 + 墓碑删除）
    if let Err(e) = apply_server_response(conn, &mut server_data, mode, &config.user_id) {
        return ApiResult::err(codes::DB_ERROR, e);
    }

    // 4. 推进增量游标：使用服务端返回的时间，避免依赖本地时钟
    if let Some(st) = server_data.server_time.as_deref() {
        let st = st.trim();
        if !st.is_empty() {
            config.last_sync_time = Some(st.to_string());
        }
    }

    // 5. 本次变更已上传成功 → 清除本地 dirty 标记，下次同步即为纯增量
    if let Err(e) = db::clear_dirty_flags(conn, mode) {
        return ApiResult::err(codes::DB_ERROR, format!("清除本地待上传标记失败: {e}"));
    }

    let summary = format!("同步成功，收到 {} 个项目、{} 个分类、{} 个接口、{} 个环境、{} 个分组、{} 个变量",
        server_data.projects.len(),
        server_data.categories.len(),
        server_data.requests.len(),
        server_data.environments.len(),
        server_data.environment_groups.len(),
        server_data.environment_variables.len(),
    );
    log::info!("run_sync cost={}ms {}", started.elapsed().as_millis(), summary);
    ApiResult::ok(summary)
}

/// 仅从服务端拉取最新数据并合并到本地（不上传本地变更）。
pub async fn run_pull(
    conn: &DbConn,
    server_url: &str,
    config: &mut crate::commands::sync::SyncConfig,
    mode: DataMode,
) -> ApiResult<String> {
    // 0. 并发保护：与 upload 共用同一把锁，避免 pull 与 upload 交叉执行
    let _guard = match try_lock_sync() {
        Some(g) => g,
        None => return ApiResult::err(codes::CONFLICT, "同步正在进行中，请稍后再试".to_string()),
    };

    // 埋点基线：拉取端到端耗时
    let started = std::time::Instant::now();

    // 1. 拉取服务端数据（仅 pull，不上传）；带上游标时服务端只回传变更
    let url = format!("{}/api/v1/sync/pull", server_url);
    let (status, body_text) = match post_json_with_retry(
        &url,
        &serde_json::json!({
            "token": config.auth_token
        }),
        NETWORK_RETRY_ATTEMPTS,
    )
    .await
    {
        Ok(r) => r,
        Err(e) => return ApiResult::err(codes::DOWNSTREAM_ERROR, e),
    };

    if !status.is_success() {
        return ApiResult::err(status.as_u16() as i32, format!("服务端返回错误{}: {}", status.as_u16(), body_text));
    }

    let api_result: ApiResult<SyncResponse> = match serde_json::from_str(&body_text) {
        Ok(r) => r,
        Err(e) => return ApiResult::err(codes::DOWNSTREAM_ERROR, format!("解析响应 JSON 失败: {e}\n响应内容: {body_text}")),
    };

    if !api_result.is_success() {
        return ApiResult::err(api_result.code, format!("服务端业务错误: {}", api_result.msg));
    }

    let mut server_data = match api_result.data {
        Some(d) => d,
        None => return ApiResult::err(api_result.code, format!("服务端未返回数据（msg: {}）", api_result.msg)),
    };

    // 2. 合并服务端数据到本地（仅读取服务器最新数据，覆盖本地缓存）
    if let Err(e) = apply_server_response(conn, &mut server_data, mode, &config.user_id) {
        return ApiResult::err(codes::DB_ERROR, e);
    }

    // 3. 推进增量游标（pull 不上传本地数据，因此不清 dirty）
    if let Some(st) = server_data.server_time.as_deref() {
        let st = st.trim();
        if !st.is_empty() {
            config.last_sync_time = Some(st.to_string());
        }
    }

    let summary = format!("拉取成功，收到 {} 个项目、{} 个分类、{} 个接口、{} 个环境、{} 个分组、{} 个变量",
        server_data.projects.len(),
        server_data.categories.len(),
        server_data.requests.len(),
        server_data.environments.len(),
        server_data.environment_groups.len(),
        server_data.environment_variables.len(),
    );
    log::info!("run_pull cost={}ms {}", started.elapsed().as_millis(), summary);
    ApiResult::ok(summary)
}
