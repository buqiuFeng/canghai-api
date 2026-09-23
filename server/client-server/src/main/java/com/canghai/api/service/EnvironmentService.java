package com.canghai.api.service;

import com.alibaba.fastjson2.JSONObject;
import com.canghai.api.common.ApiResult;
import com.canghai.api.common.ErrorCode;
import com.canghai.api.dto.req.DataModeAware;
import com.canghai.api.dto.req.EnvironmentGroupSaveRequest;
import com.canghai.api.dto.req.EnvironmentGroupUpdateRequest;
import com.canghai.api.dto.req.EnvironmentSaveRequest;
import com.canghai.api.dto.req.EnvironmentScopeRequest;
import com.canghai.api.dto.req.EnvironmentUpdateRequest;
import com.canghai.api.dto.req.EnvironmentVariableListRequest;
import com.canghai.api.dto.req.EnvironmentVariableSaveRequest;
import com.canghai.api.dto.req.EnvironmentVariableUpdateRequest;
import com.canghai.api.dto.req.IdRequest;
import com.canghai.api.entity.Environment;
import com.canghai.api.entity.EnvironmentGroup;
import com.canghai.api.entity.EnvironmentVariable;
import com.canghai.api.entity.User;
import com.canghai.api.mapper.EnvironmentGroupMapper;
import com.canghai.api.mapper.EnvironmentMapper;
import com.canghai.api.mapper.EnvironmentVariableMapper;
import com.baomidou.mybatisplus.core.conditions.query.QueryWrapper;
import com.baomidou.mybatisplus.core.conditions.update.UpdateWrapper;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.stereotype.Service;

import java.util.List;

/**
 * 环境 / 环境分组服务。路由前缀：/api/environment
 */
@Service
public class EnvironmentService extends BaseService {

    @Autowired
    private EnvironmentMapper environmentMapper;
    @Autowired
    private EnvironmentGroupMapper environmentGroupMapper;
    @Autowired
    private EnvironmentVariableMapper environmentVariableMapper;


    private String emptyIfNull(String s) {
        return s == null ? "" : s;
    }

    public ApiResult<?> getEnvironments(EnvironmentScopeRequest req, User user) {
        String projectId = req != null && req.getProjectId() != null ? req.getProjectId() : "";
        if (!isProjectAccessible(projectId, user)) {
            return ApiResult.fail(ErrorCode.FORBIDDEN, "无权访问该项目");
        }
        String dataMode = resolveDataMode(req);
        List<Environment> rows = environmentMapper.selectList(
                qw(Environment.class).eq("project_id", projectId).eq("deleted", 0).eq("data_mode", dataMode)
                        .orderByAsc("sort_order").orderByAsc("create_time"));
        return ApiResult.ok(rows);
    }

    public ApiResult<?> getEnvironmentGroups(EnvironmentScopeRequest req, User user) {
        String projectId = req != null && req.getProjectId() != null ? req.getProjectId() : "";
        if (!isProjectAccessible(projectId, user)) {
            return ApiResult.fail(ErrorCode.FORBIDDEN, "无权访问该项目");
        }
        String dataMode = resolveDataMode(req);
        List<EnvironmentGroup> rows = environmentGroupMapper.selectList(
                qw(EnvironmentGroup.class).eq("project_id", projectId).eq("deleted", 0).eq("data_mode", dataMode)
                        .orderByAsc("sort_order").orderByAsc("create_time"));
        return ApiResult.ok(rows);
    }

    public ApiResult<?> saveEnvironment(EnvironmentSaveRequest req, User user) {
        if (req == null || req.getName() == null) {
            return ApiResult.fail(ErrorCode.PARAM_INVALID, "缺少必填字段 name");
        }
        String projectId = emptyIfNull(req.getProjectId());
        if (!isProjectAccessible(projectId, user)) {
            return ApiResult.fail(ErrorCode.FORBIDDEN, "无权访问该项目");
        }
        String dataMode = resolveDataMode(req);
        String now = now();
        String id = uuid();
        Environment e = new Environment();
        e.setId(id);
        e.setProjectId(projectId);
        e.setName(req.getName());
        e.setGroupId(req.getGroupId());
        e.setIsActive(false);
        e.setSortOrder(req.getSortOrder() != null ? req.getSortOrder() : 0);
        e.setDataMode(dataMode);
        e.setCreateTime(now);
        e.setCreateBy(req.getUserId());
        e.setUpdateTime(now);
        e.setUpdateBy(req.getUserId());
        e.setDeleted(false);
        environmentMapper.insert(e);
        return ApiResult.ok(e);
    }

    public ApiResult<?> updateEnvironment(EnvironmentUpdateRequest req, User user) {
        if (req == null || req.getId() == null) return ApiResult.fail(ErrorCode.PARAM_INVALID, "缺少 id");
        String id = req.getId();
        String checked = resolveAndCheckProject(req.getProjectId(), id, environmentMapper, user);
        if ("-".equals(checked)) return ApiResult.fail(ErrorCode.FORBIDDEN, "无权访问该项目");
        String now = now();
        environmentMapper.update(null, new UpdateWrapper<Environment>().eq("id", id)
                .set("name", req.getName())
                .set("group_id", req.getGroupId())
                .set("sort_order", req.getSortOrder())
                .set("update_time", now).set("update_by", req.getUserId()));
        return ApiResult.ok("更新成功");
    }

    public ApiResult<?> deleteEnvironment(IdRequest req, User user) {
        if (req == null || req.getId() == null) return ApiResult.fail(ErrorCode.PARAM_INVALID, "缺少 id");
        String id = req.getId();
        String checked = resolveAndCheckProject(req.getProjectId(), id, environmentMapper, user);
        if ("-".equals(checked)) return ApiResult.fail(ErrorCode.FORBIDDEN, "无权访问该项目");
        softDelete(environmentMapper, id);
        return ApiResult.ok(id);
    }

    public ApiResult<?> setActiveEnvironment(IdRequest req, User user) {
        if (req == null || req.getId() == null) return ApiResult.fail(ErrorCode.PARAM_INVALID, "缺少 id");
        String id = req.getId();
        String projectId = emptyIfNull(req.getProjectId());
        if (!isProjectAccessible(projectId, user)) {
            return ApiResult.fail(ErrorCode.FORBIDDEN, "无权访问该项目");
        }
        String dataMode = resolveDataMode(req);
        String now = now();
        environmentMapper.update(null, new UpdateWrapper<Environment>()
                .eq("project_id", projectId).eq("is_active", 1).eq("deleted", 0).eq("data_mode", dataMode)
                .set("is_active", 0));
        environmentMapper.update(null, new UpdateWrapper<Environment>().eq("id", id).eq("deleted", 0).eq("data_mode", dataMode)
                .set("is_active", 1).set("update_time", now).set("update_by", req.getUserId()));
        JSONObject r = new JSONObject();
        r.put("id", id);
        r.put("active", true);
        return ApiResult.ok(r);
    }

    public ApiResult<?> getActiveEnvironment(EnvironmentScopeRequest req, User user) {
        String projectId = req != null && req.getProjectId() != null ? req.getProjectId() : "";
        if (!isProjectAccessible(projectId, user)) {
            return ApiResult.fail(ErrorCode.FORBIDDEN, "无权访问该项目");
        }
        String dataMode = resolveDataMode(req);
        List<Environment> rows = environmentMapper.selectList(
                qw(Environment.class).eq("project_id", projectId).eq("is_active", 1)
                        .eq("deleted", 0).eq("data_mode", dataMode).last("LIMIT 1"));
        return ApiResult.ok(rows.isEmpty() ? null : rows.get(0));
    }

    public ApiResult<?> getActiveEnvVariables(EnvironmentScopeRequest req, User user) {
        String projectId = req != null && req.getProjectId() != null ? req.getProjectId() : "";
        if (!isProjectAccessible(projectId, user)) {
            return ApiResult.fail(ErrorCode.FORBIDDEN, "无权访问该项目");
        }
        String dataMode = resolveDataMode(req);
        List<EnvironmentVariable> rows = environmentVariableMapper.selectActiveVariables(projectId, dataMode);
        return ApiResult.ok(rows);
    }

    public ApiResult<?> saveEnvironmentGroup(EnvironmentGroupSaveRequest req, User user) {
        if (req == null || req.getName() == null) {
            return ApiResult.fail(ErrorCode.PARAM_INVALID, "缺少必填字段 name");
        }
        String projectId = emptyIfNull(req.getProjectId());
        if (!isProjectAccessible(projectId, user)) {
            return ApiResult.fail(ErrorCode.FORBIDDEN, "无权访问该项目");
        }
        String dataMode = resolveDataMode(req);
        String now = now();
        String id = uuid();
        EnvironmentGroup g = new EnvironmentGroup();
        g.setId(id);
        g.setProjectId(projectId);
        g.setName(req.getName());
        g.setSortOrder(req.getSortOrder() != null ? req.getSortOrder() : 0);
        g.setExpanded(req.getExpanded() == null || Boolean.TRUE.equals(req.getExpanded()));
        g.setDataMode(dataMode);
        g.setCreateTime(now);
        g.setCreateBy(req.getUserId());
        g.setUpdateTime(now);
        g.setUpdateBy(req.getUserId());
        g.setDeleted(false);
        environmentGroupMapper.insert(g);
        return ApiResult.ok(g);
    }

    public ApiResult<?> updateEnvironmentGroup(EnvironmentGroupUpdateRequest req, User user) {
        if (req == null || req.getId() == null) return ApiResult.fail(ErrorCode.PARAM_INVALID, "缺少 id");
        String id = req.getId();
        String checked = resolveAndCheckProject(req.getProjectId(), id, environmentGroupMapper, user);
        if ("-".equals(checked)) return ApiResult.fail(ErrorCode.FORBIDDEN, "无权访问该项目");
        String now = now();
        environmentGroupMapper.update(null, new UpdateWrapper<EnvironmentGroup>().eq("id", id)
                .set("name", req.getName())
                .set("sort_order", req.getSortOrder())
                .set("expanded", req.getExpanded() != null && Boolean.TRUE.equals(req.getExpanded()))
                .set("update_time", now).set("update_by", req.getUserId()));
        return ApiResult.ok("更新成功");
    }

    public ApiResult<?> deleteEnvironmentGroup(IdRequest req, User user) {
        if (req == null || req.getId() == null) return ApiResult.fail(ErrorCode.PARAM_INVALID, "缺少 id");
        String id = req.getId();
        String checked = resolveAndCheckProject(req.getProjectId(), id, environmentGroupMapper, user);
        if ("-".equals(checked)) return ApiResult.fail(ErrorCode.FORBIDDEN, "无权访问该项目");
        String dataMode = resolveDataMode(req);
        String now = now();
        // 分组下环境置空
        environmentMapper.update(null, new UpdateWrapper<Environment>().eq("group_id", id).eq("deleted", 0).eq("data_mode", dataMode)
                .set("group_id", null).set("update_time", now).set("update_by", req.getUserId()));
        softDelete(environmentGroupMapper, id);
        return ApiResult.ok(id);
    }

    public ApiResult<?> getEnvironmentVariables(EnvironmentVariableListRequest req, User user) {
        String environmentId = req != null ? req.getEnvironmentId() : null;
        if (environmentId == null || environmentId.isBlank()) return ApiResult.fail(ErrorCode.PARAM_INVALID, "缺少 environmentId");
        if (!checkEnvVarAccess(environmentId, user)) {
            return ApiResult.fail(ErrorCode.FORBIDDEN, "无权访问该环境");
        }
        String dataMode = resolveDataMode(req);
        List<EnvironmentVariable> rows = environmentVariableMapper.selectList(
                qw(EnvironmentVariable.class).eq("environment_id", environmentId).eq("deleted", 0).eq("data_mode", dataMode)
                        .orderByAsc("sort_order").orderByAsc("create_time"));
        return ApiResult.ok(rows);
    }

    public ApiResult<?> saveEnvironmentVariable(EnvironmentVariableSaveRequest req, User user) {
        if (req == null || req.getEnvironmentId() == null || req.getKey() == null) {
            return ApiResult.fail(ErrorCode.PARAM_INVALID, "缺少必填字段 environmentId/key");
        }
        if (!checkEnvVarAccess(req.getEnvironmentId(), user)) {
            return ApiResult.fail(ErrorCode.FORBIDDEN, "无权访问该环境");
        }
        String dataMode = resolveDataMode(req);
        String now = now();
        String id = req.getId();
        if (id == null || id.isEmpty()) {
            id = uuid();
            EnvironmentVariable v = new EnvironmentVariable();
            v.setId(id);
            v.setEnvironmentId(req.getEnvironmentId());
            v.setVarKey(req.getKey());
            v.setValue(req.getValue() != null ? req.getValue() : "");
            v.setEnabled(req.getEnabled() != null && req.getEnabled());
            v.setSortOrder(req.getSortOrder() != null ? req.getSortOrder() : 0);
            v.setDataMode(dataMode);
            v.setCreateTime(now);
            v.setCreateBy(req.getUserId());
            v.setUpdateTime(now);
            v.setUpdateBy(req.getUserId());
            v.setDeleted(false);
            environmentVariableMapper.insert(v);
            return ApiResult.ok(v);
        }
        environmentVariableMapper.update(null, new UpdateWrapper<EnvironmentVariable>().eq("id", id).eq("deleted", 0).eq("data_mode", dataMode)
                .set("var_key", req.getKey())
                .set("value", req.getValue() != null ? req.getValue() : "")
                .set("enabled", req.getEnabled() != null && req.getEnabled())
                .set("sort_order", req.getSortOrder() != null ? req.getSortOrder() : 0)
                .set("update_time", now).set("update_by", req.getUserId()));
        return ApiResult.ok("更新成功");
    }

    public ApiResult<?> updateEnvironmentVariable(EnvironmentVariableUpdateRequest req, User user) {
        if (req == null || req.getId() == null) return ApiResult.fail(ErrorCode.PARAM_INVALID, "缺少 id");
        String id = req.getId();
        EnvironmentVariable v = environmentVariableMapper.selectById(id);
        if (v == null || Boolean.TRUE.equals(v.getDeleted())) return ApiResult.fail(ErrorCode.NOT_FOUND, "记录不存在");
        if (!checkEnvVarAccess(v.getEnvironmentId(), user)) {
            return ApiResult.fail(ErrorCode.FORBIDDEN, "无权访问该环境");
        }
        String dataMode = resolveDataMode(req);
        String now = now();
        environmentVariableMapper.update(null, new UpdateWrapper<EnvironmentVariable>().eq("id", id)
                .set("var_key", req.getKey())
                .set("value", req.getValue())
                .set("enabled", req.getEnabled() != null && req.getEnabled())
                .set("sort_order", req.getSortOrder())
                .set("update_time", now).set("update_by", req.getUserId()));
        return ApiResult.ok("更新成功");
    }

    public ApiResult<?> deleteEnvironmentVariable(IdRequest req, User user) {
        if (req == null || req.getId() == null) return ApiResult.fail(ErrorCode.PARAM_INVALID, "缺少 id");
        String id = req.getId();
        EnvironmentVariable v = environmentVariableMapper.selectById(id);
        if (v == null || Boolean.TRUE.equals(v.getDeleted())) return ApiResult.fail(ErrorCode.NOT_FOUND, "记录不存在");
        if (!checkEnvVarAccess(v.getEnvironmentId(), user)) {
            return ApiResult.fail(ErrorCode.FORBIDDEN, "无权访问该环境");
        }
        softDelete(environmentVariableMapper, id);
        return ApiResult.ok(id);
    }

    private boolean checkEnvVarAccess(String envId, User user) {
        if (envId == null || envId.isBlank()) return false;
        Environment env = environmentMapper.selectById(envId);
        if (env == null) return false;
        return isProjectAccessible(env.getProjectId(), user);
    }

    /** 解析 data_mode（默认 online，仅接受 online/offline） */
    private String resolveDataMode(DataModeAware req) {
        String mode = req != null ? req.getMode() : null;
        return "offline".equals(mode) ? "offline" : "online";
    }
}
