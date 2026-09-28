use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpRequest {
    pub method: String,
    pub url: String,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(default)]
    pub body: Option<String>,
    /// 多部件表单（multipart/form-data）字段。命中时优先于 `body` 发送，
    /// Content-Type（含 boundary）由 reqwest 自动设置。
    #[serde(default)]
    pub multipart: Option<Vec<FormPart>>,
    /// 是否允许访问私有/保留/内网地址。API 调试工具常需访问内网服务，
    /// 开启后跳过内网 IP 拦截（仍强制拦截云元数据 169.254.169.254）。默认 false（保持安全拦截）。
    #[serde(default)]
    pub allow_private: bool,
}

/// 多部件表单（multipart/form-data）的单个字段。
///
/// 文本字段只填 `name` + `value`；文件字段填 `name` + `filename` + `content_type` + `data`
/// （`data` 为 base64 编码的文件字节，由前端经 WebView 文件选择器读取后传入，
/// 避免在 Rust 侧再走一次文件系统权限校验）。后端据此组装 `reqwest::multipart::Form`。
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FormPart {
    pub name: String,
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub filename: Option<String>,
    #[serde(default)]
    pub content_type: Option<String>,
    /// 文件内容（base64，不含 `data:` 前缀）
    #[serde(default)]
    pub data: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpResponse {
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub time_ms: u64,
    pub size: u64,
}
