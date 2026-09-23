//! 文件导入（统一解析层）。
//!
//! 取代原先「前端 TypeScript 解析 + 本地写」与「Java 端 `ImportService` 解析 + 落库」双份实现：
//! 现在所有格式（OpenAPI 3.x / Postman v2.1 / cURL / CanghaiApi 备份 / APIPost 8）都在 Rust 端解析，
//! 直接写入本地 SQLite（自动置 `dirty=1`），在线模式再走既有同步推送把数据上传到服务端。
//!
//! 解析逻辑逐段对应 `spring-boot-server/.../importer/ImportParser.java` 与 `ImportService.java`
//! （方法名与段落顺序刻意保持一致，便于对照）；落库阶段统一生成新 UUID、`sortOrder` 按目标
//! 分组现有最大值追加、时间戳取 `db::now_timestamp`（Asia/Shanghai），与 Java 端在线导入行为一致。
//!
//! 语义约定（与旧实现一致）：
//! - 导入永远是「新增」：分类 / 接口 / 环境均生成新 UUID，不与既有数据去重或覆盖；
//! - 文件里的 id 一律丢弃，分类 id 仅用于在解析产物内部建立父子 / 归属引用（源 id → 新 UUID 映射）；
//! - `sortOrder` 不使用文件里的值，而是「追加到目标分组现有最大值之后」。

use crate::db::{self, AppDb, Category, DataMode, DbConn, Environment, EnvironmentVariable, SavedRequest};
use crate::infra;
use crate::sync::{codes, run_sync, ApiResult};
use base64::Engine;
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use tauri::{AppHandle, Manager, State};

// ============================================================
// 中间结构（源 id，与 Java ImportParser 一致）
// ============================================================

#[derive(Debug, Clone)]
struct ParsedCategory {
    /// 源 id（文件里的 id / APIPost target_id / OpenAPI tag 等），落库时映射为新 UUID
    id: String,
    name: String,
    /// 源父 id；落库阶段映射为新 UUID，缺失则退化为根
    parent_id: Option<String>,
    #[allow(dead_code)]
    sort_order: i32,
    expanded: bool,
}

#[derive(Debug, Clone)]
struct ParsedRequest {
    name: String,
    method: String,
    url: String,
    params: Value,
    headers: Value,
    body_type: String,
    body: String,
    form_body: Value,
    /// 源分类 id；落库阶段映射为新 UUID
    category_id: Option<String>,
    pre_script: String,
    post_script: String,
}

#[derive(Debug, Clone)]
struct ParsedEnvVar {
    key: String,
    value: String,
    enabled: bool,
    #[allow(dead_code)]
    sort_order: i32,
}

#[derive(Debug, Clone)]
struct ParsedEnv {
    #[allow(dead_code)]
    source_id: String,
    name: String,
    variables: Vec<ParsedEnvVar>,
}

#[derive(Debug, Clone)]
struct ParsedCollection {
    source: String,
    categories: Vec<ParsedCategory>,
    requests: Vec<ParsedRequest>,
    environments: Vec<ParsedEnv>,
}

impl ParsedCollection {
    fn new(source: &str) -> Self {
        ParsedCollection {
            source: source.to_string(),
            categories: Vec::new(),
            requests: Vec::new(),
            environments: Vec::new(),
        }
    }
}

// ============================================================
// 通用 JSON 小工具
// ============================================================

fn as_obj<'a>(v: &'a Value) -> Option<&'a Map<String, Value>> {
    v.as_object()
}

fn as_arr(v: &Value) -> Option<&Vec<Value>> {
    v.as_array()
}

fn get_str<'a>(v: &'a Value, k: &str) -> Option<&'a str> {
    v.get(k).and_then(|x| x.as_str())
}

fn opt_str(v: &Value, k: &str) -> String {
    get_str(v, k).unwrap_or("").to_string()
}

fn opt_i64(v: &Value, k: &str) -> Option<i64> {
    v.get(k).and_then(|x| x.as_i64())
}

/// KV 序列化为 `{ key, value, enabled }`（与前端 KV 形状一致，直接落库为 JSON 数组）
fn kv(key: &str, value: &str, enabled: bool) -> Value {
    json!({ "key": key, "value": value, "enabled": enabled })
}

/// 空值兜底（JS 的 `a || fallback`：空串同样视为缺失）
fn empty_to(s: String, fallback: &str) -> String {
    if s.is_empty() {
        fallback.to_string()
    } else {
        s
    }
}

/// 取第一个非空值
fn first_non_empty(vals: &[Option<&str>]) -> String {
    for v in vals {
        if let Some(s) = v {
            if !s.is_empty() {
                return s.to_string();
            }
        }
    }
    String::new()
}

/// 标量转字符串：字符串原样；数字 / 布尔走字面量；null → 空串；其余走 JSON。
fn scalar_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// JSON 美化（2 空格，与 Java `JSON.toJSONString(..., PrettyFormat)` 后把 tab 替换为 2 空格一致）
fn pretty(v: &Value) -> String {
    serde_json::to_string_pretty(v).unwrap_or_else(|_| v.to_string())
}

fn default_for_type(t: Option<&str>) -> Value {
    match t {
        Some("string") => Value::String("string".into()),
        Some("integer") | Some("number") => json!(0),
        Some("boolean") => Value::Bool(false),
        Some("array") => Value::Array(vec![]),
        _ => Value::Null,
    }
}

fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

fn key_of(v: Option<String>) -> String {
    match v {
        Some(s) if !s.is_empty() => s,
        _ => String::new(),
    }
}

/// 取该分组下一个可用的 sortOrder（既有最大值 + 本次已分配数量）
fn next_order(counters: &mut HashMap<String, i32>, seeds: &HashMap<String, i32>, group_key: &str) -> i32 {
    let assigned = counters.entry(group_key.to_string()).or_insert(0);
    *assigned += 1;
    seeds.get(group_key).copied().unwrap_or(0) + *assigned
}

// ============================================================
// 0. 格式识别
// ============================================================

const OPENAPI_METHODS: &[&str] = &["get", "post", "put", "delete", "patch", "head", "options"];

const SUPPORTED_HINT: &str =
    "无法识别的文件格式（支持 OpenAPI / Postman / cURL / APIPost8 / CanghaiApi 备份）";

/// 识别并解析文件内容；返回解析后的中间结构，或错误信息（文案与 Java 端一致）
fn detect(text: &str) -> Result<ParsedCollection, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("文件内容为空".to_string());
    }
    // cURL：纯文本以 curl 开头
    if trimmed.to_lowercase().starts_with("curl") {
        return Ok(parse_curl(trimmed));
    }

    // JSON 解析失败时不能直接抛，否则会跳过下面的「多行 cURL 拼接」兜底
    let parsed: Value = match serde_json::from_str::<Value>(trimmed) {
        Ok(v) => v,
        Err(_) => {
            if trimmed.contains("curl ") {
                return Ok(parse_curl(trimmed));
            }
            return Err("不是有效的 JSON 格式".to_string());
        }
    };

    if let Some(obj) = parsed.as_object() {
        // 顺序与 Java 端一致：OpenAPI → Postman → APIPost → 自有格式
        if obj.contains_key("openapi") || obj.get("swagger").and_then(|v| v.as_str()).is_some() {
            return Ok(parse_openapi(&parsed));
        }
        if is_postman(&parsed) {
            return Ok(parse_postman(&parsed));
        }
        if is_apipost(&parsed) {
            return Ok(parse_apipost(&parsed));
        }
        if is_canghai(&parsed) {
            return Ok(parse_canghai(&parsed));
        }
    }
    Err(SUPPORTED_HINT.to_string())
}

fn is_postman(v: &Value) -> bool {
    if let Some(info) = v.get("info").and_then(as_obj) {
        if let Some(schema) = info.get("schema").and_then(|x| x.as_str()) {
            if schema.contains("getpostman.com") {
                return true;
            }
        }
    }
    v.get("item").is_some() || v.get("items").is_some()
}

fn is_apipost(v: &Value) -> bool {
    let apis = match v.get("apis").and_then(as_arr) {
        Some(a) => a,
        None => return false,
    };
    for n in apis {
        let o = match n.as_object() {
            Some(o) => o,
            None => return false,
        };
        let ok = o.get("target_type").and_then(|x| x.as_str()) == Some("folder")
            || o.get("target_type").and_then(|x| x.as_str()) == Some("api")
            || o.contains_key("target_id");
        if !ok {
            return false;
        }
    }
    true
}

fn is_canghai(v: &Value) -> bool {
    let is_one = v.get("version").and_then(|x| x.as_i64()) == Some(1);
    is_one && v.get("requests").is_some()
}

// ============================================================
// 1. OpenAPI 3.x 解析
// ============================================================

fn parse_openapi(doc: &Value) -> ParsedCollection {
    let mut out = ParsedCollection::new("openapi");

    // 为每个 tag 预建分类
    if let Some(tags) = doc.get("tags").and_then(as_arr) {
        for t in tags {
            let name = opt_str(t, "name");
            if name.is_empty() {
                continue;
            }
            let cat_id = format!("src-tag-{}", out.categories.len());
            out.categories.push(ParsedCategory {
                id: cat_id.clone(),
                name: name.clone(),
                parent_id: None,
                sort_order: (out.categories.len() + 1) as i32,
                expanded: true,
            });
        }
    }

    if let Some(paths) = doc.get("paths").and_then(as_obj) {
        for (path, item) in paths {
            let item = match item.as_object() {
                Some(o) => o,
                None => continue,
            };
            for m in OPENAPI_METHODS {
                let op = match item.get(*m).and_then(as_obj) {
                    Some(o) => o,
                    None => continue,
                };
                let tag = op
                    .get("tags")
                    .and_then(as_arr)
                    .and_then(|a| a.first())
                    .and_then(|x| x.as_str())
                    .unwrap_or("");
                // tag → 分类 id 映射（同名 tag 后者覆盖，与 Java Map.set 一致）
                let mut cat_map: HashMap<String, String> = HashMap::new();
                for c in &out.categories {
                    cat_map.insert(c.name.clone(), c.id.clone());
                }
                let cat_id = if tag.is_empty() {
                    None
                } else {
                    cat_map.get(tag).cloned()
                };
                let name = first_non_empty(&[
                    op.get("summary").and_then(|x| x.as_str()),
                    op.get("operationId").and_then(|x| x.as_str()),
                    Some(&format!("{} {}", m.to_uppercase(), path)),
                ]);
                out.requests.push(openapi_op_to_request(
                    m,
                    path,
                    op,
                    cat_id,
                    name,
                ));
            }
        }
    }
    out
}

fn openapi_op_to_request(
    method: &str,
    path: &str,
    op: &Map<String, Value>,
    category_id: Option<String>,
    name: String,
) -> ParsedRequest {
    let mut params: Vec<Value> = Vec::new();
    let mut headers: Vec<Value> = Vec::new();
    if let Some(parameters) = op.get("parameters").and_then(as_arr) {
        for p in parameters {
            let in_ = opt_str(p, "in");
            // TS: p.example ?? p.schema?.default ?? ''
            let example = p
                .get("example")
                .cloned()
                .or_else(|| p.get("schema").and_then(|s| s.get("default")).cloned());
            let kv = kv(
                p.get("name").and_then(|x| x.as_str()).unwrap_or(""),
                &scalar_to_string(example.as_ref().unwrap_or(&Value::Null)),
                true,
            );
            if in_ == "query" {
                params.push(kv);
            } else if in_ == "header" {
                headers.push(kv);
            }
        }
    }

    let mut body_type = "none".to_string();
    let mut body = String::new();
    if let Some(content) = op
        .get("requestBody")
        .and_then(as_obj)
        .and_then(|c| c.get("content"))
        .and_then(as_obj)
    {
        if content.get("application/json").is_some() {
            body_type = "json".to_string();
            if let Some(schema) = content.get("application/json") {
                body = schema_to_example(schema);
            }
        } else if content.get("application/x-www-form-urlencoded").is_some()
            || content.get("multipart/form-data").is_some()
        {
            body_type = "form".to_string();
        }
    }

    ParsedRequest {
        name,
        method: method.to_string(),
        url: path.to_string(),
        params: Value::Array(params),
        headers: Value::Array(headers),
        body_type,
        body,
        form_body: Value::Array(vec![]),
        category_id,
        pre_script: String::new(),
        post_script: String::new(),
    }
}

/// 由 JSON Schema 生成示例请求体（↔ Java schemaToExample）
fn schema_to_example(schema: &Value) -> String {
    if schema.is_null() {
        return String::new();
    }
    if let Some(obj) = schema.as_object() {
        if obj.contains_key("example") {
            return pretty(obj.get("example").unwrap());
        }
        if obj.get("type").and_then(|x| x.as_str()) == Some("object") {
            if let Some(props) = obj.get("properties").and_then(as_obj) {
                let mut example = Map::new();
                for (k, prop) in props {
                    let v = prop
                        .get("example")
                        .cloned()
                        .unwrap_or_else(|| default_for_type(prop.get("type").and_then(|x| x.as_str())));
                    example.insert(k.clone(), v);
                }
                return pretty(&Value::Object(example));
            }
        }
        return pretty(schema);
    }
    pretty(schema)
}

// ============================================================
// 2. Postman Collection v2.1 解析
// ============================================================

fn parse_postman(doc: &Value) -> ParsedCollection {
    let mut out = ParsedCollection::new("postman");
    let items = doc.get("item").or_else(|| doc.get("items"));
    if let Some(items) = items.and_then(as_arr) {
        walk_postman(&mut out, items, None);
    }
    out
}

fn walk_postman(out: &mut ParsedCollection, items: &[Value], parent_cat_id: Option<String>) {
    let mut sort_order_seed = 1;
    for node in items {
        let obj = match node.as_object() {
            Some(o) => o,
            None => continue,
        };
        if obj.get("item").and_then(as_arr).is_some() {
            // 文件夹 → 分类
            let cat_id = format!("src-pm-{}", out.categories.len());
            out.categories.push(ParsedCategory {
                id: cat_id.clone(),
                name: empty_to(opt_str(node, "name"), "未命名分类"),
                parent_id: parent_cat_id.clone(),
                sort_order: sort_order_seed,
                expanded: true,
            });
            sort_order_seed += 1;
            if let Some(children) = obj.get("item").and_then(as_arr) {
                walk_postman(out, children, Some(cat_id));
            }
        } else if obj.contains_key("request") {
            out.requests.push(postman_item_to_request(node, parent_cat_id.clone()));
        }
    }
}

/// 读取 Postman 节点 `event` 数组里的脚本：
/// - `listen: "prerequest"` → 前置脚本
/// - `listen: "test"`       → 后置脚本（测试脚本）
///
/// Postman 把每个事件写成 `{ listen, script: { type, exec, src } }`，其中 `exec` 通常是
/// **按行拆分的字符串数组**（需要重新用 `\n` 拼回），也兼容直接为单个字符串的写法；
/// `disabled: true` 的事件跳过；`script.src` 为外部文件引用，无法随集合导出携带，忽略。
fn postman_extract_script(node: &Value, listen: &str) -> String {
    let events = match node.get("event").and_then(as_arr) {
        Some(e) => e,
        None => return String::new(),
    };
    let mut blocks: Vec<String> = Vec::new();
    for ev in events {
        if ev.get("disabled").and_then(|x| x.as_bool()).unwrap_or(false) {
            continue;
        }
        let ev_listen = ev.get("listen").and_then(|x| x.as_str()).unwrap_or("");
        // 兼容 "prerequest" / "pre-request" 两种写法
        let matched = match listen {
            "prerequest" => ev_listen == "prerequest" || ev_listen == "pre-request",
            _ => ev_listen == listen,
        };
        if !matched {
            continue;
        }
        let script = match ev.get("script") {
            Some(s) => s,
            None => continue,
        };
        // 脚本正文：exec 为字符串数组（按行）或单个字符串
        let text = match script.get("exec") {
            Some(Value::Array(arr)) => arr
                .iter()
                .map(scalar_to_string)
                .collect::<Vec<String>>()
                .join("\n"),
            Some(Value::String(s)) => s.clone(),
            _ => String::new(),
        };
        if !text.trim().is_empty() {
            blocks.push(text);
        }
    }
    blocks.join("\n\n")
}

fn postman_item_to_request(node: &Value, category_id: Option<String>) -> ParsedRequest {
    let req = node.get("request").and_then(as_obj).cloned().unwrap_or_default();
    let method = empty_to(opt_str(&Value::Object(req.clone()), "method"), "GET").to_uppercase();

    let mut url = String::new();
    let mut params: Vec<Value> = Vec::new();
    match req.get("url") {
        Some(Value::String(s)) => url = s.clone(),
        Some(u @ Value::Object(_)) => {
            url = opt_str(u, "raw");
            if let Some(query) = u.get("query").and_then(as_arr) {
                for q in query {
                    let key = q.get("key").and_then(|x| x.as_str()).unwrap_or("").to_string();
                    let value = q
                        .get("value")
                        .map(scalar_to_string)
                        .unwrap_or_default();
                    params.push(kv(&key, &value, true));
                }
            }
        }
        _ => {}
    }

    let mut headers: Vec<Value> = Vec::new();
    if let Some(header_arr) = req.get("header").and_then(as_arr) {
        for h in header_arr {
            if h.get("disabled").and_then(|x| x.as_bool()).unwrap_or(false) {
                continue;
            }
            let key = h.get("key").and_then(|x| x.as_str()).unwrap_or("").to_string();
            let value = h.get("value").map(scalar_to_string).unwrap_or_default();
            headers.push(kv(&key, &value, true));
        }
    }

    let mut body_type = "none".to_string();
    let mut body = String::new();
    if let Some(body_obj) = req.get("body").and_then(as_obj) {
        let mode = opt_str(&Value::Object(body_obj.clone()), "mode");
        if mode == "raw" {
            body_type = "json".to_string();
            body = opt_str(&Value::Object(body_obj.clone()), "raw");
        } else if mode == "urlencoded" && body_obj.get("urlencoded").and_then(as_arr).is_some() {
            // 与 Java/前端一致：只标记为 form，不导入键值
            body_type = "form".to_string();
        }
    }

    let name = empty_to(opt_str(node, "name"), &format!("{} {}", method, url));
    ParsedRequest {
        name,
        method,
        url,
        params: Value::Array(params),
        headers: Value::Array(headers),
        body_type,
        body,
        form_body: Value::Array(vec![]),
        category_id,
        pre_script: postman_extract_script(node, "prerequest"),
        post_script: postman_extract_script(node, "test"),
    }
}

// ============================================================
// 3. cURL 命令解析（单条）
// ============================================================

fn parse_curl(text: &str) -> ParsedCollection {
    let mut out = ParsedCollection::new("curl");
    if let Some(req) = parse_single_curl(text) {
        out.requests.push(req);
    }
    out
}

/// 去掉换行续行符（\ 后跟可选空白再换行 → 空格）
fn strip_line_continuations(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            let mut j = i + 1;
            while j < bytes.len() && (bytes[j] == b' ' || bytes[j] == b'\t' || bytes[j] == b'\r') {
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b'\n' {
                out.push(' ');
                i = j + 1;
                continue;
            }
        }
        let ch = s[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

/// 简易 tokenizer：处理单/双引号（↔ Java ImportParser.tokenize）
fn tokenize(s: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut quote: Option<char> = None;
    for c in s.chars() {
        if let Some(q) = quote {
            if c == q {
                quote = None;
            } else {
                cur.push(c);
            }
        } else if c == '"' || c == '\'' {
            quote = Some(c);
        } else if c == ' ' || c == '\t' || c == '\n' {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
        } else {
            cur.push(c);
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn parse_single_curl(text: &str) -> Option<ParsedRequest> {
    let raw = strip_line_continuations(text);
    let raw = raw.trim();
    if !raw.to_lowercase().starts_with("curl") {
        return None;
    }

    let tokens = tokenize(&raw[4..]);
    let mut params: Vec<Value> = Vec::new();
    let mut headers: Vec<Value> = Vec::new();
    let mut method = "GET".to_string();
    let mut url = String::new();
    let mut body = String::new();

    let mut iter = tokens.into_iter().enumerate();
    while let Some((_, t)) = iter.next() {
        match t.as_str() {
            "-X" | "--request" => {
                method = empty_to(
                    iter.next().map(|(_, v)| v).unwrap_or_default(),
                    "GET",
                )
                .to_uppercase();
            }
            "-H" | "--header" => {
                let h = iter.next().map(|(_, v)| v).unwrap_or_default();
                if let Some(idx) = h.find(':') {
                    if idx > 0 {
                        headers.push(kv(&h[..idx].trim().to_string(), &h[idx + 1..].trim().to_string(), true));
                    }
                }
            }
            "-d" | "--data" | "--data-raw" | "--data-binary" => {
                body = iter.next().map(|(_, v)| v).unwrap_or_default();
            }
            "-u" | "--user" => {
                let u = iter.next().map(|(_, v)| v).unwrap_or_default();
                let enc = Engine::encode(&base64::engine::general_purpose::STANDARD, u.as_bytes());
                headers.push(kv("Authorization", &format!("Basic {enc}"), true));
            }
            _ => {
                let t2 = t.clone();
                if t2.len() > 1 && t2.starts_with('-') && !t2.starts_with("--") {
                    // 合并短选项（如 -fsS）：跳过其取值
                    iter.next();
                } else if t2.starts_with("--") {
                    iter.next(); // 跳过未知长选项的值
                } else if url.is_empty()
                    && (t2.starts_with("http://")
                        || t2.starts_with("https://")
                        || t2.starts_with("{{")
                        || t2.starts_with('/'))
                {
                    url = t2;
                }
            }
        }
    }

    // 拆分 query 到 params（仅绝对 URL；相对/带模板变量原样保留）
    if let Ok(parsed) = url::Url::parse(&url) {
        if parsed.query().is_some() {
            let q = parsed.query().unwrap_or("");
            for pair in q.split('&') {
                if pair.is_empty() {
                    continue;
                }
                let eq = pair.find('=');
                let (k, v) = match eq {
                    Some(pos) => (&pair[..pos], &pair[pos + 1..]),
                    None => (pair, ""),
                };
                let k = percent_decode(k);
                let v = percent_decode(v);
                params.push(kv(&k, &v, true));
            }
            let mut cleaned = parsed;
            cleaned.set_query(None);
            url = cleaned.to_string();
        }
    }

    let mut body_type = "none".to_string();
    if !body.is_empty() {
        let content_type = headers
            .iter()
            .find_map(|h| {
                let k = h.get("key").and_then(|x| x.as_str()).unwrap_or("");
                if k.eq_ignore_ascii_case("content-type") {
                    h.get("value").and_then(|x| x.as_str())
                } else {
                    None
                }
            })
            .unwrap_or("");
        body_type = if content_type.contains("json") || looks_json(&body) {
            "json".to_string()
        } else {
            "text".to_string()
        };
    }

    let name = format!("{} {}", method, if url.is_empty() { "cURL 导入" } else { &url });
    Some(ParsedRequest {
        name,
        method,
        url,
        params: Value::Array(params),
        headers: Value::Array(headers),
        body_type,
        body,
        form_body: Value::Array(vec![]),
        category_id: None,
        pre_script: String::new(),
        post_script: String::new(),
    })
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hi = (bytes[i + 1] as char).to_digit(16);
            let lo = (bytes[i + 2] as char).to_digit(16);
            if let (Some(h), Some(l)) = (hi, lo) {
                out.push((h * 16 + l) as u8);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn looks_json(s: &str) -> bool {
    let t = s.trim();
    (t.starts_with('{') && t.ends_with('}')) || (t.starts_with('[') && t.ends_with(']'))
}

// ============================================================
// 4. 自有备份格式（canghai v1）
// ============================================================

fn parse_canghai(doc: &Value) -> ParsedCollection {
    let mut out = ParsedCollection::new("canghai");

    if let Some(categories) = doc.get("categories").and_then(as_arr) {
        for c in categories {
            let id = opt_str(c, "id");
            if id.is_empty() {
                continue;
            }
            out.categories.push(ParsedCategory {
                id,
                name: opt_str(c, "name"),
                parent_id: get_str(c, "parentId").map(|s| s.to_string()),
                sort_order: opt_i64(c, "sortOrder").unwrap_or(0) as i32,
                expanded: c.get("expanded").and_then(|x| x.as_bool()).unwrap_or(true),
            });
        }
    }

    if let Some(requests) = doc.get("requests").and_then(as_arr) {
        for r in requests {
            out.requests.push(ParsedRequest {
                name: opt_str(r, "name"),
                method: empty_to(opt_str(r, "method"), "GET").to_uppercase(),
                url: opt_str(r, "url"),
                params: r.get("params").cloned().unwrap_or(Value::Array(vec![])),
                headers: r.get("headers").cloned().unwrap_or(Value::Array(vec![])),
                body_type: empty_to(opt_str(r, "bodyType"), "none"),
                body: opt_str(r, "body"),
                form_body: r.get("formBody").cloned().unwrap_or(Value::Array(vec![])),
                category_id: get_str(r, "categoryId").map(|s| s.to_string()),
                pre_script: opt_str(r, "preScript"),
                post_script: opt_str(r, "postScript"),
            });
        }
    }

    if let Some(environments) = doc.get("environments").and_then(as_arr) {
        for e in environments {
            let mut env = ParsedEnv {
                source_id: opt_str(e, "id"),
                name: empty_to(opt_str(e, "name"), "未命名环境"),
                variables: Vec::new(),
            };
            if let Some(variables) = e.get("variables").and_then(as_arr) {
                for (j, v) in variables.iter().enumerate() {
                    let key = opt_str(v, "key");
                    if key.is_empty() {
                        continue;
                    }
                    env.variables.push(ParsedEnvVar {
                        key,
                        value: v.get("value").map(scalar_to_string).unwrap_or_default(),
                        enabled: v.get("enabled").and_then(|x| x.as_bool()).unwrap_or(true),
                        sort_order: opt_i64(v, "sortOrder").unwrap_or((j + 1) as i64) as i32,
                    });
                }
            }
            out.environments.push(env);
        }
    }
    out
}

// ============================================================
// 5. APIPost 8 导出格式解析
// ============================================================

fn parse_apipost(doc: &Value) -> ParsedCollection {
    let mut out = ParsedCollection::new("apipost");
    out.environments.extend(parse_apipost_environments(
        doc.get("global").and_then(as_obj),
    ));

    let apis = match doc.get("apis").and_then(as_arr) {
        Some(a) => a,
        None => return out,
    };
    for node in apis {
        let obj = match node.as_object() {
            Some(o) => o,
            None => continue,
        };
        let id = opt_str(node, "target_id");
        let raw_parent = opt_str(node, "parent_id");
        // APIPost 顶层节点 parent_id 为 "0"，归一化为根（None）
        let parent_id = if raw_parent.is_empty() || raw_parent == "0" {
            None
        } else {
            Some(raw_parent)
        };

        if obj.get("target_type").and_then(|x| x.as_str()) == Some("folder") {
            out.categories.push(ParsedCategory {
                id,
                name: empty_to(opt_str(node, "name"), "未命名目录"),
                parent_id,
                sort_order: opt_i64(node, "sort").unwrap_or((out.categories.len() + 1) as i64) as i32,
                expanded: true,
            });
            continue;
        }

        // api 节点
        let req = node.get("request").and_then(as_obj).cloned().unwrap_or_default();
        // APIPost 8 把 method 放在 API 节点顶层；部分变体可能在 request 下，二者都兼容。
        let method_src = opt_str(node, "method");
        let method_src = if method_src.is_empty() {
            opt_str(&Value::Object(req.clone()), "method")
        } else {
            method_src
        };
        let method = empty_to(method_src, "GET").to_uppercase();
        let url = opt_str(node, "url");

        let bd = req.get("body").and_then(as_obj).cloned().unwrap_or_default();
        let form_body_list = apipost_kv_to_kv(bd.get("parameter").and_then(as_arr));
        let raw_text = opt_str(&Value::Object(bd.clone()), "raw");
        let mode = opt_str(&Value::Object(bd.clone()), "mode").to_lowercase();
        let effective_mode = if !mode.is_empty() {
            mode
        } else if !form_body_list.is_empty() {
            "form-data".to_string()
        } else if !raw_text.is_empty() {
            "json".to_string()
        } else {
            "none".to_string()
        };

        let mut body_type = "none".to_string();
        let mut body = String::new();
        let mut form_body: Vec<Value> = Vec::new();
        match effective_mode.as_str() {
            "form-data" | "urlencoded" | "x-www-form-urlencoded" => {
                body_type = "form".to_string();
                form_body = form_body_list;
            }
            "json" => {
                body_type = "json".to_string();
                body = raw_text;
            }
            "text" | "xml" | "html" | "javascript" | "js" => {
                body_type = "text".to_string();
                body = raw_text;
            }
            "binary" => {
                body_type = "text".to_string();
                body = match bd.get("binary") {
                    Some(Value::String(s)) => s.clone(),
                    Some(other) => other.get("path").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                    None => String::new(),
                };
            }
            _ => {}
        }

        let name = empty_to(opt_str(node, "name"), &format!("{} {}", method, url));
        out.requests.push(ParsedRequest {
            name,
            method,
            url,
            params: Value::Array(apipost_kv_to_kv(req.get("query").and_then(as_obj).and_then(|q| q.get("parameter")).and_then(as_arr))),
            headers: Value::Array(apipost_kv_to_kv(req.get("header").and_then(as_obj).and_then(|h| h.get("parameter")).and_then(as_arr))),
            body_type,
            body,
            form_body: Value::Array(form_body),
            category_id: parent_id,
            pre_script: apipost_extract_script(
                req.get("pre_tasks")
                    .and_then(as_arr)
                    .or_else(|| node.get("pre_tasks").and_then(as_arr)),
            ),
            post_script: apipost_extract_script(
                req.get("post_tasks")
                    .and_then(as_arr)
                    .or_else(|| node.get("post_tasks").and_then(as_arr)),
            ),
        });
    }
    out
}

fn apipost_kv_to_kv(list: Option<&Vec<Value>>) -> Vec<Value> {
    let list = match list {
        Some(l) => l,
        None => return Vec::new(),
    };
    let mut out = Vec::new();
    for p in list {
        // 只要求 key 字段「存在」（允许为 null）
        if !p.get("key").is_some() {
            continue;
        }
        let key = p.get("key").map(scalar_to_string).unwrap_or_default();
        if key.is_empty() {
            continue;
        }
        let value = p.get("value").map(scalar_to_string).unwrap_or_default();
        let enabled = match p.get("is_checked") {
            None => true,
            Some(Value::Bool(b)) => *b,
            Some(Value::Number(n)) => n.as_i64() == Some(1),
            _ => false,
        };
        out.push(kv(&key, &value, enabled));
    }
    out
}

/// APIPost 8 脚本任务 → 本软件脚本（Postman/pm 语法）转换。
///
/// APIPost 前后置脚本使用 `apt.*` API；本软件（`useScriptEngine.ts`）使用 `pm.*`
/// （Postman 语法：`pm.request.headers.add` / `pm.variables.set` / `pm.environment.set` /
/// `pm.response.code` / `pm.response.json()` / `pm.test` / `pm.expect` / `CryptoJS` 等），
/// 在独立 Web Worker 沙箱中执行。导入时做一次尽力转换，使脚本可直接运行。
///
/// 覆盖：`apt.setHeader/getHeader/removeHeader`、`apt.setRequestUrl/setRequestBody`、
/// `apt.getRequestUrl()/getRequestBody()`、`apt.setEnvVariable/getEnvVariable`、
/// `apt.setGlobalVariable/getGlobalVariable`、`apt.setLocalVariable/getLocalVariable`、
/// `apt.variables` / `apt.environmentVariables` / `apt.globalVariables` / `apt.globals`、
/// `apt.response.*`（含 statusCode/raw/body）、`apt.test`、`apt.assert`、
/// `apt.beforeSendRequest` / `apt.afterResponse` 回调式包裹（拆为顶层语句并映射 request/response），
/// 以及 `kv` 结构化任务（设为变量）。
/// 未覆盖：APIPost 内置函数（`customApi.*`，由 customApiId 引用的断言/工具）、`apt.sendRequest`、
/// 及 `wait`/`loop`/`if` 等结构化任务——会插入注释提示人工核对。
/// 从任务里取出脚本文本（兼容 data 为字符串，或 { customApiId, customScript } 对象）。
/// 返回 (脚本文本, 是否引用了 customApiId)。
fn extract_script_data(t: &Value) -> (String, bool) {
    match t.get("data") {
        Some(Value::String(s)) => (s.to_string(), false),
        Some(Value::Object(o)) => {
            let s = o
                .get("customScript")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            let api = o
                .get("customApiId")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            (s, !api.is_empty())
        }
        _ => (String::new(), false),
    }
}

fn apipost_extract_script(tasks: Option<&Vec<Value>>) -> String {
    let tasks = match tasks {
        Some(t) => t,
        None => return String::new(),
    };
    let mut blocks: Vec<String> = Vec::new();
    for t in tasks {
        let ttype = t.get("type").and_then(|x| x.as_str()).unwrap_or("");
        // enabled=-1 表示禁用，跳过（缺省视为启用）
        if t.get("enabled").and_then(|x| x.as_i64()) == Some(-1) {
            continue;
        }
        let (script_data, had_custom_api) = extract_script_data(t);
        match ttype {
            "customScript" | "function" => {
                if !script_data.trim().is_empty() {
                    let mut b = String::new();
                    if had_custom_api {
                        b.push_str("// APIPost 内置函数（customApi.*）可能需手动核对\n");
                    }
                    b.push_str(&convert_apt_to_pm(&script_data));
                    blocks.push(b);
                }
            }
            "kv" => {
                // 结构化键值任务：data 可能含 { key, value, scope }，或直接挂在任务上
                let data = t.get("data");
                let key = data
                    .and_then(|d| d.get("key"))
                    .and_then(|x| x.as_str())
                    .or_else(|| t.get("key").and_then(|x| x.as_str()))
                    .unwrap_or("");
                if key.is_empty() {
                    continue;
                }
                let value = data
                    .and_then(|d| d.get("value"))
                    .and_then(|x| x.as_str())
                    .or_else(|| t.get("value").and_then(|x| x.as_str()))
                    .unwrap_or("");
                let scope = data
                    .and_then(|d| d.get("scope"))
                    .and_then(|x| x.as_str())
                    .unwrap_or("");
                let target = if scope == "environment" { "pm.environment" } else { "pm.variables" };
                let key_lit = serde_json::to_string(key).unwrap_or_else(|_| "\"\"".to_string());
                let val_lit = serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_string());
                blocks.push(format!("{target}.set({key_lit}, {val_lit});"));
            }
            other => {
                if !other.is_empty() {
                    if !script_data.trim().is_empty() {
                        // 未知类型的任务仍携带脚本文本：尽力转换并提示人工核对
                        blocks.push(format!(
                            "// [APIPost 任务 type={other}，已尽力转换，请核对脚本语义]"
                        ));
                        blocks.push(convert_apt_to_pm(&script_data));
                    } else {
                        blocks.push(format!(
                            "// [未自动转换的 APIPost 任务 type={other}] 请手动核对其脚本语义"
                        ));
                    }
                }
            }
        }
    }
    if blocks.is_empty() {
        return String::new();
    }
    let header = "// 由 APIPost 脚本自动转换（apt.* → pm.* Postman 语法）。\n\
                  // 注意：APIPost 内置函数（customApi.*）及部分结构化任务可能需手动核对。\n";
    format!("{header}\n{}", blocks.join("\n\n"))
}

/// 把一段 APIPost 脚本（apt.* 语法）转成本软件的 pm.* 语法（尽力而为）。
///
/// 覆盖 APIPost 8 常用 API：
/// - 变量：setEnvVariable/getEnvVariable → pm.environment.*；setGlobalVariable/getGlobalVariable
///   → pm.globals.*；setLocalVariable/getLocalVariable/setVariable/getVariable → pm.variables.*
/// - 请求：setHeader/getHeader/removeHeader → pm.request.headers.*；setRequestUrl/setRequestBody
///   → 赋值语句；getRequestUrl()/getRequestBody() → 属性读取
/// - 响应：statusCode → pm.response.code；raw/body → pm.response.text()；json()/headers 等经
///   apt.response. → pm.response. 通用前缀兜底
/// - 断言：assert → pm.test + pm.expect；test/expect 直接透传
/// - 回调式包裹：apt.beforeSendRequest = function(request){...} / apt.afterResponse =
///   function(response){...} 拆为顶层语句，回调参数映射为 pm.request/pm.response
/// 未覆盖项（如 apt.sendRequest、customApi.* 内置函数）保留原样并附注释提示人工核对。
fn convert_apt_to_pm(js: &str) -> String {
    let mut s = js.to_string();
    // 1. 拆掉回调式包裹（beforeSendRequest/afterResponse），参数名映射为 pm.request/pm.response
    s = unwrap_apt_callback(&s, "apt.beforeSendRequest", "pm.request");
    s = unwrap_apt_callback(&s, "apt.afterResponse", "pm.response");
    // 2. 赋值型调用
    s = convert_assign_call(&s, "apt.setRequestUrl(", "pm.request.url = ");
    s = convert_assign_call(&s, "apt.setRequestBody(", "pm.request.body.raw = ");
    // 3. apt.assert → pm.test + pm.expect
    s = convert_assert(&s);
    // 4. 1:1 重命名（更具体的先匹配，避免被通用前缀吞掉）
    let renames: &[(&str, &str)] = &[
        ("apt.setEnvVariable(", "pm.environment.set("),
        ("apt.getEnvVariable(", "pm.environment.get("),
        ("apt.setGlobalVariable(", "pm.globals.set("),
        ("apt.getGlobalVariable(", "pm.globals.get("),
        ("apt.setLocalVariable(", "pm.variables.set("),
        ("apt.getLocalVariable(", "pm.variables.get("),
        ("apt.setVariable(", "pm.variables.set("),
        ("apt.getVariable(", "pm.variables.get("),
        ("apt.environmentVariables.set(", "pm.environment.set("),
        ("apt.environmentVariables.get(", "pm.environment.get("),
        ("apt.globalVariables.set(", "pm.globals.set("),
        ("apt.globalVariables.get(", "pm.globals.get("),
        ("apt.environment.set(", "pm.environment.set("),
        ("apt.environment.get(", "pm.environment.get("),
        ("apt.setHeader(", "pm.request.headers.add("),
        ("apt.getHeader(", "pm.request.headers.get("),
        ("apt.removeHeader(", "pm.request.headers.remove("),
        ("apt.variables.set(", "pm.variables.set("),
        ("apt.variables.get(", "pm.variables.get("),
        ("apt.globals.set(", "pm.globals.set("),
        ("apt.globals.get(", "pm.globals.get("),
        ("apt.getRequestUrl()", "pm.request.url"),
        ("apt.getRequestBody()", "pm.request.body.raw"),
        ("apt.response.statusCode", "pm.response.code"),
        ("apt.response.raw", "pm.response.text()"),
        ("apt.response.body", "pm.response.text()"),
        ("apt.response.status", "pm.response.code"),
        ("apt.response.message", "pm.response.status"),
        ("apt.request.", "pm.request."),
        ("apt.response.", "pm.response."),
        ("apt.environmentVariables.", "pm.environment."),
        ("apt.globalVariables.", "pm.globals."),
        ("apt.environment.", "pm.environment."),
        ("apt.globals.", "pm.globals."),
        ("apt.variables.", "pm.variables."),
        ("apt.test(", "pm.test("),
        ("apt.expect(", "pm.expect("),
    ];
    for (from, to) in renames {
        s = s.replace(from, to);
    }
    s
}

/// 去掉 APIPost 回调式包裹：
///   apt.beforeSendRequest = function(request) { ... }
///   apt.afterResponse = function(response) { ... }
/// 提取函数体为顶层语句，并把回调参数名（如 request/response）映射为 pm.request/pm.response。
/// 无法识别结构时原样返回（由后续 apt.* 重命名兜底）。
fn unwrap_apt_callback(js: &str, prefix: &str, pm_target: &str) -> String {
    let needle = format!("{prefix} = function");
    let pos = match js.find(&needle) {
        Some(p) => p,
        None => return js.to_string(),
    };
    let after_fn = &js[pos + needle.len()..];
    // 解析参数名：(name)
    let param = after_fn.find('(').and_then(|s| {
        let inside = &after_fn[s + 1..];
        let end = inside.find(|c| c == ',' || c == ')').unwrap_or(inside.len());
        let name = inside[..end].trim();
        if name.is_empty() {
            None
        } else {
            Some(name.to_string())
        }
    });
    // 定位函数体 { ... }（括号匹配）
    let body_start = match after_fn.find('{') {
        Some(b) => b,
        None => return js.to_string(),
    };
    let mut depth = 0i32;
    let mut body_end = None;
    for (i, c) in after_fn[body_start..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    body_end = Some(body_start + i);
                    break;
                }
            }
            _ => {}
        }
    }
    let body_end = match body_end {
        Some(e) => e,
        None => return js.to_string(),
    };
    let mut body = after_fn[body_start + 1..body_end].to_string();
    if let Some(p) = &param {
        // 把回调参数名的成员访问（如 request. / response.）映射为 pm.request/pm.response
        body = body.replace(&format!("{p}."), &format!("{pm_target}."));
    }
    let mut out = String::with_capacity(js.len());
    out.push_str(&js[..pos]);
    out.push_str(body.trim());
    out.push_str(&js[body_end + 1..]);
    out
}

/// 将 `from(ARGS)` 形式的调用替换为 `to ARGS`（去掉匹配的右括号），用于赋值型 API。
fn convert_assign_call(js: &str, from: &str, to: &str) -> String {
    let mut out = String::with_capacity(js.len());
    let mut rest = js;
    while let Some(pos) = rest.find(from) {
        out.push_str(&rest[..pos]);
        out.push_str(to);
        let after = &rest[pos + from.len()..];
        let mut depth = 0i32;
        let mut end = None;
        for (i, c) in after.char_indices() {
            match c {
                '(' => depth += 1,
                ')' => {
                    if depth == 0 {
                        end = Some(i);
                        break;
                    }
                    depth -= 1;
                }
                _ => {}
            }
        }
        match end {
            Some(i) => {
                out.push_str(&after[..i]);
                rest = &after[i + 1..];
            }
            None => {
                out.push_str(from);
                rest = &rest[pos + from.len()..];
                break;
            }
        }
    }
    out.push_str(rest);
    out
}

/// apt.assert(cond, msg) → pm.test(msg, () => { pm.expect(cond).to.be.true })
fn convert_assert(js: &str) -> String {
    let mut out = String::with_capacity(js.len());
    let mut rest = js;
    let from = "apt.assert(";
    while let Some(pos) = rest.find(from) {
        out.push_str(&rest[..pos]);
        let after = &rest[pos + from.len()..];
        let mut depth = 0i32;
        let mut args_end = None;
        for (i, c) in after.char_indices() {
            match c {
                '(' => depth += 1,
                ')' => {
                    if depth == 0 {
                        args_end = Some(i);
                        break;
                    }
                    depth -= 1;
                }
                _ => {}
            }
        }
        match args_end {
            Some(i) => {
                let (cond, msg) = split_top_args(&after[..i]);
                out.push_str(&format!(
                    "pm.test({msg}, () => {{ pm.expect({cond}).to.be.true }})"
                ));
                rest = &after[i + 1..];
            }
            None => {
                out.push_str(from);
                rest = &rest[pos + from.len()..];
                break;
            }
        }
    }
    out.push_str(rest);
    out
}

/// 按顶层逗号把实参拆分为 (第一项, 第二项)；不足则第二项补空字符串字面量。
fn split_top_args(args: &str) -> (String, String) {
    let mut depth = 0i32;
    let mut idx = None;
    for (i, c) in args.char_indices() {
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            ',' if depth == 0 => {
                idx = Some(i);
                break;
            }
            _ => {}
        }
    }
    match idx {
        Some(i) => (args[..i].trim().to_string(), args[i + 1..].trim().to_string()),
        None => (args.trim().to_string(), "''".to_string()),
    }
}

/// 解析 APIPost 8 的 global.envs：
/// env_var_list 为 { 变量名: { value, current_value, description } } 形式；
/// server_list[].uri 为服务前置地址，作为 baseUrl 变量一并导入（不覆盖同名变量）。
fn parse_apipost_environments(global: Option<&Map<String, Value>>) -> Vec<ParsedEnv> {
    let global = match global {
        Some(g) => g,
        None => return Vec::new(),
    };
    let envs = match global.get("envs").and_then(as_arr) {
        Some(e) => e,
        None => return Vec::new(),
    };
    let mut out = Vec::new();
    for env in envs {
        let obj = match env.as_object() {
            Some(o) => o,
            None => continue,
        };
        let mut parsed = ParsedEnv {
            source_id: opt_str(env, "env_id"),
            name: empty_to(opt_str(env, "name"), "未命名环境"),
            variables: Vec::new(),
        };

        if let Some(var_map) = obj.get("env_var_list").and_then(as_obj) {
            for (key, def) in var_map {
                if key.is_empty() {
                    continue;
                }
                // value 为远程值，current_value 为本地值，两者都可能为空
                let raw = def
                    .get("value")
                    .cloned()
                    .or_else(|| def.get("current_value").cloned());
                let value = raw.map(|v| scalar_to_string(&v)).unwrap_or_default();
                parsed.variables.push(ParsedEnvVar {
                    key: key.clone(),
                    value,
                    enabled: true,
                    sort_order: (parsed.variables.len() + 1) as i32,
                });
            }
        }

        // 服务前置地址：作为 baseUrl 变量，使导入的接口能直接解析 {{baseUrl}}
        let uri = obj
            .get("server_list")
            .and_then(as_arr)
            .and_then(|servers| {
                servers.iter().find_map(|s| {
                    s.get("uri")
                        .and_then(|x| x.as_str())
                        .filter(|u| !u.trim().is_empty())
                })
            })
            .unwrap_or("")
            .to_string();
        let has_base_url = parsed.variables.iter().any(|v| v.key == "baseUrl");
        if !uri.is_empty() && !has_base_url {
            parsed.variables.push(ParsedEnvVar {
                key: "baseUrl".to_string(),
                value: uri,
                enabled: true,
                sort_order: (parsed.variables.len() + 1) as i32,
            });
        }
        out.push(parsed);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apt_to_pm_variables_and_response() {
        let src = "apt.setEnvVariable(\"token\", apt.response.json().token);\n\
                   apt.test(\"status 200\", function() {\n  \
                   apt.assert(apt.response.statusCode == 200, \"应为200\");\n});";
        let out = convert_apt_to_pm(src);
        assert!(
            out.contains("pm.environment.set(\"token\", pm.response.json().token)"),
            "env var 未转换: {out}"
        );
        assert!(out.contains("pm.response.code == 200"), "statusCode 未转换: {out}");
        assert!(out.contains("pm.test("), "test 未转换: {out}");
        assert!(!out.contains("apt."), "残留 apt.: {out}");
    }

    #[test]
    fn apt_after_response_wrapper() {
        let src = "apt.afterResponse = function(response) {\n  \
                   var t = response.json().token;\n  \
                   apt.setGlobalVariable(\"t\", t);\n};";
        let out = convert_apt_to_pm(src);
        assert!(out.contains("pm.response.json().token"), "response 未映射: {out}");
        assert!(out.contains("pm.globals.set(\"t\", t)"), "globals 未转换: {out}");
        assert!(!out.contains("apt."), "残留 apt.: {out}");
    }

    #[test]
    fn apt_extract_script_converts_unknown_type() {
        let tasks = serde_json::json!([
            { "type": "customScript", "data": "apt.setEnvVariable('a','1');" },
            { "type": "weird", "data": "apt.setEnvVariable('b','2');" }
        ]);
        let out = apipost_extract_script(tasks.as_array());
        assert!(out.contains("pm.environment.set('a','1')"), "{out}");
        assert!(out.contains("pm.environment.set('b','2')"), "{out}");
    }

    #[test]
    fn apipost_method_read_from_node() {
        let doc = serde_json::json!({
            "apis": [
                {
                    "target_id": "1",
                    "parent_id": "0",
                    "name": "登录",
                    "url": "/login",
                    "method": "post",
                    "request": { "header": {"parameter": []}, "query": {"parameter": []}, "body": {} }
                }
            ]
        });
        let out = parse_apipost(&doc);
        assert_eq!(out.requests.len(), 1, "应解析出 1 个接口");
        assert_eq!(out.requests[0].method, "POST", "method 应从节点顶层读取并大写");
    }

    #[test]
    fn postman_event_scripts_are_imported() {
        let doc = serde_json::json!({
            "info": { "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json" },
            "item": [
                {
                    "name": "获取token",
                    "event": [
                        {
                            "listen": "prerequest",
                            "script": { "type": "text/javascript", "exec": ["console.log('pre');", "pm.variables.set('a','1');"] }
                        },
                        {
                            "listen": "test",
                            "script": { "type": "text/javascript", "exec": ["var body = pm.response.json();", "pm.environment.set('token', body.data.accessToken);"] }
                        },
                        {
                            "listen": "test",
                            "disabled": true,
                            "script": { "type": "text/javascript", "exec": ["pm.environment.set('x','y');"] }
                        }
                    ],
                    "request": { "method": "POST", "url": { "raw": "{{host}}/login" } }
                }
            ]
        });
        let out = parse_postman(&doc);
        assert_eq!(out.requests.len(), 1);
        let r = &out.requests[0];
        assert!(r.pre_script.contains("console.log('pre');"), "前置脚本缺失: {}", r.pre_script);
        assert!(r.pre_script.contains("pm.variables.set('a','1');"), "前置脚本行未拼回: {}", r.pre_script);
        assert!(r.post_script.contains("pm.environment.set('token', body.data.accessToken);"), "后置脚本缺失: {}", r.post_script);
        assert!(!r.post_script.contains("pm.environment.set('x','y');"), "禁用的 test 事件不应导入: {}", r.post_script);
    }
}

// ============================================================
// 落库：源 id → 新 UUID 映射 + sortOrder 追加 + 写入本地库
// ============================================================

/// 构建实体并写入本地 SQLite（自动置 dirty=1），返回统计（分类数 / 接口数 / 环境数 / 环境变量数）。
fn build_and_write(
    conn: &DbConn,
    mode: DataMode,
    parsed: &ParsedCollection,
    project_id: &str,
    user_id: &str,
    now: &str,
) -> (i32, i32, i32, i32) {
    // ---- 分类：先建立「源 id → 新 UUID」映射，再挂父子关系 ----
    let mut id_map: HashMap<String, String> = HashMap::new();
    for c in &parsed.categories {
        if !c.id.is_empty() {
            id_map.insert(c.id.clone(), new_id());
        }
    }

    // 目标项目现有分类的最大 sort_order（按父级分组），用于把导入分类追加到其后
    let existing_cats = db::get_categories(conn, project_id, mode).unwrap_or_default();
    let mut cat_seeds: HashMap<String, i32> = HashMap::new();
    for c in &existing_cats {
        let key = key_of(c.parent_id.clone());
        let cur = cat_seeds.get(&key).copied().unwrap_or(0);
        if c.sort_order > cur {
            cat_seeds.insert(key, c.sort_order);
        }
    }
    let mut cat_counters: HashMap<String, i32> = HashMap::new();
    for c in &parsed.categories {
        let new_id = match id_map.get(&c.id) {
            Some(x) => x.clone(),
            None => new_id(),
        };
        let parent_id = c.parent_id.as_ref().and_then(|p| id_map.get(p).cloned());
        let group_key = key_of(parent_id.clone());
        let order = next_order(&mut cat_counters, &cat_seeds, &group_key);
        let cat = Category {
            id: new_id,
            project_id: project_id.to_string(),
            name: empty_to(c.name.clone(), "未命名分类"),
            parent_id,
            sort_order: order,
            expanded: c.expanded,
            create_time: now.to_string(),
            update_time: now.to_string(),
            create_by: user_id.to_string(),
            update_by: user_id.to_string(),
            deleted: false,
            server_update_time: String::new(),
            sync_version: 1,
        };
        let _ = db::save_category(conn, &cat, mode);
    }

    // ---- 请求：分类归属按 id_map 改写，sortOrder 追加到目标分类末尾 ----
    let existing_reqs =
        db::get_all_saved_requests(conn, &[project_id.to_string()], mode, false).unwrap_or_default();
    let mut req_seeds: HashMap<String, i32> = HashMap::new();
    for r in &existing_reqs {
        let key = key_of(r.category_id.clone());
        let cur = req_seeds.get(&key).copied().unwrap_or(0);
        if r.sort_order > cur {
            req_seeds.insert(key, r.sort_order);
        }
    }
    let mut req_counters: HashMap<String, i32> = HashMap::new();
    for r in &parsed.requests {
        let category_id = r.category_id.as_ref().and_then(|cid| id_map.get(cid).cloned());
        let group_key = key_of(category_id.clone());
        let order = next_order(&mut req_counters, &req_seeds, &group_key);
        let req = SavedRequest {
            id: new_id(),
            user_id: user_id.to_string(),
            name: empty_to(r.name.clone(), "未命名接口"),
            method: empty_to(r.method.clone(), "GET").to_uppercase(),
            url: r.url.clone(),
            params: r.params.clone(),
            headers: r.headers.clone(),
            body_type: empty_to(r.body_type.clone(), "none"),
            body: r.body.clone(),
            form_body: r.form_body.clone(),
            category_id,
            project_id: Some(project_id.to_string()),
            pre_script: r.pre_script.clone(),
            post_script: r.post_script.clone(),
            sort_order: order,
            create_time: now.to_string(),
            create_by: user_id.to_string(),
            update_time: now.to_string(),
            update_by: user_id.to_string(),
            deleted: false,
            current_user_role: String::new(),
            server_update_time: String::new(),
            sync_version: 1,
        };
        let _ = db::save_saved_request(conn, &req, mode);
    }

    // ---- 环境 + 变量（导入出的环境不自动激活）----
    let mut env_count = 0;
    let mut env_var_count = 0;
    for env in &parsed.environments {
        let env_id = new_id();
        let e = Environment {
            id: env_id.clone(),
            project_id: Some(project_id.to_string()),
            name: empty_to(env.name.clone(), "未命名环境"),
            group_id: None,
            is_active: false,
            create_time: now.to_string(),
            create_by: user_id.to_string(),
            update_time: now.to_string(),
            update_by: user_id.to_string(),
            deleted: false,
            current_user_role: String::new(),
            server_update_time: String::new(),
            sync_version: 1,
        };
        let _ = db::save_environment(conn, &e, mode);
        env_count += 1;
        for (j, v) in env.variables.iter().enumerate() {
            if v.key.is_empty() {
                continue;
            }
            let var = EnvironmentVariable {
                id: new_id(),
                environment_id: env_id.clone(),
                key: v.key.clone(),
                value: v.value.clone(),
                enabled: v.enabled,
                sort_order: (j + 1) as i32,
                create_time: now.to_string(),
                create_by: user_id.to_string(),
                update_time: now.to_string(),
                update_by: user_id.to_string(),
                deleted: false,
                current_user_role: String::new(),
                server_update_time: String::new(),
                sync_version: 1,
            };
            let _ = db::save_env_variable(conn, &var, mode);
            env_var_count += 1;
        }
    }

    (
        parsed.categories.len() as i32,
        parsed.requests.len() as i32,
        env_count,
        env_var_count,
    )
}

// ============================================================
// Tauri 命令：解析 + 落库（+ 在线模式同步推送）
// ============================================================

#[tauri::command]
pub async fn import_collection(
    app: AppHandle,
    state: State<'_, AppDb>,
    #[allow(unused_variables)] file_name: String,
    content: String,
    project_id: Option<String>,
    data_mode: Option<String>,
) -> Result<ApiResult<serde_json::Value>, String> {
    let mode = data_mode
        .as_deref()
        .map(db::DataMode::from_str)
        .unwrap_or_default();

    let pid = match project_id.filter(|p| !p.trim().is_empty()) {
        Some(p) => p,
        None => return Ok(ApiResult::err(codes::PARAM_INVALID, "请先选择项目再导入".to_string())),
    };

    if content.trim().is_empty() {
        return Ok(ApiResult::err(codes::PARAM_INVALID, "文件内容为空".to_string()));
    }

    let parsed = match detect(&content) {
        Ok(p) => p,
        Err(e) => return Ok(ApiResult::err(codes::PARAM_INVALID, e)),
    };

    let now = db::now_timestamp();
    let user_id = {
        let app_data_dir = match app.path().app_data_dir() {
            Ok(d) => d,
            Err(e) => return Ok(ApiResult::err(codes::INTERNAL_ERROR, format!("获取数据目录失败: {e}"))),
        };
        infra::load_config(&app_data_dir).user_id
    };

    let (cats, reqs, envs, env_vars) =
        build_and_write(&state.conn, mode, &parsed, &pid, &user_id, &now);

    // 在线模式：导入数据已写入本地（dirty=1），触发一次同步把增量推到服务端并拉回最新。
    // 服务端 SyncService 会按 writableProjectIds 重新校验项目归属，不会越权写入。
    // 离线模式：仅落本地，不推送。
    if mode == db::DataMode::Online {
        let app_data_dir = match app.path().app_data_dir() {
            Ok(d) => d,
            Err(e) => return Ok(ApiResult::err(codes::INTERNAL_ERROR, format!("获取数据目录失败: {e}"))),
        };
        let mut config = infra::load_config(&app_data_dir);
        if config.auth_token.is_empty() {
            return Ok(ApiResult::err(codes::UNAUTHORIZED, "请先登录后再导入".to_string()));
        }
        if config.server_url.trim().is_empty() {
            return Ok(ApiResult::err(codes::INTERNAL_ERROR, "服务端地址未配置".to_string()));
        }
        let server_url = config.server_url.clone();
        let push = run_sync(&state.conn, &server_url, &mut config, db::DataMode::Online).await;
        if !push.is_success() {
            return Ok(ApiResult::err(
                push.code,
                format!("导入数据已写入本地，但同步到云端失败: {}", push.msg),
            ));
        }
        let _ = infra::save_config(&app_data_dir, &config);
    }

    let result = json!({
        "source": parsed.source,
        "imported": reqs,
        "skipped": 0,
        "categories": cats,
        "envImported": envs,
        "envVarImported": env_vars,
    });
    Ok(ApiResult::ok(result))
}
