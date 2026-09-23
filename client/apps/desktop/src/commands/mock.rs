use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::sync::ApiResult;
use crate::sync::codes;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

/// 单条 Mock 路由规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockRoute {
    pub path: String,           // 精确路径，如 /api/user，或 /api/user/* 前缀匹配
    pub method: String,         // GET/POST/...
    pub status: u16,            // 返回状态码
    pub body: String,           // 响应体
    pub content_type: String,   // Content-Type
    pub headers: Option<Vec<(String, String)>>,
    pub delay_ms: Option<u64>,  // 模拟延迟
    pub sse: Option<bool>,      // 是否 SSE 持续推送
}

/// Mock 服务运行状态
pub struct MockServerState {
    running: AtomicBool,
    handle: Mutex<Option<thread::JoinHandle<()>>>,
}

impl Default for MockServerState {
    fn default() -> Self {
        MockServerState {
            running: AtomicBool::new(false),
            handle: Mutex::new(None),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockLogEntry {
    pub time_ms: u64,
    pub method: String,
    pub path: String,
    pub matched: bool,
    pub status: u16,
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 启动 Mock 服务
#[tauri::command]
pub async fn start_mock_server(
    app: AppHandle,
    routes: Vec<MockRoute>,
    port: u16,
) -> ApiResult<()> {
    let state = app.state::<MockServerState>();
    if state.running.load(Ordering::SeqCst) {
        return ApiResult::err(codes::CONFLICT, "Mock 服务已在运行".to_string());
    }
    let routes = Arc::new(routes);
    let app_for_log = app.clone();
    let prt = if port == 0 { 8787 } else { port };

    // 在主线程同步绑定端口，失败可立即返回（原实现在子线程绑定，错误无法回传）
    let server = match tiny_http::Server::http(format!("127.0.0.1:{prt}")) {
        Ok(s) => s,
        Err(e) => return ApiResult::err(codes::INTERNAL_ERROR, format!("启动 Mock 服务失败: {e}")),
    };

    state.running.store(true, Ordering::SeqCst);
    let handle = thread::spawn(move || {
        let st = app_for_log.state::<MockServerState>();
        // 用 recv_timeout 轮询：空闲时也能感知 running=false 并及时退出
        // （原实现阻塞在 incoming_requests 上，无请求时无法停止）
        loop {
            if !st.running.load(Ordering::SeqCst) {
                break;
            }
            let request = match server.recv_timeout(std::time::Duration::from_millis(200)) {
                Ok(Some(r)) => r,
                Ok(None) => continue,
                Err(_) => break,
            };
            let method = request.method().as_str().to_string();
            let url = request.url().to_string();
            let matched = match_route(&routes, &method, &url);

            let log = match &matched {
                Some(r) => MockLogEntry {
                    time_ms: now_ms(),
                    method: method.clone(),
                    path: url.clone(),
                    matched: true,
                    status: r.status,
                },
                None => MockLogEntry {
                    time_ms: now_ms(),
                    method: method.clone(),
                    path: url.clone(),
                    matched: false,
                    status: 404,
                },
            };
            let _ = app_for_log.emit("mock-log", &log);

            match matched {
                Some(r) => {
                    if let Some(d) = r.delay_ms {
                        thread::sleep(std::time::Duration::from_millis(d));
                    }
                    if r.sse.unwrap_or(false) {
                        // SSE：返回事件流（简单回推一次心跳 + body）
                        let ct = "text/event-stream".to_string();
                        let headers = build_headers(&ct, &r.headers);
                        let resp = tiny_http::Response::new(
                            tiny_http::StatusCode(200),
                            headers,
                            std::io::Cursor::new(format!("data: {}\n\n", r.body)),
                            Some(r.body.len() as usize),
                            None,
                        );
                        let _ = request.respond(resp);
                    } else {
                        let body = r.body.clone();
                        let ct = if r.content_type.is_empty() {
                            "application/json".to_string()
                        } else {
                            r.content_type.clone()
                        };
                        let headers = build_headers(&ct, &r.headers);
                        let resp = tiny_http::Response::new(
                            tiny_http::StatusCode(r.status),
                            headers,
                            std::io::Cursor::new(body),
                            Some(r.body.len() as usize),
                            None,
                        );
                        let _ = request.respond(resp);
                    }
                }
                None => {
                    let resp = tiny_http::Response::from_string("{\"error\":\"mock route not found\"}")
                        .with_status_code(404)
                        .with_header(
                            tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap(),
                        );
                    let _ = request.respond(resp);
                }
            }
        }
        st.running.store(false, Ordering::SeqCst);
    });

    *state.handle.lock().unwrap() = Some(handle);
    ApiResult::ok(())
}

/// 停止 Mock 服务
#[tauri::command]
pub async fn stop_mock_server(app: AppHandle) -> ApiResult<()> {
    let state = app.state::<MockServerState>();
    state.running.store(false, Ordering::SeqCst);
    // 取出并 join 后台线程，确保监听端口被释放
    // （原实现仅置标志、不 join → 无请求时线程阻塞在 accept 上无法退出、端口持续占用）
    let handle = state.handle.lock().unwrap().take();
    if let Some(h) = handle {
        let _ = tokio::task::spawn_blocking(move || h.join()).await;
    }
    ApiResult::ok(())
}

#[tauri::command]
pub async fn mock_server_status(app: AppHandle) -> ApiResult<bool> {
    let state = app.state::<MockServerState>();
    ApiResult::ok(state.running.load(Ordering::SeqCst))
}

fn match_route(routes: &[MockRoute], method: &str, url: &str) -> Option<MockRoute> {
    let path_only = url.split('?').next().unwrap_or(url);
    for r in routes {
        if !r.method.eq_ignore_ascii_case(method) {
            continue;
        }
        if r.path == path_only {
            return Some(r.clone());
        }
        // 前缀匹配：/api/user/* 匹配 /api/user/123
        if r.path.ends_with("/*") {
            let prefix = &r.path[..r.path.len() - 1];
            if path_only.starts_with(prefix) {
                return Some(r.clone());
            }
        }
    }
    None
}

fn build_headers(ct: &str, extra: &Option<Vec<(String, String)>>) -> Vec<tiny_http::Header> {
    let mut headers = vec![
        tiny_http::Header::from_bytes(&b"Content-Type"[..], ct.as_bytes()).unwrap(),
    ];
    if let Some(extra) = extra {
        for (k, v) in extra {
            if let Ok(h) = tiny_http::Header::from_bytes(k.as_bytes(), v.as_bytes()) {
                headers.push(h);
            }
        }
    }
    headers
}
