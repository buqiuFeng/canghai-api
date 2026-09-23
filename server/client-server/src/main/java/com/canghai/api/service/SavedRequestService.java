package com.canghai.api.service;

import com.canghai.api.common.ApiResult;
import com.canghai.api.common.ErrorCode;
import com.canghai.api.dto.req.IdRequest;
import com.canghai.api.dto.req.ProjectIdRequest;
import com.canghai.api.entity.SavedRequest;
import com.canghai.api.entity.User;
import com.canghai.api.mapper.SavedRequestMapper;
import com.baomidou.mybatisplus.core.conditions.update.UpdateWrapper;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.stereotype.Service;

import java.util.List;

/**
 * 保存的接口请求管理。路由前缀：/api/request
 */
@Service
public class SavedRequestService extends BaseService {

    @Autowired
    private SavedRequestMapper savedRequestMapper;


    public ApiResult<List<SavedRequest>> list(ProjectIdRequest req, User user) {
        String projectId = req != null ? req.getProjectId() : null;
        if (projectId == null || projectId.isBlank()) return ApiResult.fail(ErrorCode.PARAM_INVALID, "缺少 projectId");
        if (!isProjectAccessible(projectId, user)) {
            return ApiResult.fail(ErrorCode.FORBIDDEN, "无权访问该项目");
        }
        List<SavedRequest> rows = savedRequestMapper.selectList(
                qw(SavedRequest.class).eq("project_id", projectId).eq("deleted", 0)
                        .orderByAsc("sort_order").orderByAsc("name"));
        return ApiResult.ok(rows);
    }

    public ApiResult<SavedRequest> detail(IdRequest req, User user) {
        String id = req != null ? req.getId() : null;
        if (id == null || id.isBlank()) return ApiResult.fail(ErrorCode.PARAM_INVALID, "缺少 id");
        SavedRequest r = savedRequestMapper.selectOne(
                qw(SavedRequest.class).eq("id", id).eq("deleted", 0));
        if (r == null) return ApiResult.fail(ErrorCode.NOT_FOUND, "接口不存在");
        if (!isProjectAccessible(r.getProjectId(), user)) {
            return ApiResult.fail(ErrorCode.FORBIDDEN, "无权访问该项目");
        }
        return ApiResult.ok(r);
    }

    public ApiResult<SavedRequest> create(SavedRequest req, User user) {
        if (req == null || req.getProjectId() == null || req.getName() == null) {
            return ApiResult.fail(ErrorCode.PARAM_INVALID, "缺少必填字段 projectId/name");
        }
        if (!isProjectAccessible(req.getProjectId(), user)) {
            return ApiResult.fail(ErrorCode.FORBIDDEN, "无权访问该项目");
        }
        String now = now();
        String id = (req.getId() != null && !req.getId().isBlank()) ? req.getId() : uuid();
        SavedRequest r = new SavedRequest();
        r.setId(id);
        r.setProjectId(req.getProjectId());
        r.setName(req.getName());
        r.setMethod(req.getMethod() != null ? req.getMethod() : "GET");
        r.setUrl(req.getUrl() != null ? req.getUrl() : "");
        r.setParams(req.getParams());
        r.setHeaders(req.getHeaders());
        r.setBodyType(req.getBodyType() != null ? req.getBodyType() : "none");
        r.setBody(req.getBody() != null ? req.getBody() : "");
        r.setFormBody(req.getFormBody());
        r.setCategoryId(req.getCategoryId());
        r.setPreScript(req.getPreScript() != null ? req.getPreScript() : "");
        r.setPostScript(req.getPostScript() != null ? req.getPostScript() : "");
        r.setSortOrder(req.getSortOrder() != null ? req.getSortOrder() : 0);
        r.setCreateTime(now);
        r.setUpdateTime(now);
        r.setDeleted(false);
        savedRequestMapper.insert(r);
        audit(req.getProjectId(), "create", "request", id, req.getName(), r, user);
        // 回读落库后的行再返回：sync_version 由列默认值/触发器维护，内存里的 r 拿不到；
        // 客户端要把它当作「冲突判定」的本地基线，缺失会导致同一行反复更新被误判为冲突。
        SavedRequest saved = savedRequestMapper.selectById(id);
        return ApiResult.ok(saved != null ? saved : r);
    }

    public ApiResult<?> update(SavedRequest req, User user) {
        if (req == null || req.getId() == null) return ApiResult.fail(ErrorCode.PARAM_INVALID, "缺少 id");
        String pid = req.getProjectId();
        if (pid != null && !pid.isBlank()) {
            if (!isProjectAccessible(pid, user)) return ApiResult.fail(ErrorCode.FORBIDDEN, "无权访问该项目");
        } else {
            String checked = resolveAndCheckProject(null, req.getId(), savedRequestMapper, user);
            if ("-".equals(checked)) return ApiResult.fail(ErrorCode.FORBIDDEN, "无权访问该项目");
        }
        // 审计 before 快照：必须在 UPDATE 之前读取，否则拿到的是变更后的值
        Object beforeSnapshot = snapshot(savedRequestMapper, req.getId());
        String now = now();
        savedRequestMapper.update(null, new UpdateWrapper<SavedRequest>().eq("id", req.getId())
                .set("name", req.getName())
                .set("method", req.getMethod())
                .set("url", req.getUrl())
                // JSON 列必须显式序列化为字符串：UpdateWrapper.set 不会应用实体的
                // JacksonTypeHandler，直接传 List<KV> 会因无法绑定参数而抛
                // 「Cannot convert class java.util.ArrayList to SQL type」——接口更新长期不可用。
                .set("params", jsonOrNull(req.getParams()))
                .set("headers", jsonOrNull(req.getHeaders()))
                .set("body_type", req.getBodyType())
                .set("body", req.getBody())
                .set("form_body", jsonOrNull(req.getFormBody()))
                .set("category_id", req.getCategoryId())
                .set("pre_script", req.getPreScript())
                .set("post_script", req.getPostScript())
                .set("sort_order", req.getSortOrder())
                .set("update_time", now));
        req.setUpdateTime(now);
        audit(req.getProjectId(), "update", "request", req.getId(), req.getName(), beforeSnapshot, req, user);
        // 同样回读落库后的行：update_time 由本次写入确定、sync_version 由 BEFORE UPDATE
        // 触发器自增，二者都要返回「库里的真实值」，客户端才能据此更新冲突判定基线。
        SavedRequest saved = savedRequestMapper.selectById(req.getId());
        return ApiResult.ok(saved != null ? saved : req);
    }

    public ApiResult<?> delete(IdRequest req, User user) {
        String id = req != null ? req.getId() : null;
        if (id == null || id.isBlank()) return ApiResult.fail(ErrorCode.PARAM_INVALID, "缺少 id");
        String pid = req != null ? req.getProjectId() : null;
        if (pid != null && !pid.isBlank()) {
            if (!isProjectAccessible(pid, user)) return ApiResult.fail(ErrorCode.FORBIDDEN, "无权访问该项目");
        } else {
            String checked = resolveAndCheckProject(null, id, savedRequestMapper, user);
            if ("-".equals(checked)) return ApiResult.fail(ErrorCode.FORBIDDEN, "无权访问该项目");
        }
        Object beforeSnapshot = snapshot(savedRequestMapper, id);
        softDelete(savedRequestMapper, id);
        audit(pid, "delete", "request", id, "", beforeSnapshot, null, user);
        return ApiResult.ok();
    }

    /** 把 JSON 列实体序列化为字符串；null 保持 null（避免写入字面量 "null" 触发 JSON 列校验失败） */
    private static String jsonOrNull(Object value) {
        return value == null ? null : com.alibaba.fastjson2.JSON.toJSONString(value);
    }
}
