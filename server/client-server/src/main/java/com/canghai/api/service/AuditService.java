package com.canghai.api.service;

import com.alibaba.fastjson2.JSON;
import com.alibaba.fastjson2.JSONObject;
import com.canghai.api.common.ApiResult;
import com.canghai.api.common.ErrorCode;
import com.canghai.api.dto.AuditQuery;
import com.canghai.api.entity.AuditLog;
import com.canghai.api.entity.User;
import com.baomidou.mybatisplus.core.conditions.query.QueryWrapper;
import com.canghai.api.mapper.AuditLogMapper;
import com.canghai.api.util.RequestContext;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.stereotype.Service;

import java.util.ArrayList;
import java.util.List;

/**
 * 操作审计日志服务
 */
@Service
public class AuditService extends BaseService {

    @Autowired
    private AuditLogMapper auditLogMapper;

    /**
     * 审计快照中需要脱敏的字段名（归一化后比较：小写 + 去掉 `_`/`-`）。
     * 请求头/请求体常含 token、密码、签名等凭证，落库前统一替换为 `***`。
     */
    private static final java.util.Set<String> SENSITIVE_KEYS = java.util.Set.of(
            "password", "passwd", "pwd", "token", "authtoken", "authorization", "cookie",
            "setcookie", "secret", "apikey", "accesskey", "accesskeyid", "accesskeysecret",
            "privatekey", "clientsecret", "refreshtoken");

    /** 单条快照 JSON 的体积上限（防止大响应体把审计表撑爆） */
    private static final int MAX_SNAPSHOT_LEN = 20_000;

    private static final String MASK = "***";

    /** 记录一条审计日志 */
    public void log(String projectId, User user, String action, String entityType, String entityId,
                    String entityName, Object before, Object after) {
        try {
            String beforeJson = toMaskedJson(before);
            String afterJson = toMaskedJson(after);

            AuditLog log = new AuditLog();
            log.setId(uuid());
            log.setProjectId(projectId);
            log.setUserId(user == null ? null : user.getId());
            log.setUsername(user == null ? "" : (user.getNickname() != null && !user.getNickname().isBlank() ? user.getNickname() : user.getUsername()));
            log.setAction(action);
            log.setEntityType(entityType);
            log.setEntityId(entityId);
            log.setEntityName(entityName);
            log.setBeforeJson(beforeJson);
            log.setAfterJson(afterJson);
            log.setIp(RequestContext.getClientIp());
            log.setCreateTime(now());
            log.setDeleted(false);
            auditLogMapper.insert(log);
        } catch (Exception e) {
            System.err.println("[ERROR] audit log failed: " + e.getClass().getName() + ": " + e.getMessage());
        }
    }

    /**
     * 生成审计快照 JSON：敏感字段脱敏 + 体积截断。
     *
     * <p>原先 `afterJson` 直接落完整请求体（含脚本、请求头中的 token/密码），
     * 既泄露凭证又可能写入超大字段；此处统一做脱敏与截断（Phase 8.7）。
     */
    private String toMaskedJson(Object o) {
        if (o == null) return null;
        String json;
        if (o instanceof String s) {
            json = s;
        } else {
            try {
                // 先序列化为 JSON 文本再解析回 JSONObject/JSONArray：
                // 直接对 POJO 调 JSON.toJSON 的返回类型不保证是 JSONObject，会导致遍历脱敏失效。
                Object parsed = JSON.parse(JSON.toJSONString(o));
                json = JSON.toJSONString(mask(parsed));
            } catch (Exception e) {
                return null;
            }
        }
        return json.length() > MAX_SNAPSHOT_LEN
                ? json.substring(0, MAX_SNAPSHOT_LEN) + "...(truncated)"
                : json;
    }

    /**
     * 递归遍历 JSON，对敏感内容打码。覆盖三种形态：
     * <ol>
     *   <li>字段名本身敏感（如 {@code {"token": "..."}}）；</li>
     *   <li>KV 形态：{@code {"key": "Authorization", "value": "..."}} —— 敏感信息在
     *       {@code key} 的**值**里，需据此打码同级的 {@code value}；</li>
     *   <li>值是「内嵌 JSON 字符串」（如请求体 {@code body: "{\"pwd\":\"...\"}"}），
     *       解析后递归打码再序列化回字符串。</li>
     * </ol>
     */
    private Object mask(Object node) {
        if (node instanceof JSONObject obj) {
            Object nameVal = obj.get("key");
            if (!(nameVal instanceof String)) nameVal = obj.get("name");
            if (nameVal instanceof String s && isSensitiveKey(s) && obj.containsKey("value")) {
                obj.put("value", MASK);
            }
            for (String k : new java.util.ArrayList<>(obj.keySet())) {
                Object v = obj.get(k);
                if (isSensitiveKey(k)) {
                    obj.put(k, MASK);
                } else if (v instanceof String str && looksLikeJson(str)) {
                    try {
                        obj.put(k, JSON.toJSONString(mask(JSON.parse(str))));
                    } catch (Exception ignored) {
                        // 非法 JSON：保持原样
                    }
                } else {
                    obj.put(k, mask(v));
                }
            }
            return obj;
        }
        if (node instanceof com.alibaba.fastjson2.JSONArray arr) {
            for (int i = 0; i < arr.size(); i++) {
                arr.set(i, mask(arr.get(i)));
            }
            return arr;
        }
        return node;
    }

    /** 判断字符串是否为「内嵌 JSON 对象/数组」文本 */
    private static boolean looksLikeJson(String s) {
        String t = s.trim();
        return (t.startsWith("{") && t.endsWith("}")) || (t.startsWith("[") && t.endsWith("]"));
    }

    private static boolean isSensitiveKey(String key) {
        if (key == null || key.isBlank()) return false;
        String k = key.toLowerCase(java.util.Locale.ROOT).replace("_", "").replace("-", "");
        return SENSITIVE_KEYS.contains(k);
    }

    /** 查询审计日志（按项目 + 可选实体类型/关键字/分页） */
    public ApiResult<?> query(AuditQuery q, User user) {
        if (q == null) q = new AuditQuery();
        String entityType = q.getEntityType();
        String entityId = q.getEntityId();
        String keyword = q.getKeyword();
        int page = q.getPage();
        int size = q.getSize();
        if (size <= 0) size = 20;
        if (size > 100) size = 100;

        String projectId = q.getProjectId();
        if (projectId == null || projectId.isBlank()) {
            return ApiResult.fail(ErrorCode.PARAM_INVALID, "缺少 projectId");
        }
        if (user == null) {
            return ApiResult.fail(ErrorCode.UNAUTHORIZED, "未登录");
        }
        // 校验当前用户确为该项目的成员（直接成员或经团队间接关联）
        if (!isProjectAccessible(projectId, user)) {
            return ApiResult.fail(ErrorCode.FORBIDDEN, "无权访问该项目审计日志");
        }

        com.baomidou.mybatisplus.core.conditions.query.QueryWrapper<AuditLog> query =
                qw(AuditLog.class).eq("project_id", projectId);
        if (entityType != null && !entityType.isBlank()) query.eq("entity_type", entityType);
        if (entityId != null && !entityId.isBlank()) query.eq("entity_id", entityId);
        if (keyword != null && !keyword.isBlank()) query.and(w -> w.like("entity_name", keyword).or().like("username", keyword));
        int safePage = Math.max(0, page);
        query.orderByDesc("create_time").orderByDesc("id").last("LIMIT " + size + " OFFSET " + (safePage * size));

        List<AuditLog> rows = auditLogMapper.selectList(query);
        List<JSONObject> list = new ArrayList<>();
        for (AuditLog r : rows) {
            JSONObject o = new JSONObject();
            o.put("id", r.getId());
            o.put("projectId", r.getProjectId());
            o.put("userId", r.getUserId());
            o.put("username", r.getUsername());
            o.put("action", r.getAction());
            o.put("entityType", r.getEntityType());
            o.put("entityId", r.getEntityId());
            o.put("entityName", r.getEntityName());
            o.put("beforeJson", r.getBeforeJson());
            o.put("afterJson", r.getAfterJson());
            o.put("createTime", r.getCreateTime());
            list.add(o);
        }
        JSONObject data = new JSONObject();
        data.put("list", list);
        data.put("page", page);
        data.put("size", size);
        return ApiResult.ok(data);
    }
}
