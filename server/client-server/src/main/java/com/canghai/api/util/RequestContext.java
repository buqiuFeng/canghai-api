package com.canghai.api.util;

import com.canghai.api.entity.User;

/**
 * 请求上下文 — 通过 ThreadLocal 在当前请求线程内透传轻量元数据（客户端 IP、当前登录用户）。
 * 必须在请求结束时 {@link #clear()} 以避免线程复用导致的信息串号。
 */
public final class RequestContext {

    private static final ThreadLocal<String> CLIENT_IP = new ThreadLocal<>();
    private static final ThreadLocal<User> CURRENT_USER = new ThreadLocal<>();

    private RequestContext() {}

    public static void setClientIp(String ip) {
        CLIENT_IP.set(ip == null ? "" : ip);
    }

    public static String getClientIp() {
        String ip = CLIENT_IP.get();
        return ip == null ? "" : ip;
    }

    /** 设置当前登录用户（由鉴权切面注入） */
    public static void setUser(User user) {
        CURRENT_USER.set(user);
    }

    /** 获取当前登录用户（未登录为 null） */
    public static User getUser() {
        return CURRENT_USER.get();
    }

    public static void clear() {
        CLIENT_IP.remove();
        CURRENT_USER.remove();
    }
}
