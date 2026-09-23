package com.canghai.api.service;

import com.alibaba.fastjson2.JSONObject;
import com.baomidou.mybatisplus.core.conditions.query.QueryWrapper;
import com.canghai.api.common.ApiResult;
import com.canghai.api.common.ErrorCode;
import com.canghai.api.dto.req.LoginRequest;
import com.canghai.api.dto.resp.LoginResponse;
import com.canghai.api.entity.Team;
import com.canghai.api.entity.User;
import com.canghai.api.mapper.TeamMapper;
import com.canghai.api.mapper.TeamMemberMapper;
import com.canghai.api.mapper.UserMapper;
import com.canghai.api.util.JwtUtil;
import com.canghai.api.util.PasswordUtil;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.stereotype.Service;
import org.springframework.transaction.annotation.Transactional;

/**
 * 认证服务 — 注册、登录、登出、令牌验证。
 * 在线模式使用无状态 JWT（RS256 非对称，与 Rust 端共用同一密钥对），token 含 sub / team_id / exp。
 *
 * POST /api/auth/register
 * POST /api/auth/login
 * POST /api/auth/logout
 * POST /api/auth/me
 */
@Service
public class AuthService extends BaseService {

    private static final long TOKEN_EXPIRY_MS = 7L * 24 * 3600 * 1000; // 7 天

    /** token → 用户 的短 TTL 缓存（见 validateToken 注释） */
    private static final long TOKEN_CACHE_TTL_MS = 60_000L; // 60s
    private static final int TOKEN_CACHE_MAX = 1000;
    private final java.util.concurrent.ConcurrentHashMap<String, CachedUser> TOKEN_CACHE =
            new java.util.concurrent.ConcurrentHashMap<>();

    /** 缓存条目：用户 + 过期时间戳（毫秒） */
    private record CachedUser(User user, long expireAt) {}

    @Autowired
    private UserMapper userMapper;
    @Autowired
    private TeamMapper teamMapper;
    @Autowired
    private TeamMemberMapper teamMemberMapper;
    @Autowired
    private JwtUtil jwtUtil;


    /** 注册：用户 + 默认工作区 + owner 成员三次写入必须原子，避免产生无工作区的孤儿用户 */
    @Transactional(rollbackFor = Exception.class)
    public ApiResult<LoginResponse> register(LoginRequest req) {
        if (req == null || req.getUsername() == null || req.getUsername().isBlank()) {
            return ApiResult.fail(ErrorCode.PARAM_MISSING, "用户名不能为空");
        }
        if (req.getPassword() == null || req.getPassword().length() < 6) {
            return ApiResult.fail(ErrorCode.PARAM_INVALID, "密码长度不能少于 6 位");
        }
        String username = req.getUsername().trim();
        String nickname = (req.getNickname() != null && !req.getNickname().isBlank()) ? req.getNickname().trim() : username;

        Long count = userMapper.selectCount(
                qw(User.class)
                        .eq("username", username).eq("deleted", 0));
        if (count != null && count > 0) {
            return ApiResult.fail(ErrorCode.CONFLICT, "用户名已被注册");
        }

        String now = now();
        String userId = uuid();
        User user = new User();
        user.setId(userId);
        user.setUsername(username);
        user.setNickname(nickname);
        user.setEmail(req.getEmail() != null ? req.getEmail().trim() : "");
        user.setPasswordHash(PasswordUtil.hashPassword(req.getPassword()));
        user.setCreateTime(now);
        user.setCreateBy(userId);
        user.setUpdateTime(now);
        user.setUpdateBy(userId);
        user.setDeleted(false);
        userMapper.insert(user);


        return ApiResult.ok(generateLoginResponse(userId, username, nickname, user.getEmail(), now));
    }

    public ApiResult<LoginResponse> login(LoginRequest req) {
    
        if (req == null || req.getUsername() == null || req.getUsername().isBlank()) {
            return ApiResult.fail(ErrorCode.PARAM_MISSING, "用户名不能为空");
        }
        if (req.getPassword() == null || req.getPassword().isBlank()) {
            return ApiResult.fail(ErrorCode.PARAM_MISSING, "密码不能为空");
        }
        User user = userMapper.selectOne(
                qw(User.class)
                        .eq("username", req.getUsername().trim()).eq("deleted", 0));
        if (user == null) {
            return ApiResult.fail(ErrorCode.BAD_CREDENTIALS);
        }
        if (!PasswordUtil.verifyPassword(req.getPassword(), user.getPasswordHash())) {
            return ApiResult.fail(ErrorCode.BAD_CREDENTIALS);
        }
        String now = now();
        return ApiResult.ok(generateLoginResponse(user.getId(), user.getUsername(), user.getNickname(), user.getEmail(), now));
    }

    /** 无状态 JWT：客户端丢弃 token 即可；此处仅作兼容返回成功 */
    public ApiResult<?> logout() {
        return ApiResult.ok();
    }

    /** 按令牌查询当前登录用户 */
    public ApiResult<User> me(String token) {
        User user = validateToken(token);
        if (user == null) {
            return ApiResult.fail(ErrorCode.TOKEN_EXPIRED);
        }
        return ApiResult.ok(user);
    }

    /** 验证 token 并返回用户（无效返回 null） */
    public User validateToken(String token) {
        if (token == null || token.isBlank()) return null;

        // 短 TTL 缓存：AuthAspect 对每个请求都会调用本方法查库，高频请求下会放大 DB 负载。
        // JWT 验签本身是 CPU 计算，真正的瓶颈是「每次 selectOne 用户」。缓存 60s 可让
        // 同一 token 的连续请求只查一次库（§7.4）。登出后最多滞后 60s，属可接受折衷。
        String cacheKey = token;
        CachedUser cached = TOKEN_CACHE.get(cacheKey);
        if (cached != null && cached.expireAt() > System.currentTimeMillis()) {
            return cached.user();
        }
        if (cached != null) {
            TOKEN_CACHE.remove(cacheKey);
        }

        java.util.Map<String, Object> claims = jwtUtil.verify(token);
        if (claims == null) return null;
        Object sub = claims.get("sub");
        if (sub == null || sub.toString().isBlank()) return null;
        User user = userMapper.selectOne(
                qw(User.class)
                        .eq("id", sub.toString()).eq("deleted", 0));
        if (user != null) {
            // 防内存膨胀：超过上限时清理过期项（单实例桌面/团队场景，1000 条足够）
            if (TOKEN_CACHE.size() >= TOKEN_CACHE_MAX) {
                purgeExpiredTokens();
            }
            TOKEN_CACHE.put(cacheKey, new CachedUser(user, System.currentTimeMillis() + TOKEN_CACHE_TTL_MS));
        }
        return user;
    }

    /** 清理过期 token 缓存项 */
    private void purgeExpiredTokens() {
        long now = System.currentTimeMillis();
        TOKEN_CACHE.entrySet().removeIf(e -> e.getValue().expireAt() <= now);
    }

    public String extractToken(String body) {
        if (body == null || body.isBlank()) return null;
        try {
            JSONObject obj = JSONObject.parseObject(body);
            String token = obj.getString("token");
            return (token != null && !token.isBlank()) ? token : null;
        } catch (Exception ignored) {
            return null;
        }
    }

    private LoginResponse generateLoginResponse(String userId, String username, String nickname, String email, String now) {


        String token = jwtUtil.sign(userId, username);
        long expiresAt = System.currentTimeMillis() + TOKEN_EXPIRY_MS;

        User safeUser = new User();
        safeUser.setId(userId);
        safeUser.setUsername(username);
        safeUser.setNickname(nickname != null ? nickname : username);
        safeUser.setEmail(email != null ? email : "");

        LoginResponse resp = new LoginResponse();
        resp.setToken(token);
        resp.setUser(safeUser);
        resp.setExpiresAt(expiresAt);
        return resp;
    }
}
