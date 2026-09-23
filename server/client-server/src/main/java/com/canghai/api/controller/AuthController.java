package com.canghai.api.controller;

import com.canghai.api.common.ApiResult;
import com.canghai.api.dto.req.LoginRequest;
import com.canghai.api.dto.req.TokenRequest;
import com.canghai.api.service.AuthService;
import com.canghai.api.util.RequestBodyUtil;
import jakarta.servlet.http.HttpServletRequest;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.web.bind.annotation.PostMapping;
import org.springframework.web.bind.annotation.RequestBody;
import org.springframework.web.bind.annotation.RequestMapping;
import org.springframework.web.bind.annotation.RestController;

/**
 * 认证路由：/api/v1/auth/**
 * 该路由无需登录（login/register），由 AuthService 内部自行处理 token 校验。
 */
@RestController
@RequestMapping("/api/v1/auth")
public class AuthController extends BaseController {

    @Autowired
    private AuthService authService;

    /**
     * 注册
     * POST /api/v1/auth/register
     */
    @PostMapping("/register")
    public ApiResult<?> register(@RequestBody(required = false) LoginRequest q) {
        try {
            return authService.register(q);
        } finally {
            clear();
        }
    }

    /**
     * 登录
     * POST /api/v1/auth/login
     */
    @PostMapping("/login")
    public ApiResult<?> login(@RequestBody(required = false) LoginRequest q) {
        try {
            return authService.login(q);
        } finally {
            clear();
        }
    }

    /**
     * 登出（无状态 JWT，服务端无需处理）
     * POST /api/v1/auth/logout
     */
    @PostMapping("/logout")
    public ApiResult<?> logout() {
        try {
            return authService.logout();
        } finally {
            clear();
        }
    }

    /**
     * 当前登录用户
     * POST /api/v1/auth/me
     * token 优先取请求体，缺省时回退到 query / Authorization / X-Token。
     */
    @PostMapping("/me")
    public ApiResult<?> me(HttpServletRequest req, @RequestBody(required = false) TokenRequest q) {
        try {
            String token = q != null ? q.getToken() : null;
            if (token == null || token.isBlank()) {
                token = RequestBodyUtil.extractToken(req, readBody(req));
            }
            return authService.me(token);
        } finally {
            clear();
        }
    }
}
