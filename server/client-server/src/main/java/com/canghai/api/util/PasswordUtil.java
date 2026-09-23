package com.canghai.api.util;

import java.security.SecureRandom;
import java.security.spec.KeySpec;
import java.util.Arrays;
import java.util.Base64;

import javax.crypto.SecretKeyFactory;
import javax.crypto.spec.PBEKeySpec;

/**
 * 密码工具 — PBKDF2-HMAC-SHA256（10 万次迭代 + 随机盐）。
 * 兼容旧格式（salt:hash 的 SHA-256）以便存量数据迁移。
 */
public final class PasswordUtil {

    private static final int ITERATIONS = 100_000;
    private static final int KEY_LENGTH = 256; // bits
    private static final SecureRandom RANDOM = new SecureRandom();

    private PasswordUtil() {}

    /** 哈希密码，格式: pbkdf2$iterations$saltB64$hashB64 */
    public static String hashPassword(String password) {
        try {
            byte[] salt = new byte[16];
            RANDOM.nextBytes(salt);
            KeySpec spec = new PBEKeySpec(password.toCharArray(), salt, ITERATIONS, KEY_LENGTH);
            SecretKeyFactory skf = SecretKeyFactory.getInstance("PBKDF2WithHmacSHA256");
            byte[] hash = skf.generateSecret(spec).getEncoded();
            return "pbkdf2$" + ITERATIONS + "$"
                    + Base64.getEncoder().encodeToString(salt) + "$"
                    + Base64.getEncoder().encodeToString(hash);
        } catch (Exception e) {
            throw new RuntimeException("密码哈希失败", e);
        }
    }

    /** 校验密码：兼容 PBKDF2 新格式与旧 SHA-256（salt:hash）格式 */
    public static boolean verifyPassword(String password, String storedHash) {
        if (storedHash == null || storedHash.isBlank()) return false;
        try {
            if (!storedHash.startsWith("pbkdf2$")) {
                String[] parts = storedHash.split(":", 2);
                if (parts.length != 2) return false;
                byte[] salt = Base64.getDecoder().decode(parts[0]);
                java.security.MessageDigest md = java.security.MessageDigest.getInstance("SHA-256");
                md.update(salt);
                byte[] hash = md.digest(password.getBytes("UTF-8"));
                return Base64.getEncoder().encodeToString(hash).equals(parts[1]);
            }
            String[] parts = storedHash.split("\\$", 4);
            if (parts.length != 4) return false;
            int iterations = Integer.parseInt(parts[1]);
            byte[] salt = Base64.getDecoder().decode(parts[2]);
            byte[] expected = Base64.getDecoder().decode(parts[3]);
            KeySpec spec = new PBEKeySpec(password.toCharArray(), salt, iterations, KEY_LENGTH);
            SecretKeyFactory skf = SecretKeyFactory.getInstance("PBKDF2WithHmacSHA256");
            byte[] hash = skf.generateSecret(spec).getEncoded();
            return Arrays.equals(hash, expected);
        } catch (Exception e) {
            return false;
        }
    }
}
