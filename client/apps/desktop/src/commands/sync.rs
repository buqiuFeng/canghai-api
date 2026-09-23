use crate::db::{AppDb, DataMode};
use crate::infra;
use crate::sync::codes;
use crate::jwt;
use crate::sync;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::Manager;

// ==================== 同步配置 ====================

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SyncConfig {
    /// 前端可能只传它关心的字段（如仅改 serverUrl），故未传时按空处理，
    /// 由 `set_sync_config` 做「读-合并-写」，避免少传字段就清掉既有配置。
    #[serde(default)]
    pub server_url: String,
    #[serde(default)]
    pub team_id: String,
    #[serde(default)]
    pub auth_token: String,
    #[serde(default)]
    pub user_id: String,
    /// 增量同步游标：上次同步时服务端返回的 serverTime（服务端时钟，避免本地时钟漂移）。
    /// 为空表示下次做全量同步。
    #[serde(default)]
    pub last_sync_time: Option<String>,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            server_url: "http://localhost:8092".to_string(),
            team_id: "default".to_string(),
            auth_token: String::new(),
            user_id: String::new(),
            last_sync_time: None,
        }
    }
}

/// 读取同步配置（委托 infra 层单一实现）。
fn load_config(app_data_dir: &PathBuf) -> SyncConfig {
    infra::load_config(app_data_dir)
}

/// 持久化同步配置（委托 infra 层单一实现，含目录创建）。
fn save_config(app_data_dir: &PathBuf, config: &SyncConfig) -> Result<(), String> {
    infra::save_config(app_data_dir, config)
}

// ==================== 登录/注册请求模型 ====================

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    pub token: String,
    pub user: UserInfo,
    #[serde(rename = "expiresAt")]
    pub expires_at: u64,
    #[serde(default)]
    #[serde(rename = "teamId")]
    pub team_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserInfo {
    pub id: String,
    pub username: String,
    #[serde(default)]
    pub nickname: String,
    pub email: String,
}

// ==================== Tauri 命令 ====================

/// 获取当前同步配置
#[tauri::command]
pub fn get_sync_config(app: tauri::AppHandle) -> Result<sync::ApiResult<SyncConfig>, String> {
    let app_data_dir = match app.path().app_data_dir() {
        Ok(d) => d,
        Err(e) => return Ok(sync::ApiResult::err(codes::INTERNAL_ERROR, format!("获取数据目录失败: {e}"))),
    };
    Ok(sync::ApiResult::ok(load_config(&app_data_dir)))
}

/// 保存同步配置（**读-合并-写**）。
///
/// 前端各处只会传它关心的字段（保存全局设置只传 serverUrl、保存同步设置传 serverUrl+teamId、
/// 登录后持久化传 serverUrl+teamId+authToken）。若直接整体覆盖落盘，
/// 「少传的字段」会被重置为空——我们已经实测到落盘文件里的 `userId` 与 `lastSyncTime`
/// 被前端写入抹掉（`"userId": ""`、`"lastSyncTime": null`）。
/// 因此这里统一合并：**只有非空字段才覆盖**，其余保持既有值。
#[tauri::command]
pub fn set_sync_config(app: tauri::AppHandle, config: SyncConfig) -> Result<sync::ApiResult<()>, String> {
    let app_data_dir = match app.path().app_data_dir() {
        Ok(d) => d,
        Err(e) => return Ok(sync::ApiResult::err(codes::INTERNAL_ERROR, format!("获取数据目录失败: {e}"))),
    };
    let mut merged = load_config(&app_data_dir);
    if !config.server_url.trim().is_empty() {
        merged.server_url = config.server_url.trim().trim_end_matches('/').to_string();
    }
    if !config.team_id.trim().is_empty() {
        merged.team_id = config.team_id.trim().to_string();
    }
    if !config.user_id.trim().is_empty() {
        merged.user_id = config.user_id.trim().to_string();
    }
    if config.last_sync_time.is_some() {
        merged.last_sync_time = config.last_sync_time.clone();
    }
    if !config.auth_token.trim().is_empty() {
        merged.auth_token = config.auth_token.trim().to_string();
    }
    match save_config(&app_data_dir, &merged) {
        Ok(_) => Ok(sync::ApiResult::ok(())),
        Err(e) => Ok(sync::ApiResult::err(codes::INTERNAL_ERROR, e)),
    }
}

/// 登录/注册成功后把 auth_token / server_url / user_id 持久化到配置，
/// 供后续 `call_server_api` / `pull_data` 复用同一后端实例。消除 login/register 重复样板。
fn persist_auth(
    app_data_dir: &PathBuf,
    server_url: &str,
    login_data: &LoginResponse,
) -> Result<(), String> {
    let mut config = load_config(app_data_dir);
    config.server_url = server_url.trim().trim_end_matches('/').to_string();
    config.auth_token = login_data.token.clone();
    config.user_id = login_data.user.id.clone();
    save_config(app_data_dir, &config)
}

/// 用户登录
#[tauri::command]
pub async fn login(
    app: tauri::AppHandle,
    server_url: String,
    username: String,
    password: String,
) -> Result<sync::ApiResult<LoginResponse>, String> {
    let url = format!("{}/api/v1/auth/login", server_url.trim_end_matches('/'));
    let result = infra::post_api_result::<LoginResponse>(
        &url,
        serde_json::json!({ "username": username, "password": password }),
    )
    .await;

    if !result.is_success() {
        return Ok(result);
    }

    let login_data = match result.data {
        Some(d) => d,
        None => return Ok(sync::ApiResult::err(result.code, "登录返回数据为空".to_string())),
    };

    let app_data_dir = match app.path().app_data_dir() {
        Ok(d) => d,
        Err(e) => return Ok(sync::ApiResult::err(codes::INTERNAL_ERROR, format!("获取数据目录失败: {e}"))),
    };
    if let Err(e) = persist_auth(&app_data_dir, &server_url, &login_data) {
        return Ok(sync::ApiResult::err(codes::INTERNAL_ERROR, e));
    }

    Ok(sync::ApiResult::ok(login_data))
}

/// 用户注册
#[tauri::command]
pub async fn register(
    app: tauri::AppHandle,
    server_url: String,
    username: String,
    password: String,
    nickname: String,
    email: String,
) -> Result<sync::ApiResult<LoginResponse>, String> {
    let url = format!("{}/api/v1/auth/register", server_url.trim_end_matches('/'));
    let result = infra::post_api_result::<LoginResponse>(
        &url,
        serde_json::json!({
            "username": username,
            "password": password,
            "nickname": nickname,
            "email": email,
        }),
    )
    .await;

    if !result.is_success() {
        return Ok(result);
    }

    let login_data = match result.data {
        Some(d) => d,
        None => return Ok(sync::ApiResult::err(result.code, "注册返回数据为空".to_string())),
    };

    let app_data_dir = match app.path().app_data_dir() {
        Ok(d) => d,
        Err(e) => return Ok(sync::ApiResult::err(codes::INTERNAL_ERROR, format!("获取数据目录失败: {e}"))),
    };
    if let Err(e) = persist_auth(&app_data_dir, &server_url, &login_data) {
        return Ok(sync::ApiResult::err(codes::INTERNAL_ERROR, e));
    }

    Ok(sync::ApiResult::ok(login_data))
}

/// 登出
#[tauri::command]
pub fn logout(app: tauri::AppHandle) -> Result<sync::ApiResult<()>, String> {
    let app_data_dir = match app.path().app_data_dir() {
        Ok(d) => d,
        Err(e) => return Ok(sync::ApiResult::err(codes::INTERNAL_ERROR, format!("获取数据目录失败: {e}"))),
    };
    let mut config = load_config(&app_data_dir);
    config.user_id = String::new();
    // 登出是唯一需要真正抹掉令牌的路径：走 clear_auth（清钥匙串 + 落盘），
    // 不能用 save_config（它按约定把空令牌视为「不改动」）。
    match infra::clear_auth(&app_data_dir, &config) {
        Ok(_) => Ok(sync::ApiResult::ok(())),
        Err(e) => Ok(sync::ApiResult::err(codes::INTERNAL_ERROR, e)),
    }
}

/// 执行一次同步：收集本地全量数据 -> POST 到服务端 -> 合并返回数据。
/// 命令层只做参数解析 + 配置加载 + 委托核心层 `sync::run_sync`。
#[tauri::command]
pub async fn sync_data(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppDb>,
    server_url: String,
    team_id: String,
) -> Result<sync::ApiResult<String>, String> {
    let app_data_dir = match app.path().app_data_dir() {
        Ok(d) => d,
        Err(e) => return Ok(sync::ApiResult::err(codes::INTERNAL_ERROR, format!("获取数据目录失败: {e}"))),
    };
    let server_url = server_url.trim().trim_end_matches('/').to_string();
    let team_id = team_id.trim().to_string();
    let mut config = load_config(&app_data_dir);

    if config.auth_token.is_empty() {
        return Ok(sync::ApiResult::err(codes::UNAUTHORIZED, "请先登录后再同步".to_string()));
    }

    // 回写本次使用的 server_url/team_id 到持久化配置，确保后续拉取与 call_server_api 指向同一实例
    config.server_url = server_url.clone();
    config.team_id = team_id.clone();
    if let Err(e) = save_config(&app_data_dir, &config) {
        return Ok(sync::ApiResult::err(codes::INTERNAL_ERROR, e));
    }

    let result = sync::run_sync(&state.conn, &server_url, &mut config, DataMode::Online).await;
    // 同步成功后持久化新游标（run_sync 已把服务端 serverTime 写入内存 config）
    if result.is_success() {
        if let Err(e) = save_config(&app_data_dir, &config) {
            return Ok(sync::ApiResult::err(codes::INTERNAL_ERROR, format!("保存同步游标失败: {e}")));
        }
    }
    Ok(result)
}

/// 只从服务器拉取数据（不再向服务器上传本地数据）。
/// 在线模式下，接口 / 分类 / 环境变量的增删改已直接调用后端接口落库，
/// 因此同步只需把服务器最新数据合并到本地缓存即可。
/// 命令层只做参数解析 + 委托核心层 `sync::run_pull`。
#[tauri::command]
pub async fn pull_data(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppDb>,
    server_url: String,
    team_id: String,
) -> Result<sync::ApiResult<String>, String> {
    let app_data_dir = match app.path().app_data_dir() {
        Ok(d) => d,
        Err(e) => return Ok(sync::ApiResult::err(codes::INTERNAL_ERROR, format!("获取数据目录失败: {e}"))),
    };
    let server_url = server_url.trim().trim_end_matches('/').to_string();
    let team_id = team_id.trim().to_string();
    let mut config = load_config(&app_data_dir);

    if config.auth_token.is_empty() {
        return Ok(sync::ApiResult::err(codes::UNAUTHORIZED, "请先登录后再同步".to_string()));
    }

    config.server_url = server_url.clone();
    config.team_id = team_id.clone();
    if let Err(e) = save_config(&app_data_dir, &config) {
        return Ok(sync::ApiResult::err(codes::INTERNAL_ERROR, e));
    }

    let result = sync::run_pull(&state.conn, &server_url, &mut config, DataMode::Online).await;
    // 拉取成功后持久化新游标
    if result.is_success() {
        if let Err(e) = save_config(&app_data_dir, &config) {
            return Ok(sync::ApiResult::err(codes::INTERNAL_ERROR, format!("保存同步游标失败: {e}")));
        }
    }
    Ok(result)
}

/// 检测服务端连通性
#[tauri::command]
pub async fn check_server_connection(server_url: String) -> Result<sync::ApiResult<bool>, String> {
    let url = format!("{}/api/v1/ping", server_url.trim_end_matches('/'));
    // 探测使用更短超时（5s），不阻塞自动同步轮询
    let client = match infra::http_client_with_timeout(std::time::Duration::from_secs(5)) {
        Ok(c) => c,
        Err(e) => return Ok(sync::ApiResult::err(codes::INTERNAL_ERROR, format!("创建客户端失败: {e}"))),
    };

    // 探测专用轻量端点 /api/v1/ping，服务端直接返回 200 + {success:true}，不产生登录失败日志
    match client.get(&url).send().await {
        Ok(resp) => Ok(sync::ApiResult::ok(resp.status().is_success())),
        Err(_) => Ok(sync::ApiResult::ok(false)),
    }
}

/// 返回服务端 base url，供前端在线模式直连后端 HTTP 接口使用。
/// 前端 useServerApi 在每次请求前调用，从持久化配置读取最新 serverUrl。
#[tauri::command]
pub fn get_server_base_url(app: tauri::AppHandle) -> Result<sync::ApiResult<String>, String> {
    let app_data_dir = app.path().app_data_dir().unwrap_or_default();
    Ok(sync::ApiResult::ok(load_config(&app_data_dir).server_url))
}

/// 通用服务端接口代理：前端把所有在线模式的后端 HTTP 调用都集中到这里，
/// 由 Rust 用 reqwest 转发，避免前端 WebView 直接跨域访问后端造成的 CORS 问题。
///
/// 参数：
/// - `path`：后端接口路径（如 `/api/v1/environment/groups/save`），不包含 base url。
/// - `payload`：业务参数对象（不含 token）。
///
/// 返回：统一包装的 `ApiResult<String>`，`data` 字段承载后端原始 JSON 字符串（前端自行二次解析）。
#[tauri::command]
pub async fn call_server_api(
    app: tauri::AppHandle,
    path: String,
    payload: serde_json::Value,
    // 前端在线模式传入的 token（来自内存中的登录态）。
    // 若提供则 Rust 端用内置公钥先验签（拿到 userId/teamId 供本地过滤），
    // 并透传给 Java 端（同时放入 body.token 与 Authorization 头）。
    // 若不提供则回退到本地持久化配置中的 auth_token（旧行为）。
    token: Option<String>,
) -> Result<sync::ApiResult<serde_json::Value>, String> {
    let app_data_dir = match app.path().app_data_dir() {
        Ok(d) => d,
        Err(e) => return Ok(sync::ApiResult::err(codes::INTERNAL_ERROR, format!("获取数据目录失败: {e}"))),
    };
    let config = load_config(&app_data_dir);

    // 决定实际使用的 token：优先前端传入，否则回退本地配置。
    let effective_token: String = match &token {
        Some(t) if !t.trim().is_empty() => t.trim().to_string(),
        _ => config.auth_token.clone(),
    };

    if effective_token.is_empty() {
        return Ok(sync::ApiResult::err(codes::UNAUTHORIZED, "请先登录后再操作".to_string()));
    }
    if config.server_url.is_empty() {
        return Ok(sync::ApiResult::err(codes::INTERNAL_ERROR, "服务端地址未配置".to_string()));
    }

    // —— 在线模式 JWT 验签（私钥在 Java 端、公钥在 Rust 端）——
    // 若前端传入的 token 是合法 JWT（RS256），Rust 验签后可拿到 userId/teamId，
    // 可用于本地 SQLite 按归属过滤；验签失败直接拒绝，阻断伪造 token 打到后端。
    // 若为旧随机 token（非 JWT 形态），则 Passthrough 交给 Java 端校验。
    let mut verified_user_id: Option<String> = None;
    let mut verified_team_id: Option<String> = None;
    if token.is_some() {
        match jwt::verify_token(&effective_token) {
            jwt::TokenVerification::Jwt(claims) => {
                verified_user_id = Some(claims.sub.clone());
                verified_team_id = Some(claims.team_id.clone());
                // 诊断日志：在线模式下 Rust 已成功验签，后续本地查询可基于该 userId 过滤。
                log::info!(
                    "JWT 验签通过: user_id={}, team_id={}",
                    claims.sub,
                    claims.team_id
                );
            }
            jwt::TokenVerification::Passthrough => {
                // 旧随机 token：不在此验签，交由 Java 端从 body.token 校验。
            }
            jwt::TokenVerification::Invalid(reason) => {
                return Ok(sync::ApiResult::err(
                    401,
                    format!("token 验签失败，请重新登录: {reason}"),
                ));
            }
        }
    }

    let base = config.server_url.trim().trim_end_matches('/');

    // M3: 限制 path 必须为以 `/api/v1/` 开头的合法相对路径，禁止 `//`、`/../`、
    // 绝对 URL 或携带协议前缀，避免通过 path 拼接实现 SSRF / 任意跳转。
    let path = path.trim();
    if !path.starts_with("/api/v1/") {
        return Ok(sync::ApiResult::err(codes::PARAM_INVALID, "非法接口路径：必须以 /api/v1/ 开头".to_string()));
    }
    if path.contains("//") || path.contains("../") || path.contains('\\') || path.contains(':') {
        return Ok(sync::ApiResult::err(codes::PARAM_INVALID, "非法接口路径：包含非法字符".to_string()));
    }
    let url = format!("{}{}", base, path);

    // 后端鉴权从请求体 body 的 token 字段提取（见 ApiHandler.authenticate），
    // 同时新增 Authorization: Bearer 头（供 Java 端用公钥验签 JWT）。双通道兼容。
    let mut body = if payload.is_object() {
        payload.as_object().cloned().unwrap_or_default()
    } else {
        serde_json::Map::new()
    };
    body.insert("mode".to_string(), serde_json::json!("online"));
    body.insert("token".to_string(), serde_json::json!(effective_token.clone()));

    // 若 Rust 已验签成功，把归属信息一并传给 Java 端，便于其按用户/团队鉴权。
    if let Some(uid) = &verified_user_id {
        body.insert("x_user_id".to_string(), serde_json::json!(uid.clone()));
    }
    if let Some(tid) = &verified_team_id {
        body.insert("x_team_id".to_string(), serde_json::json!(tid.clone()));
    }

    let client = match infra::http_client() {
        Ok(c) => c,
        Err(e) => return Ok(sync::ApiResult::err(codes::INTERNAL_ERROR, format!("创建客户端失败: {e}"))),
    };

    let resp = match client
        .post(&url)
        .header("Content-Type", "application/json; charset=utf-8")
        .header("Authorization", format!("Bearer {}", effective_token))
        .json(&serde_json::Value::Object(body))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => return Ok(sync::ApiResult::err(codes::DOWNSTREAM_ERROR, format!("请求服务端失败[{url}]: {e}"))),
    };

    let status = resp.status();
    let body_text = match resp.text().await {
        Ok(t) => t,
        Err(e) => return Ok(sync::ApiResult::err(codes::DOWNSTREAM_ERROR, format!("读取响应失败: {e}"))),
    };

    if !status.is_success() {
        // 带诊断信息，便于定位 401：token 是否已配置、目标 host。
        let host = url
            .split('/')
            .nth(2)
            .unwrap_or("未知")
            .to_string();
        return Ok(sync::ApiResult::err(
            status.as_u16() as i32,
            format!(
                "服务端返回错误{}: {} [诊断: host={}, token_len={}, token_非空={}]",
                status.as_u16(),
                body_text,
                host,
                effective_token.len(),
                !effective_token.is_empty()
            ),
        ));
    }

    // 后端统一响应结构为 `{ success, code, msg, data }`（HTTP 层成功时状态码为 2xx，
    // 业务成败由 body.success 决定）。Rust ApiResult.data 直接透传后端的业务 data，
    // 避免前端再手动解析一层嵌套 JSON（如 /api/v1/auth/me 返回的 data 内又包了一层）。
    let backend: serde_json::Value = match serde_json::from_str::<serde_json::Value>(&body_text) {
        Ok(v) => v,
        Err(e) => {
            // 非 JSON（理论不该发生）：原样透传文本，便于排查。
            return Ok(sync::ApiResult::err(codes::DOWNSTREAM_ERROR, format!("解析服务端响应失败: {e} | 原文: {body_text}")));
        }
    };

    let backend_success = backend.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
    let backend_code = backend.get("code").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    let backend_msg = backend
        .get("msg")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let backend_data = backend.get("data").cloned().unwrap_or(serde_json::Value::Null);

    if backend_success && backend_code == 0 {
        Ok(sync::ApiResult::ok(backend_data))
    } else {
        Ok(sync::ApiResult::err(
            backend_code,
            if backend_msg.is_empty() {
                format!("服务端业务失败 (code={backend_code})")
            } else {
                backend_msg
            },
        ))
    }
}


