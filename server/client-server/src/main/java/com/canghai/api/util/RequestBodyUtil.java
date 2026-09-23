package com.canghai.api.util;

import com.alibaba.fastjson2.JSONObject;
import jakarta.servlet.http.HttpServletRequest;

import java.io.BufferedReader;

/**
 * 请求体工具 — 读取请求体（仅一次，缓存于 request attribute）并提取 token。
 * 供控制器与鉴权切面共用，避免重复消费输入流。
 */
public final class RequestBodyUtil {

    private RequestBodyUtil() {}

    /** 读取请求体（仅读取一次，缓存于 request attribute "__body"） */
    public static String readBody(HttpServletRequest req) {
        String cached = (String) req.getAttribute("__body");
        if (cached != null) return cached;
        StringBuilder sb = new StringBuilder();
        try {
            req.setCharacterEncoding("UTF-8");
            BufferedReader reader = req.getReader();
            String line;
            while ((line = reader.readLine()) != null) {
                sb.append(line);
            }
        } catch (Exception ignored) {
        }
        String body = sb.toString();
        req.setAttribute("__body", body);
        return body;
    }

    /** 从 body / query / header 中提取 token */
    public static String extractToken(HttpServletRequest req, String body) {
        if (body != null && !body.isBlank()) {
            try {
                JSONObject o = JSONObject.parseObject(body);
                String t = o.getString("token");
                if (t != null && !t.isBlank()) return t;
            } catch (Exception ignored) {
            }
        }
        String p = req.getParameter("token");
        if (p != null && !p.isBlank()) return p;
        String h = req.getHeader("Authorization");
        if (h != null) {
            if (h.toLowerCase().startsWith("bearer ")) return h.substring(7).trim();
            return h.trim();
        }
        String x = req.getHeader("X-Token");
        if (x != null && !x.isBlank()) return x;
        return null;
    }
}
