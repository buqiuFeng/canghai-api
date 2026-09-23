//! 基础设施层：配置读写 + 统一 HTTP 客户端 + 统一响应解析。
//!
//! 把原先散落在 `commands/mod.rs`（`load_sync_config`/`server_get`）与
//! `commands/sync.rs`（`load_config`/`save_config`/各处 `reqwest::Client::builder()`）
//! 的重复实现收敛为单一事实来源，避免超时值/配置路径/解析逻辑分叉。

use crate::commands::sync::SyncConfig;
use crate::sync::ApiResult;
use crate::sync::codes;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;

/// 统一的 HTTP 请求超时（秒）。原先各地方 5/10/15/30s 不一致，集中管理。
pub const HTTP_TIMEOUT: Duration = Duration::from_secs(30);

/// 进程级共享客户端（懒初始化）。
///
/// `reqwest::Client` 内部持有连接池，克隆只是复制 Arc，因此全局复用一份即可：
/// 原先每次调用都 `Client::builder().build()`，导致每个请求都要重新建连与 TLS 握手
/// （实测单次额外 100~300ms）。调试工具属于高频请求场景，这里复用连接池。
static SHARED_CLIENT: OnceLock<Option<reqwest::Client>> = OnceLock::new();

/// 同步配置文件名（单一命名常量，避免各处硬编码 "sync_config.json"）。
const CONFIG_FILE: &str = "sync_config.json";

/// 令牌在钥匙串中的 service 名（应用标识）。
const TOKEN_SERVICE: &str = "canghai-api-doc";
/// 令牌在钥匙串中的 account 名。
const TOKEN_ACCOUNT: &str = "auth_token";

/// 令牌安全存储：写入系统钥匙串，并**回读校验**是否真正落地。
///
/// 返回 false 表示当前平台不可用（如 Linux 无 Secret Service），
/// **或写入未真正生效**——部分 Windows 环境下 `CredWrite` 返回成功、但条目随后
/// 不可读（凭据管理器里查不到），旧实现只信 `set_password().is_ok()` 便清空了
/// 配置文件中的明文兜底，导致「登录成功 → 刷新即未登录」。
///
/// 因此这里以回读结果为准：只有写入后能原样读回才视为成功，否则由调用方
/// 回退到配置文件明文存储，保证功能可用而非直接丢失登录态。
pub fn save_token(token: &str) -> bool {
    let entry = match keyring::Entry::new(TOKEN_SERVICE, TOKEN_ACCOUNT) {
        Ok(e) => e,
        Err(_) => return false,
    };
    if entry.set_password(token).is_err() {
        return false;
    }
    match entry.get_password() {
        Ok(v) if v == token => true,
        _ => {
            log::warn!("[infra] 钥匙串写入后回读校验失败，回退为配置文件明文存储");
            false
        }
    }
}

/// 从系统钥匙串读取令牌；不可用或未存过返回 None。
pub fn load_token() -> Option<String> {
    keyring::Entry::new(TOKEN_SERVICE, TOKEN_ACCOUNT)
        .ok()
        .and_then(|e| e.get_password().ok())
}

/// 登出时清除钥匙串中的令牌。
///
/// 仅限「显式登出」调用；普通配置写入请勿调用（见 [`save_config`] 的语义约定）。
pub fn delete_token() {
    if let Ok(entry) = keyring::Entry::new(TOKEN_SERVICE, TOKEN_ACCOUNT) {
        let _ = entry.delete_credential();
    }
}

/// 读取当前生效的令牌：钥匙串优先，其次配置文件（钥匙串不可用时的明文回退）。
fn existing_token(app_data_dir: &PathBuf) -> String {
    if let Some(t) = load_token() {
        if !t.is_empty() {
            return t;
        }
    }
    let path = config_path(app_data_dir);
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|c| serde_json::from_str::<SyncConfig>(&c).ok())
        .map(|c| c.auth_token)
        .unwrap_or_default()
}

/// 配置落盘路径，单一实现。
pub fn config_path(app_data_dir: &PathBuf) -> PathBuf {
    app_data_dir.join(CONFIG_FILE)
}

/// 读取同步配置。文件缺失或解析失败时返回默认（未登录）配置。
/// 取代 `commands/mod.rs::load_sync_config` 与旧 `commands/sync.rs::load_config` 的双实现。
pub fn load_config(app_data_dir: &PathBuf) -> SyncConfig {
    let path = config_path(app_data_dir);
    let mut config = if path.exists() {
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|c| serde_json::from_str::<SyncConfig>(&c).ok())
            .unwrap_or_default()
    } else {
        SyncConfig::default()
    };
    // 令牌已从配置文件迁移到系统钥匙串：落盘字段为空时回源到钥匙串
    if config.auth_token.is_empty() {
        config.auth_token = load_token().unwrap_or_default();
        // 诊断：两处都取不到令牌但存在 userId，说明本机登录态已丢失（例如钥匙串条目
        // 被清理或写入未真正落地），此时前端会表现为「刷新后变成未登录」。
        if config.auth_token.is_empty() && !config.user_id.is_empty() {
            log::warn!("[infra] 配置文件与系统钥匙串中均无令牌（userId 非空），本机登录态已丢失，请重新登录");
        }
    }
    config
}

/// 持久化同步配置（server_url / team_id / user_id 等）。
///
/// 令牌不落盘：优先写入系统钥匙串，成功则把落盘 JSON 中的 auth_token 置空；
/// 钥匙串不可用时才回退明文落盘，保证「安全优先、可用兜底」。
///
/// ⚠️ 令牌语义约定（修复「登录后刷新变成未登录」）：
/// - 传入**非空** token → 更新钥匙串；
/// - 传入**空** token → **保持既有令牌不变**（钥匙串优先，其次旧文件里的明文），
///   不再删除。
///
/// 旧实现在空 token 时直接 `delete_token()`，而前端有多处 `set_sync_config`
/// 调用并不携带令牌（保存全局设置、保存同步设置等），结果「保存一次设置」
/// 就会把登录态抹掉，表现为刷新页面即为未登录。
/// 需要真正清除令牌时，请走「先 [`delete_token`]，再 `save_config`（令牌为空）」
/// 的显式路径（见 `commands/sync.rs::logout`）。
pub fn save_config(app_data_dir: &PathBuf, config: &SyncConfig) -> Result<(), String> {
    let mut persist = config.clone();
    if persist.auth_token.is_empty() {
        // 空 = 不改动：回填既有令牌后原样写回，避免误删登录态
        persist.auth_token = existing_token(app_data_dir);
    }
    if !persist.auth_token.is_empty() && save_token(&persist.auth_token) {
        persist.auth_token.clear();
    }

    let path = config_path(app_data_dir);
    let json = serde_json::to_string_pretty(&persist)
        .map_err(|e| format!("序列化配置失败: {e}"))?;
    std::fs::write(&path, json).map_err(|e| format!("写入配置失败: {e}"))
}

/// 显式清除认证态（仅供登出使用）。
///
/// 与 [`save_config`] 的区别：save_config 把「空令牌」视为「不改动」，因此
/// 登出这条真正要抹掉令牌的路径必须走这里 —— 同时清除钥匙串条目与落盘文件中的令牌，
/// 不受「回填既有令牌」逻辑影响（钥匙串不可用、令牌以明文回退落盘时同样能清干净）。
pub fn clear_auth(app_data_dir: &PathBuf, config: &SyncConfig) -> Result<(), String> {
    delete_token();
    let mut persist = config.clone();
    persist.auth_token = String::new();
    let path = config_path(app_data_dir);
    let json = serde_json::to_string_pretty(&persist)
        .map_err(|e| format!("序列化配置失败: {e}"))?;
    std::fs::write(&path, json).map_err(|e| format!("写入配置失败: {e}"))
}

/// 构建统一 HTTP 客户端（复用共享连接池）。所有服务端调用（login/register/sync/pull）
/// 与 HTTP 调试代理共用，避免重复 builder 与重复建连。
pub fn http_client() -> Result<reqwest::Client, String> {
    let cached = SHARED_CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(HTTP_TIMEOUT)
            .build()
            .ok()
    });
    match cached {
        Some(c) => Ok(c.clone()),
        None => Err("创建 HTTP 客户端失败".to_string()),
    }
}

/// 指定超时的 HTTP 客户端变体（用于连通性探测等需要更短超时的场景）。
pub fn http_client_with_timeout(timeout: Duration) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(timeout)
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {e}"))
}

/// 调试代理专用客户端（懒初始化，全进程复用连接池）。
///
/// 与 `http_client()` 分开的原因：代理请求必须 **禁用自动重定向**
/// （防 SSRF 二次利用：重定向到内网/不可信地址），而服务端调用沿用默认重定向策略。
/// 原先 `http.rs` 每次请求都 `Client::builder().build()`，导致每个调试请求都要
/// 重新建连 + TLS 握手（实测额外 100~300ms），此处改为共享单例。
static PROXY_CLIENT: OnceLock<Option<reqwest::Client>> = OnceLock::new();

pub fn proxy_client() -> Result<reqwest::Client, String> {
    let cached = PROXY_CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .user_agent("CanghaiApiDebugger/0.1")
            .timeout(HTTP_TIMEOUT)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .ok()
    });
    match cached {
        Some(c) => Ok(c.clone()),
        None => Err("初始化 HTTP 客户端失败".to_string()),
    }
}

/// 通用「POST + 解析 ApiResult」辅助，消除 login/register/sync/pull 中
/// 重复的 body 构造 + JSON 解析 + success 判断样板。
/// 返回统一的 `ApiResult<T>`：传输层/解析层错误也包装为 `err`，调用方可直接透传。
pub async fn post_api_result<T: serde::de::DeserializeOwned>(
    url: &str,
    body: serde_json::Value,
) -> ApiResult<T> {
    // 埋点：记录端到端耗时与结果，供 Phase 7 基线对比（仅元信息，不含请求体）
    let started = std::time::Instant::now();
    let client = match http_client() {
        Ok(c) => c,
        Err(e) => return ApiResult::err(codes::INTERNAL_ERROR, format!("创建客户端失败: {e}")),
    };
    let resp = match client.post(url).json(&body).send().await {
        Ok(r) => r,
        Err(e) => {
            log::warn!("HTTP POST {} 失败 cost={}ms err={}", url, started.elapsed().as_millis(), e);
            return ApiResult::err(codes::DOWNSTREAM_ERROR, format!("请求失败[{url}]: {e}"));
        }
    };
    let status = resp.status();
    let text = match resp.text().await {
        Ok(t) => t,
        Err(e) => {
            log::warn!("HTTP POST {} 读取响应失败 cost={}ms err={}", url, started.elapsed().as_millis(), e);
            return ApiResult::err(codes::DOWNSTREAM_ERROR, format!("读取响应失败: {e}"));
        }
    };
    // 必须校验 HTTP 状态码：否则 5xx/4xx 的 HTML 错误页会被当成 JSON 解析，
    // 表现成莫名其妙的「解析响应失败」而不是真实的服务端错误。
    if !status.is_success() {
        log::warn!(
            "HTTP POST {} status={} cost={}ms",
            url,
            status.as_u16(),
            started.elapsed().as_millis()
        );
        return ApiResult::err(
            status.as_u16() as i32,
            format!("服务端返回错误 {}: {}", status.as_u16(), text),
        );
    }
    let result = match serde_json::from_str::<ApiResult<T>>(&text) {
        Ok(r) => r,
        Err(e) => ApiResult::err(codes::DOWNSTREAM_ERROR, format!("解析响应失败: {e}\n响应内容: {text}")),
    };
    log::info!(
        "HTTP POST {} status={} cost={}ms success={}",
        url,
        status.as_u16(),
        started.elapsed().as_millis(),
        result.is_success()
    );
    result
}

/// 读取持久化配置中的 server_url 与 auth_token；未登录返回 401。
/// 统一 team/project/project_member 各处重复的 get_server_url_and_token 实现。
pub fn get_server_url_and_token(app: &tauri::AppHandle) -> ApiResult<(String, String)> {
    use tauri::Manager;
    let app_data_dir = match app.path().app_data_dir() {
        Ok(d) => d,
        Err(e) => return ApiResult::err(codes::INTERNAL_ERROR, format!("获取数据目录失败: {e}")),
    };
    let cfg = load_config(&app_data_dir);
    if cfg.auth_token.is_empty() {
        return ApiResult::err(codes::UNAUTHORIZED, "请先登录".to_string());
    }
    ApiResult::ok((cfg.server_url.trim_end_matches('/').to_string(), cfg.auth_token))
}

/// 通用「GET + 解析 ApiResult」辅助（token 以 JSON body 传递，兼容现有后端契约）。
/// 与 `post_api_result` 对称，仅供后端确实声明为 GET 的端点使用。
///
/// 注意：后端绝大多数业务端点（含 `/api/v1/project-member/visible`）都声明为
/// `@PostMapping`，用本函数会因方法不匹配拿不到数据；新增调用前先确认控制器注解。
///
/// 当前无调用方（可见项目查询已改走 POST），保留作为与 `post_api_result` 对称的
/// 基础设施能力，待后端出现真正的 GET 端点时复用。
#[allow(dead_code)]
pub async fn get_api_result<T: serde::de::DeserializeOwned>(
    url: &str,
    token: &str,
) -> ApiResult<T> {
    let client = match http_client() {
        Ok(c) => c,
        Err(e) => return ApiResult::err(codes::INTERNAL_ERROR, format!("创建客户端失败: {e}")),
    };
    let resp = match client
        .get(url)
        .json(&serde_json::json!({ "token": token }))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => return ApiResult::err(codes::DOWNSTREAM_ERROR, format!("请求失败[{url}]: {e}")),
    };
    let status = resp.status();
    let text = match resp.text().await {
        Ok(t) => t,
        Err(e) => return ApiResult::err(codes::DOWNSTREAM_ERROR, format!("读取响应失败: {e}")),
    };
    if !status.is_success() {
        return ApiResult::err(
            status.as_u16() as i32,
            format!("服务端返回错误 {}: {}", status.as_u16(), text),
        );
    }
    match serde_json::from_str::<ApiResult<T>>(&text) {
        Ok(r) => r,
        Err(e) => ApiResult::err(codes::DOWNSTREAM_ERROR, format!("解析响应失败: {e}\n响应内容: {text}")),
    }
}
