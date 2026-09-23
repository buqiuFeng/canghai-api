package com.canghai.api.aspect;

import com.canghai.api.common.ApiResult;
import com.canghai.api.common.ErrorCode;
import com.canghai.api.config.AuthProperties;
import com.canghai.api.entity.User;
import com.canghai.api.service.AuthService;
import com.canghai.api.util.RequestBodyUtil;
import com.canghai.api.util.RequestContext;
import jakarta.servlet.http.HttpServletRequest;
import org.aspectj.lang.ProceedingJoinPoint;
import org.aspectj.lang.annotation.Around;
import org.aspectj.lang.annotation.Aspect;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.stereotype.Component;
import org.springframework.util.AntPathMatcher;
import org.springframework.web.context.request.RequestContextHolder;
import org.springframework.web.context.request.ServletRequestAttributes;

/**
 * 登录认证切面 — 对所有 @RestController 方法统一鉴权。
 *
 * <p>流程：从请求（body / query / header）提取 token → 校验得到用户 → 注入 {@link RequestContext}；
 * 若请求路径不在免登录白名单且用户为空，则直接返回 401，不再进入控制器方法。
 * 白名单通过 {@link AuthProperties} 配置（application.yml: canghai.auth.whitelist）。
 */
@Aspect
@Component
public class AuthAspect {

    private static final AntPathMatcher ANT = new AntPathMatcher();

    @Autowired
    private AuthService authService;
    @Autowired
    private AuthProperties authProperties;

    @Around("within(@org.springframework.web.bind.annotation.RestController *)")
    public Object around(ProceedingJoinPoint pjp) throws Throwable {
        ServletRequestAttributes attrs = (ServletRequestAttributes) RequestContextHolder.getRequestAttributes();
        if (attrs == null) {
            return pjp.proceed();
        }
        HttpServletRequest req = attrs.getRequest();

        // 透传客户端 IP（供审计日志使用），原 BaseController.prepare 逻辑下沉至此
        String ip = req.getHeader("X-Forwarded-For");
        if (ip == null || ip.isBlank()) {
            ip = req.getRemoteAddr();
        } else {
            ip = ip.split(",")[0].trim();
        }
        RequestContext.setClientIp(ip);

        String body = RequestBodyUtil.readBody(req);
        String token = RequestBodyUtil.extractToken(req, body);
        User user = (token != null && !token.isBlank()) ? authService.validateToken(token) : null;
        RequestContext.setUser(user);

        String uri = req.getRequestURI();
        boolean whitelisted = authProperties.getWhitelist().stream()
                .anyMatch(pattern -> ANT.match(pattern, uri));

        if (!whitelisted && user == null) {
            return ApiResult.fail(ErrorCode.UNAUTHORIZED);
        }

        try {
            return pjp.proceed();
        } finally {
            RequestContext.clear();
        }
    }
}
