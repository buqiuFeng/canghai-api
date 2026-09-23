//! 在线模式 JWT 验证模块（仅验签，不签发）。
//!
//! 设计要点（满足"私钥在 Java 端、公钥在 Rust 端"的约束）：
//! - Java 端持有 RSA 私钥，用 RS256 签发 token；
//! - Rust 端内置 `resources/public.pem`（公钥），只用它验签，无法伪造 token；
//! - 验证通过后可拿到 `sub`(用户ID)、`team_id`、`username` 等声明，
//!   用于本地数据按用户/团队过滤，以及转发 Java 时携带。
//!
//! 兼容性：本模块只负责 JWT 验证。项目同时保留旧的"随机 token + ch_user_tokens 表"
//! 有状态方案（由 Java 端通过 body.token 验证）。前端传入的 token 若为 JWT，
//! 走本模块验签；若为旧随机 token，则直接透传给 Java 端由其校验。

use jsonwebtoken::{decode, decode_header, DecodingKey, Validation, Algorithm};
use serde::{Deserialize, Serialize};

/// JWT 载荷声明。字段命名与 Java 端签发保持一致。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    /// 用户ID（Java 端用 `sub` 承载，jjwt 默认映射）。
    #[serde(default)]
    pub sub: String,
    /// 用户名。
    #[serde(default)]
    pub username: String,
    /// 团队ID（在线模式下本地数据按团队/用户归属过滤）。
    #[serde(default = "default_team_id")]
    pub team_id: String,
    /// 过期时间（jjwt 标准 `exp` 字段，unix 秒）。
    #[serde(default)]
    pub exp: usize,
}

fn default_team_id() -> String {
    "default".to_string()
}

/// 验证结果。若 token 为 JWT 且验签成功则返回 claims；否则返回 None（交由 Java 端处理旧 token）。
#[derive(Debug, Clone)]
pub enum TokenVerification {
    /// JWT 验签成功，携带解析出的声明。
    Jwt(JwtClaims),
    /// 非 JWT（或无法用本地公钥验签），需透传给 Java 端用旧方案校验。
    Passthrough,
    /// 是 JWT 结构但验签失败（如过期、被篡改），直接拒绝。
    Invalid(String),
}

/// 判断字符串是否为 JWT（三段式 base64url，由 `header.payload.signature` 组成）。
fn looks_like_jwt(token: &str) -> bool {
    let trimmed = token.trim();
    if trimmed.is_empty() {
        return false;
    }
    // 去除可能的 Bearer 前缀
    let raw = trimmed.strip_prefix("Bearer ").unwrap_or(trimmed);
    let parts: Vec<&str> = raw.split('.').collect();
    parts.len() == 3 && parts.iter().all(|p| !p.is_empty())
}

/// 用内置公钥验证 JWT。
///
/// - 若 token 不是 JWT 形态 → `Passthrough`（交给 Java 端旧逻辑）。
/// - 若是 JWT 但验签失败 → `Invalid(reason)`（拒绝，不再透传，避免伪造 token 打到后端）。
/// - 若验签成功 → `Jwt(claims)`。
pub fn verify_token(token: &str) -> TokenVerification {
    let trimmed = token.trim();
    let raw = trimmed.strip_prefix("Bearer ").unwrap_or(trimmed);

    if !looks_like_jwt(raw) {
        return TokenVerification::Passthrough;
    }

    // 先尝试解析 header，确认算法为 RS256（防止 alg=none 等降级攻击）。
    let header = match decode_header(raw) {
        Ok(h) => h,
        Err(e) => return TokenVerification::Invalid(format!("无法解析 JWT 头: {e}")),
    };
    if header.alg != Algorithm::RS256 {
        return TokenVerification::Invalid(format!(
            "不支持的 JWT 算法: {:?}，仅允许 RS256",
            header.alg
        ));
    }

    // 从内置公钥文件加载 DecodingKey（编译期嵌入，无需运行时读取磁盘）。
    let public_pem = include_str!("../resources/public.pem");
    let decoding_key = match DecodingKey::from_rsa_pem(public_pem.as_bytes()) {
        Ok(k) => k,
        Err(e) => {
            // 公钥文件缺失/非法（如仍是占位符）时，不应放行伪造 token，
            // 但也不应阻断旧 token 透传——这里返回 Invalid 以便上层诊断。
            return TokenVerification::Invalid(format!("加载验签公钥失败: {e}"));
        }
    };

    let mut validation = Validation::new(Algorithm::RS256);
    validation.validate_exp = true;
    // 仅验证签名与过期，不约束 audience/issuer（保持对 Java 端签发灵活）。

    match decode::<JwtClaims>(raw, &decoding_key, &validation) {
        Ok(data) => TokenVerification::Jwt(data.claims),
        Err(e) => TokenVerification::Invalid(format!("JWT 验签失败: {e}")),
    }
}

/// 便捷：从 token 中提取 userId（仅当为有效 JWT 时）。
pub fn extract_user_id(token: &str) -> Option<String> {
    match verify_token(token) {
        TokenVerification::Jwt(claims) if !claims.sub.is_empty() => Some(claims.sub),
        _ => None,
    }
}
