use crate::infra;
use crate::models::{HttpRequest, HttpResponse};
use crate::sync::ApiResult;
use crate::sync::codes;
use futures_util::StreamExt;
use serde::Serialize;
use std::net::{IpAddr, Ipv4Addr};
use std::str::FromStr;
use std::time::Instant;
use tauri::{Emitter, Window};

/// 允许访问的 URL 协议白名单（防止 file:// / gopher:// 等非常规协议引发的 SSRF/本地文件泄露）。
const ALLOWED_URL_SCHEMES: [&str; 2] = ["http", "https"];

/// 流式响应的 IPC 合批间隔（毫秒）：普通分块文本按此间隔合并后再推送（Phase 7.8）。
const STREAM_FLUSH_INTERVAL_MS: u128 = 50;
/// 流式响应的 IPC 合批体积上限：达到即立刻推送，避免大响应长时间滞留内存。
const STREAM_FLUSH_BYTES: usize = 64 * 1024;

/// SSRF / 协议防护：校验请求 URL 是否符合白名单，并拦截指向私有/环回/链路本地地址的请求。
///
/// 修复 C3：原实现直接拿客户端传入的 `req.url` 发起请求，可被诱导访问内网资源
///（如 http://169.254.169.254 云元数据、http://127.0.0.1:6379 等），造成服务端侧 SSRF。
///
/// `allow_private` 为 true 时跳过内网地址拦截（API 调试工具常需访问内网服务），
/// 但云元数据地址 169.254.169.254 始终拦截，避免开启开关后泄露云厂商凭证。
fn assert_safe_target_url(url: &str, allow_private: bool) -> Result<(), String> {
    let parsed = url::Url::parse(url).map_err(|e| format!("非法 URL: {e}"))?;

    // 1) 协议白名单
    if !ALLOWED_URL_SCHEMES.contains(&parsed.scheme()) {
        return Err(format!(
            "不支持的协议 `{}`，仅允许 http/https",
            parsed.scheme()
        ));
    }

    // 2) 解析 host，拦截环回 / 私有 / 链路本地 / 未指定地址
    let host = parsed
        .host_str()
        .ok_or_else(|| "URL 缺少主机名".to_string())?;
    if let Ok(ip) = IpAddr::from_str(host) {
        // 云元数据地址始终拦截（无论是否允许内网），防止凭证泄露
        if ip == IpAddr::V4(Ipv4Addr::new(169, 254, 169, 254)) {
            return Err("禁止访问云元数据地址 169.254.169.254".to_string());
        }
        if !allow_private && is_private_or_reserved_ip(&ip) {
            return Err(format!("禁止访问保留/内网地址 {ip}（可在设置中开启「允许内网地址」）"));
        }
    } else {
        // 域名：先尝试解析（以覆盖指向内网 IP 的域名），解析失败则放行（DNS 可能在运行时变化）。
        if let Ok(addrs) = std::net::ToSocketAddrs::to_socket_addrs(&(host, 0u16)) {
            for addr in addrs {
                let ip = addr.ip();
                if ip == IpAddr::V4(Ipv4Addr::new(169, 254, 169, 254)) {
                    return Err("禁止访问云元数据地址 169.254.169.254".to_string());
                }
                if !allow_private && is_private_or_reserved_ip(&ip) {
                    return Err(format!("禁止访问解析到内网地址的域名 {host}（可在设置中开启「允许内网地址」）"));
                }
            }
        }
    }

    Ok(())
}

/// 判断 IP 是否为环回 / 私有 / 链路本地 / 未指定 / 文档化保留地址。
fn is_private_or_reserved_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_unspecified()
                || v4.is_broadcast()
                // 文档化保留地址（如 192.0.2.0/24、198.51.100.0/24、203.0.113.0/24）
                || (v4.octets()[0] == 192 && v4.octets()[1] == 0 && v4.octets()[2] == 2)
                || (v4.octets()[0] == 198 && v4.octets()[1] == 51 && v4.octets()[2] == 100)
                || (v4.octets()[0] == 203 && v4.octets()[1] == 0 && v4.octets()[2] == 113)
        }
        IpAddr::V6(v6) => {
            v6.is_loopback()
                || v6.is_unspecified()
                || (v6.segments()[0] & 0xfe00) == 0xfc00 // 唯一本地地址 fc00::/7
                || (v6.segments()[0] & 0xffc0) == 0xfe80 // 链路本地 fe80::/10
        }
    }
}

/// 代理请求错误的业务码归类：
/// 目标校验类失败（SSRF 拦截 / 非法 URL / 非法协议）属参数问题，其余（连接 / DNS / TLS / 超时）属下游异常。
fn http_err_code(msg: &str) -> i32 {
    if msg.starts_with("禁止") || msg.starts_with("非法") || msg.starts_with("不支持") || msg.starts_with("URL 缺少") {
        codes::PARAM_INVALID
    } else {
        codes::DOWNSTREAM_ERROR
    }
}

/// 统一包装：内部实现沿用标准 Result（可用 ?），外层命令包装为 ApiResult
#[tauri::command]
pub async fn send_http_request(req: HttpRequest) -> ApiResult<HttpResponse> {
    match inner_send_http_request(req).await {
        Ok(v) => ApiResult::ok(v),
        Err(e) => ApiResult::err(http_err_code(&e), e),
    }
}

async fn inner_send_http_request(req: HttpRequest) -> Result<HttpResponse, String> {
    assert_safe_target_url(&req.url, req.allow_private)?;

    let method = reqwest::Method::from_bytes(req.method.to_uppercase().as_bytes())
        .map_err(|e| format!("非法 HTTP 方法: {}", e))?;

    // 复用进程级共享代理客户端：内部已设 UA、超时与「禁止自动跟随重定向」
    // （M2 SSRF 防护），避免每次请求重复建连 + TLS 握手。
    let client = infra::proxy_client()?;

    let mut builder = client.request(method, &req.url);

    for (k, v) in &req.headers {
        builder = builder.header(k.as_str(), v.as_str());
    }
    if let Some(body) = req.body {
        builder = builder.body(body);
    }
    let start = Instant::now();
    let resp = builder
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    let status = resp.status().as_u16();
    let status_text = resp
        .status()
        .canonical_reason()
        .unwrap_or("")
        .to_string();

    let mut headers: Vec<(String, String)> = Vec::with_capacity(resp.headers().len());
    for (k, v) in resp.headers().iter() {
        headers.push((k.to_string(), v.to_str().unwrap_or("").to_string()));
    }

    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("读取响应体失败: {}", e))?;
    let size = bytes.len() as u64;
    let body = String::from_utf8_lossy(&bytes).to_string();
    let time_ms = start.elapsed().as_millis() as u64;

    Ok(HttpResponse {
        status,
        status_text,
        headers,
        body,
        time_ms,
        size,
    })
}

/// 推送给前端的流式数据块
#[derive(Serialize, Clone)]
struct StreamChunk {
    request_id: String,
    /// 本块原始文本（已解析 SSE 后的 data 内容，按事件聚合）
    data: String,
    /// 该块是否为 SSE 格式（否则为普通分块文本）
    sse: bool,
    /// 是否为最后一个块
    done: bool,
}

/// 流式发送 HTTP 请求（用于 SSE / 大响应体逐步展示）
/// 通过 Tauri event `http-stream-chunk` 将分块增量推给前端，最后再返回完整 HttpResponse
#[tauri::command]
pub async fn send_http_request_stream(
    req: HttpRequest,
    window: Window,
) -> ApiResult<HttpResponse> {
    match inner_send_http_request_stream(req, window).await {
        Ok(v) => ApiResult::ok(v),
        Err(e) => ApiResult::err(http_err_code(&e), e),
    }
}

async fn inner_send_http_request_stream(
    req: HttpRequest,
    window: Window,
) -> Result<HttpResponse, String> {
    assert_safe_target_url(&req.url, req.allow_private)?;

    let request_id = req
        .headers
        .get("x-canghai-request-id")
        .cloned()
        .unwrap_or_else(|| {
            format!(
                "stream-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis())
                    .unwrap_or(0)
            )
        });

    let method = reqwest::Method::from_bytes(req.method.to_uppercase().as_bytes())
        .map_err(|e| format!("非法 HTTP 方法: {}", e))?;

    // 与普通请求共用同一份共享代理客户端（连接池复用）
    let client = infra::proxy_client()?;

    let mut builder = client.request(method, &req.url);
    for (k, v) in &req.headers {
        builder = builder.header(k.as_str(), v.as_str());
    }
    if let Some(body) = req.body {
        builder = builder.body(body);
    }

    let start = Instant::now();
    let resp = builder
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    let status = resp.status().as_u16();
    let status_text = resp
        .status()
        .canonical_reason()
        .unwrap_or("")
        .to_string();

    let mut headers: Vec<(String, String)> = Vec::with_capacity(resp.headers().len());
    for (k, v) in resp.headers().iter() {
        headers.push((k.to_string(), v.to_str().unwrap_or("").to_string()));
    }

    // 依据 Content-Type 判断是否 SSE
    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_lowercase();
    let is_sse = content_type.contains("text/event-stream");

    let mut stream = resp.bytes_stream();
    let mut full: Vec<u8> = Vec::new();
    // SSE 缓冲：跨块累积，按事件边界(\n\n)切分
    let mut sse_buf = String::new();
    let mut total_size: u64 = 0;

    // 非 SSE 大响应的 IPC 合批缓冲（Phase 7.8）
    let mut bulk_buf: Vec<u8> = Vec::new();
    let mut last_flush = Instant::now();

    let emit_chunk = |chunk: StreamChunk| {
        let _ = window.emit("http-stream-chunk", chunk);
    };

    while let Some(item) = stream.next().await {
        let bytes = match item {
            Ok(b) => b,
            Err(e) => {
                emit_chunk(StreamChunk {
                    request_id: request_id.clone(),
                    data: format!("\n[流读取错误] {e}"),
                    sse: is_sse,
                    done: true,
                });
                break;
            }
        };
        total_size += bytes.len() as u64;
        full.extend_from_slice(&bytes);

        if is_sse {
            // SSE: 把新增字节追加到缓冲，逐事件解析
            sse_buf.push_str(&String::from_utf8_lossy(&bytes));
            // 合批：把**同一次网络读取**中解析出的多个事件合并为一次 IPC 推送。
            // 直接首尾拼接（不插入分隔符），保证前端累积文本与逐事件推送时完全一致；
            // 事件以数据驱动、无缓冲区滞留，因此不引入实时性延迟。
            let mut coalesced = String::new();
            while let Some(pos) = sse_buf.find("\n\n") {
                let event_block = sse_buf[..pos].to_string();
                sse_buf.replace_range(..pos + 2, "");
                // 提取 data: 行（允许多行 data）
                let mut data_lines: Vec<&str> = Vec::new();
                for line in event_block.lines() {
                    if let Some(rest) = line.strip_prefix("data:") {
                        data_lines.push(rest.strip_prefix(' ').unwrap_or(rest));
                    }
                }
                if !data_lines.is_empty() {
                    coalesced.push_str(&data_lines.join("\n"));
                }
            }
            if !coalesced.is_empty() {
                emit_chunk(StreamChunk {
                    request_id: request_id.clone(),
                    data: coalesced,
                    sse: true,
                    done: false,
                });
            }
        } else {
            // 普通分块文本：按「50ms 间隔」或「64KB 体积」合批后再推送，
            // 取代原先「每收到一个网络分块就发一次 IPC」造成的事件洪泛。
            bulk_buf.extend_from_slice(&bytes);
            if bulk_buf.len() >= STREAM_FLUSH_BYTES
                || last_flush.elapsed().as_millis() >= STREAM_FLUSH_INTERVAL_MS
            {
                let data = String::from_utf8_lossy(&bulk_buf).to_string();
                bulk_buf.clear();
                last_flush = Instant::now();
                emit_chunk(StreamChunk {
                    request_id: request_id.clone(),
                    data,
                    sse: false,
                    done: false,
                });
            }
        }
    }

    // 收尾：冲刷非 SSE 合批缓冲中尚未推送的数据
    if !bulk_buf.is_empty() {
        emit_chunk(StreamChunk {
            request_id: request_id.clone(),
            data: String::from_utf8_lossy(&bulk_buf).to_string(),
            sse: false,
            done: false,
        });
        bulk_buf.clear();
    }

    // 收尾：SSE 缓冲中残留的最后一个事件（可能无结尾空行）
    if is_sse && !sse_buf.trim().is_empty() {
        let mut data_lines: Vec<&str> = Vec::new();
        for line in sse_buf.lines() {
            if let Some(rest) = line.strip_prefix("data:") {
                data_lines.push(rest.strip_prefix(' ').unwrap_or(rest));
            }
        }
        if !data_lines.is_empty() {
            emit_chunk(StreamChunk {
                request_id: request_id.clone(),
                data: data_lines.join("\n"),
                sse: true,
                done: false,
            });
        }
    }

    emit_chunk(StreamChunk {
        request_id: request_id.clone(),
        data: String::new(),
        sse: is_sse,
        done: true,
    });

    let body = String::from_utf8_lossy(&full).to_string();
    let time_ms = start.elapsed().as_millis() as u64;

    Ok(HttpResponse {
        status,
        status_text,
        headers,
        body,
        time_ms,
        size: total_size,
    })
}
