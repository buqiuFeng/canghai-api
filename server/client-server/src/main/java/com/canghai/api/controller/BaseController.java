package com.canghai.api.controller;

import com.canghai.api.util.RequestBodyUtil;
import jakarta.servlet.http.HttpServletRequest;

/**
 * 控制器基类 — 提供请求体读取与上下文清理。
 *
 * <p>登录鉴权由 {@code AuthAspect} 切面统一处理，当前用户从 {@code RequestContext} 获取；
 * 异常统一由 {@link GlobalExceptionHandler} 收敛，控制器只负责参数绑定与委派，
 * 并在 finally 中清理 {@code RequestContext}（线程复用下防止用户信息泄漏）。
 */
public abstract class BaseController {

    /** 清理当前线程的请求上下文 */
    protected void clear() {
        com.canghai.api.util.RequestContext.clear();
    }

    /** 读取请求体（仅读取一次，缓存于 request attribute） */
    protected String readBody(HttpServletRequest req) {
        return RequestBodyUtil.readBody(req);
    }
}
