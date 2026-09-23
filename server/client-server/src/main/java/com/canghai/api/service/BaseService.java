package com.canghai.api.service;

import com.alibaba.fastjson2.JSON;
import com.canghai.api.common.ApiException;
import com.canghai.api.common.ApiResult;
import com.canghai.api.common.ErrorCode;
import com.canghai.api.entity.User;
import com.baomidou.mybatisplus.core.conditions.query.QueryWrapper;
import com.baomidou.mybatisplus.core.conditions.update.UpdateWrapper;
import com.baomidou.mybatisplus.core.mapper.BaseMapper;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.context.annotation.Lazy;

import java.time.LocalDateTime;
import java.time.ZoneId;
import java.time.format.DateTimeFormatter;
import java.util.List;

/**
 * Service 基类 — 提供 JSON 解析、时间/UUID、路由解析、软删除、权限校验、审计等公共能力。
 */
public abstract class BaseService {

    private static final DateTimeFormatter DTF = DateTimeFormatter.ofPattern("yyyy-MM-dd HH:mm:ss");

    // @Lazy 打断循环依赖：BaseService 注入子类 AuditService/ProjectMemberService，
    // 而二者均 extends BaseService。@Lazy 注入代理，避免初始化期互相等待，
    // 因此无需开启 spring.main.allow-circular-references。
    @Autowired
    @Lazy
    protected AuditService auditService;
    @Autowired
    @Lazy
    protected ProjectMemberService projectMemberService;

    /** 解析请求体为指定类型 */
    protected <T> T parseBody(String body, Class<T> type) {
        if (body == null || body.isBlank()) return null;
        try {
            return JSON.parseObject(body, type);
        } catch (Exception e) {
            return null;
        }
    }

    /** 解析请求体为 List<T> */
    protected <T> List<T> parseList(String json, Class<T> elementType) {
        if (json == null || json.isBlank()) return List.of();
        try {
            return JSON.parseArray(json, elementType);
        } catch (Exception e) {
            return List.of();
        }
    }

    /** 生成 UUID */
    protected String uuid() {
        return java.util.UUID.randomUUID().toString();
    }

    /** 应用统一时区：Asia/Shanghai（UTC+8，无夏令时，固定偏移） */
    private static final ZoneId APP_ZONE = ZoneId.of("Asia/Shanghai");

    /**
     * 当前时间（**Asia/Shanghai / UTC+8**，yyyy-MM-dd HH:mm:ss）。
     *
     * <p>三端时间基准统一为北京时间：MySQL 连接会话时区由连接串钉在 Asia/Shanghai，
     * {@code server_update_time} 与同步游标同源；Rust {@code db::now_timestamp()}、
     * 前端 {@code utils.now()} 亦输出北京时间。
     *
     * <p>此前这里取 UTC、而 {@code CURRENT_TIMESTAMP} 取数据库时区，业务列与
     * {@code server_update_time} 会相差一个时区，同步冲突判定随之错乱。
     */
    protected String now() {
        return LocalDateTime.now(APP_ZONE).format(DTF);
    }

    /** 软删除指定记录 */
    protected <T> int softDelete(BaseMapper<T> mapper, String id) {
        return mapper.update(null, new UpdateWrapper<T>().eq("id", id).set("deleted", 1).set("update_time", now()));
    }

    /**
     * 项目归属校验（修复越权访问）。
     * projectId 为空表示「个人空间」，放行；否则要求当前用户对该项目可见。
     *
     * <p>注意：Service 为单例，用户必须通过参数显式传入，禁止用实例字段保存请求态。
     */
    protected boolean isProjectAccessible(String projectId, User user) {
        if (projectId == null || projectId.isBlank()) return true;
        if (user == null) return false;
        return projectMemberService.getVisibleProjectIds(user).contains(projectId);
    }

    /**
     * 按记录 id 反查所属 projectId 并校验可见性。
     * @return 所属 projectId（可能为空串=个人空间）；越权或记录不存在返回特殊标记 "-"
     */
    protected String resolveAndCheckProject(String projectId, String id, BaseMapper<?> mapper, User user) {
        if (id == null || id.isBlank()) return "-";
        Object entity = mapper.selectById(id);
        if (entity == null) return "-";
        try {
            java.lang.reflect.Method m = entity.getClass().getMethod("getProjectId");
            Object pid = m.invoke(entity);
            String p = pid == null ? "" : pid.toString();
            if (!isProjectAccessible(p, user)) return "-";
            return p;
        } catch (Exception e) {
            return "-";
        }
    }

    /**
     * 记录审计日志（写入失败不影响主流程）；操作人由参数显式传入。
     *
     * <p>此重载不带变更前快照（创建类操作适用）；更新/删除请用带 {@code before} 的重载，
     * 并用 {@link #snapshot} 在真正写库**之前**取快照。
     */
    protected void audit(String projectId, String action, String type, String id, String name, Object after, User user) {
        audit(projectId, action, type, id, name, null, after, user);
    }

    /** 记录审计日志（含变更前快照，供「对比变更前后值」使用） */
    protected void audit(String projectId, String action, String type, String id, String name,
                         Object before, Object after, User user) {
        try {
            auditService.log(projectId, user, action, type, id, name, before, after);
        } catch (Exception ignored) {
        }
    }

    /**
     * 读取实体的「变更前快照」，用于审计 beforeJson。
     *
     * <p>必须在执行 update/softDelete **之前**调用（否则读到的是变更后的值）。
     * 记录不存在或反射失败时返回 null，不阻断主流程。
     */
    protected Object snapshot(BaseMapper<?> mapper, String id) {
        if (mapper == null || id == null || id.isBlank()) return null;
        try {
            return mapper.selectById(id);
        } catch (Exception e) {
            return null;
        }
    }

    /** 抛出统一业务异常（错误码必须来自 {@link ErrorCode}，禁止散用字面量） */
    protected ApiResult<?> fail(ErrorCode code, String msg) {
        throw new ApiException(code.getCode(), msg == null || msg.isBlank() ? code.getMessage() : msg);
    }

    /**
     * 构造 MyBatis-Plus 查询包装器。
     * 传入实体 Class 以固定泛型类型，避免链式调用时菱形推断退化为
     * QueryWrapper&lt;Object&gt; 导致与 Wrapper&lt;Entity&gt; 不兼容的编译错误。
     */
    protected <T> QueryWrapper<T> qw(Class<T> type) {
        return new QueryWrapper<>();
    }
}
