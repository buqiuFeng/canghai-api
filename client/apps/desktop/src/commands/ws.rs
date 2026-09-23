use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};
use crate::sync::ApiResult;
use crate::sync::codes;
use tokio::net::TcpStream;
use tokio_tungstenite::{
    connect_async,
    tungstenite::client::IntoClientRequest,
    tungstenite::Message,
    MaybeTlsStream,
    WebSocketStream,
};

type WsWrite = tokio::sync::Mutex<
    futures_util::stream::SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
>;

/// 一条 WebSocket 连接的可写半边（共享引用便于发送命令复用）
pub struct WsConnection {
    write: Arc<WsWrite>,
}

/// 全局连接表：session id -> 连接
pub struct WsManager(pub Mutex<HashMap<String, WsConnection>>);

impl Default for WsManager {
    fn default() -> Self {
        WsManager(Mutex::new(HashMap::new()))
    }
}

/// 推送给前端的消息事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsMessageEvent {
    pub session_id: String,
    pub direction: String, // "recv" | "sent" | "system"
    pub text: String,
    pub time_ms: u64,
}

#[derive(Debug, Deserialize)]
pub struct WsConnectArgs {
    pub url: String,
    pub headers: Option<Vec<(String, String)>>,
}

/// 建立 WebSocket 连接，返回 session id
/// 后台任务持续读取消息并通过 `ws-message` 事件推送给前端
#[tauri::command]
pub async fn ws_connect(app: AppHandle, args: WsConnectArgs) -> ApiResult<String> {
    let mut request = match args
        .url
        .into_client_request()
    {
        Ok(r) => r,
        Err(e) => return ApiResult::err(-1, format!("非法的WebSocket 地址: {e}")),
    };
    if let Some(headers) = &args.headers {
        for (k, v) in headers {
            let name = match reqwest::header::HeaderName::from_bytes(k.as_bytes()) {
                Ok(n) => n,
                Err(e) => return ApiResult::err(codes::PARAM_INVALID, e.to_string()),
            };
            let val = match reqwest::header::HeaderValue::from_str(v) {
                Ok(val) => val,
                Err(e) => return ApiResult::err(codes::PARAM_INVALID, e.to_string()),
            };
            request.headers_mut().append(name, val);
        }
    }

    let (ws_stream, _resp) = match connect_async(request).await {
        Ok(pair) => pair,
        Err(e) => return ApiResult::err(-1, format!("WebSocket 连接失败: {e}")),
    };

    let (write, mut read) = ws_stream.split();
    let write = Arc::new(tokio::sync::Mutex::new(write));

    let session_id = format!("ws-{}-{}", now_ms(), rand_id());

    {
        let manager = app.state::<WsManager>();
        manager.0.lock().unwrap().insert(
            session_id.clone(),
            WsConnection { write: write.clone() },
        );
    }

    // 后台读取任务
    let app2 = app.clone();
    let sid = session_id.clone();
    let write_for_close = write.clone();
    tauri::async_runtime::spawn(async move {
        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(t)) => {
                    emit_msg(&app2, &sid, "recv", &t.to_string());
                }
                Ok(Message::Binary(b)) => {
                    emit_msg(&app2, &sid, "recv", &format!("[binary {} bytes]", b.len()));
                }
                Ok(Message::Ping(_)) | Ok(Message::Pong(_)) => { /* keep-alive，忽略 */ }
                Ok(Message::Close(_)) => {
                    emit_msg(&app2, &sid, "system", "连接已关闭");
                    break;
                }
                Ok(Message::Frame(_)) => { /* 底层帧，忽略 */ }
                Err(e) => {
                    emit_msg(&app2, &sid, "system", &format!("读取错误: {e}"));
                    break;
                }
            }
        }
        // 清理连接
        let manager = app2.state::<WsManager>();
        manager.0.lock().unwrap().remove(&sid);
        // 静默关闭写端
        let _ = write_for_close.lock().await.send(Message::Close(None)).await;
    });

    emit_msg(&app, &session_id, "system", "连接已建立");
    ApiResult::ok(session_id)
}

/// 发送文本消息
#[tauri::command]
pub async fn ws_send(app: AppHandle, session_id: String, message: String) -> ApiResult<()> {
    let write = {
        let manager = app.state::<WsManager>();
        let guard = manager.0.lock().unwrap();
        match guard.get(&session_id) {
            Some(c) => c.write.clone(),
            None => return ApiResult::err(-1, "连接不存在或已断开".to_string()),
        }
    };
    let mut w = write.lock().await;
    if let Err(e) = w.send(Message::Text(message.clone().into())).await {
        return ApiResult::err(-1, format!("发送失败: {e}"));
    }
    if let Err(e) = w.flush().await {
        return ApiResult::err(-1, format!("发送刷新失败: {e}"));
    }
    drop(w);
    emit_msg(&app, &session_id, "sent", &message);
    ApiResult::ok(())
}

/// 断开连接
#[tauri::command]
pub async fn ws_disconnect(app: AppHandle, session_id: String) -> ApiResult<()> {
    let write = {
        let manager = app.state::<WsManager>();
        let mut guard = manager.0.lock().unwrap();
        guard.remove(&session_id).map(|c| c.write.clone())
    };
    if let Some(w) = write {
        let mut w = w.lock().await;
        let _ = w.send(Message::Close(None)).await;
    }
    emit_msg(&app, &session_id, "system", "已主动断开连接");
    ApiResult::ok(())
}

fn emit_msg(app: &AppHandle, session_id: &str, direction: &str, text: &str) {
    let _ = app.emit(
        "ws-message",
        WsMessageEvent {
            session_id: session_id.to_string(),
            direction: direction.to_string(),
            text: text.to_string(),
            time_ms: now_ms(),
        },
    );
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 生成进程内唯一的会话随机段。
/// 说明：原实现以 now_ms() 作 DefaultHasher 种子，同一毫秒内并发连接会得到相同的 id，
/// 造成 session 碰撞。改用「单调原子计数器 + 纳秒时间」组合，保证唯一性。
fn rand_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    format!("{nanos:x}{seq:x}")
}
