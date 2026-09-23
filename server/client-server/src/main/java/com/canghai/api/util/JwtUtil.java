package com.canghai.api.util;

import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.beans.factory.annotation.Value;
import org.springframework.stereotype.Component;

import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Paths;
import java.security.KeyFactory;
import java.security.PrivateKey;
import java.security.PublicKey;
import java.security.Signature;
import java.security.spec.PKCS8EncodedKeySpec;
import java.security.spec.X509EncodedKeySpec;
import java.time.Instant;
import java.util.Base64;
import java.util.HashMap;
import java.util.Map;

/**
 * 在线模式 JWT 工具（RS256 非对称，与 Rust 端 jwt.rs 共用同一密钥对）。
 *
 * <p>设计约束（满足"私钥在 Java 端、公钥在 Rust 端"）：
 * - 本类【仅 Java 端】持有 RSA 私钥，用于签发 token；
 * - Rust 端只持有对应公钥（resources/public.pem，编译期嵌入），仅用于验签，无法伪造；
 * - token 为无状态 JWT（header.payload.signature，RS256 签名），含 sub / username / team_id / iat / exp。
 *
 * <p>密钥来源：
 * - 私钥：环境变量 CANGHAI_JWT_PRIVATE_KEY（文件路径）优先，否则 classpath:keys/private.pem（已 gitignore，不入库）。
 * - 公钥：环境变量 CANGHAI_JWT_PUBLIC_KEY（文件路径）优先，否则 classpath:keys/public.pem。
 * 若私钥缺失，登录将明确失败（不再静默回退到旧 token 方案）。
 *
 * <p>注：JSON 编解码使用手写极简实现，输出标准 JSON，与 Rust 端 jsonwebtoken 解析兼容。
 */
@Component
public class JwtUtil {

    private static final Logger log = LoggerFactory.getLogger(JwtUtil.class);
    private static final String ALG = "RSA";
    private static final String SIGN_ALG = "SHA256withRSA";
    private static final long DEFAULT_TTL_SECONDS = 7L * 24 * 3600; // 7 天

    private final PrivateKey privateKey;
    private final PublicKey publicKey;

    public JwtUtil(
            @Value("${canghai.jwt.private-key-path:}") String privateKeyPath,
            @Value("${canghai.jwt.public-key-path:}") String publicKeyPath) {
        this.privateKey = loadPrivateKey(privateKeyPath);
        this.publicKey = loadPublicKey(publicKeyPath);
        if (this.privateKey == null) {
            log.error("[JwtUtil] RSA 私钥未初始化，JWT 签发不可用！请通过 CANGHAI_JWT_PRIVATE_KEY 或 classpath:keys/private.pem 提供私钥。");
        }
        if (this.publicKey == null) {
            log.error("[JwtUtil] RSA 公钥未初始化，JWT 验签不可用！请通过 CANGHAI_JWT_PUBLIC_KEY 或 classpath:keys/public.pem 提供公钥。");
        }
    }

    /** 私钥是否可用（用于 AuthService 判断是否可签发 JWT）。 */
    public boolean isEnabled() {
        return privateKey != null;
    }

    /**
     * 签发 JWT（RS256）。
     * @param userId   用户ID（写入 sub）
     * @param username 用户名
     * @return 完整 JWT 字符串
     */
    public String sign(String userId, String username) {
        if (privateKey == null) {
            throw new IllegalStateException("JWT 私钥未初始化，无法签发 token");
        }
        long now = Instant.now().getEpochSecond();
        long exp = now + DEFAULT_TTL_SECONDS;

        Map<String, Object> header = new HashMap<>();
        header.put("alg", "RS256");
        header.put("typ", "JWT");

        Map<String, Object> payload = new HashMap<>();
        payload.put("sub", userId);
        payload.put("username", username == null ? "" : username);
        payload.put("iat", now);
        payload.put("exp", exp);

        try {
            String headerB64 = base64Url(toJson(header));
            String payloadB64 = base64Url(toJson(payload));
            String signingInput = headerB64 + "." + payloadB64;

            Signature sig = Signature.getInstance(SIGN_ALG);
            sig.initSign(privateKey);
            sig.update(signingInput.getBytes(StandardCharsets.UTF_8));
            byte[] signature = sig.sign();

            return signingInput + "." + base64Url(signature);
        } catch (Exception e) {
            throw new RuntimeException("JWT 签发失败", e);
        }
    }

    /**
     * 解析并校验 JWT 签名（用内置公钥）。
     * @return 解析出的 claims（含 sub/team_id/exp 等），失败返回 null。
     */
    public Map<String, Object> verify(String token) {
        if (token == null || token.isBlank()) return null;
        String raw = token.trim();
        if (raw.startsWith("Bearer ")) raw = raw.substring(7).trim();
        String[] parts = raw.split("\\.");
        if (parts.length != 3) return null;
        if (publicKey == null) return null;

        try {
            String signingInput = parts[0] + "." + parts[1];
            byte[] signature = base64UrlDecode(parts[2]);

            Signature sig = Signature.getInstance(SIGN_ALG);
            sig.initVerify(publicKey);
            sig.update(signingInput.getBytes(StandardCharsets.UTF_8));
            if (!sig.verify(signature)) return null;

            String payloadJson = new String(base64UrlDecode(parts[1]), StandardCharsets.UTF_8);
            Map<String, Object> claims = parseJsonMap(payloadJson);
            Object expObj = claims.get("exp");
            if (expObj instanceof Number) {
                long exp = ((Number) expObj).longValue();
                if (Instant.now().getEpochSecond() > exp) return null;
            }
            return claims;
        } catch (Exception e) {
            return null;
        }
    }

    // —— 密钥加载 ——

    private PrivateKey loadPrivateKey(String pathEnv) {
        String pem = readPem(pathEnv, "keys/private.pem");
        if (pem == null) return null;
        try {
            byte[] der = pemToDer(pem, "PRIVATE KEY");
            PKCS8EncodedKeySpec spec = new PKCS8EncodedKeySpec(der);
            return KeyFactory.getInstance(ALG).generatePrivate(spec);
        } catch (Exception e) {
            log.error("[JwtUtil] 私钥解析失败: {}", e.getMessage());
            return null;
        }
    }

    private PublicKey loadPublicKey(String pathEnv) {
        String pem = readPem(pathEnv, "keys/public.pem");
        if (pem == null) return null;
        try {
            byte[] der = pemToDer(pem, "PUBLIC KEY");
            X509EncodedKeySpec spec = new X509EncodedKeySpec(der);
            return KeyFactory.getInstance(ALG).generatePublic(spec);
        } catch (Exception e) {
            log.error("[JwtUtil] 公钥解析失败: {}", e.getMessage());
            return null;
        }
    }

    private String readPem(String pathEnv, String classpath) {
        try {
            if (pathEnv != null && !pathEnv.isBlank()) {
                return Files.readString(Paths.get(pathEnv));
            }
            var res = JwtUtil.class.getClassLoader().getResourceAsStream(classpath);
            if (res != null) {
                return new String(res.readAllBytes(), StandardCharsets.UTF_8);
            }
        } catch (Exception e) {
            log.warn("[JwtUtil] 读取密钥失败 ({}): {}", classpath, e.getMessage());
        }
        return null;
    }

    // —— 工具方法（与 Rust jsonwebtoken 兼容的标准 JWT 编解码）——

    private static String base64Url(byte[] s) {
        return Base64.getUrlEncoder().withoutPadding().encodeToString(s);
    }

    private static String base64Url(String s) {
        return Base64.getUrlEncoder().withoutPadding().encodeToString(s.getBytes(StandardCharsets.UTF_8));
    }

    /** 极简 JSON 序列化（仅支持本工具生成的扁平 {k:v,...} 结构）。 */
    private static String toJson(Map<String, Object> map) {
        StringBuilder sb = new StringBuilder("{");
        boolean first = true;
        for (Map.Entry<String, Object> e : map.entrySet()) {
            if (!first) sb.append(",");
            first = false;
            sb.append('"').append(jsonEscape(e.getKey())).append("\":");
            Object v = e.getValue();
            if (v instanceof Number) {
                sb.append(v.toString());
            } else {
                sb.append('"').append(jsonEscape(String.valueOf(v))).append('"');
            }
        }
        sb.append("}");
        return sb.toString();
    }

    private static String jsonEscape(String s) {
        StringBuilder sb = new StringBuilder();
        for (int i = 0; i < s.length(); i++) {
            char c = s.charAt(i);
            switch (c) {
                case '"' -> sb.append("\\\"");
                case '\\' -> sb.append("\\\\");
                case '\n' -> sb.append("\\n");
                case '\r' -> sb.append("\\r");
                case '\t' -> sb.append("\\t");
                default -> {
                    if (c < 0x20) sb.append(String.format("\\u%04x", (int) c));
                    else sb.append(c);
                }
            }
        }
        return sb.toString();
    }

    private static byte[] base64UrlDecode(String s) {
        return Base64.getUrlDecoder().decode(s);
    }

    private static byte[] pemToDer(String pem, String type) {
        String clean = pem
                .replaceAll("-----BEGIN " + type + "-----", "")
                .replaceAll("-----END " + type + "-----", "")
                .replaceAll("\\s+", "");
        return Base64.getDecoder().decode(clean);
    }

    @SuppressWarnings("unchecked")
    private static Map<String, Object> parseJsonMap(String json) {
        Map<String, Object> map = new HashMap<>();
        String inner = json.trim();
        if (inner.startsWith("{")) inner = inner.substring(1);
        if (inner.endsWith("}")) inner = inner.substring(0, inner.length() - 1);
        for (String pair : splitTopLevel(inner)) {
            int idx = pair.indexOf(':');
            if (idx < 0) continue;
            String k = unquote(pair.substring(0, idx).trim());
            String v = pair.substring(idx + 1).trim();
            if (v.startsWith("\"") && v.endsWith("\"")) {
                map.put(k, unquote(v));
            } else if (v.matches("-?\\d+")) {
                map.put(k, Long.parseLong(v));
            } else if (v.matches("-?\\d+\\.\\d+")) {
                map.put(k, Double.parseDouble(v));
            } else {
                map.put(k, v);
            }
        }
        return map;
    }

    private static java.util.List<String> splitTopLevel(String s) {
        java.util.List<String> out = new java.util.ArrayList<>();
        int depth = 0, start = 0;
        for (int i = 0; i < s.length(); i++) {
            char c = s.charAt(i);
            if (c == '{' || c == '[') depth++;
            else if (c == '}' || c == ']') depth--;
            else if (c == ',' && depth == 0) {
                out.add(s.substring(start, i));
                start = i + 1;
            }
        }
        if (start < s.length()) out.add(s.substring(start));
        return out;
    }

    private static String unquote(String s) {
        if (s.length() >= 2 && s.startsWith("\"") && s.endsWith("\"")) {
            return s.substring(1, s.length() - 1);
        }
        return s;
    }
}
