package com.canghai.api.controller;

import com.canghai.api.common.ApiResult;
import com.canghai.api.dto.req.EnvironmentGroupSaveRequest;
import com.canghai.api.dto.req.EnvironmentGroupUpdateRequest;
import com.canghai.api.dto.req.EnvironmentSaveRequest;
import com.canghai.api.dto.req.EnvironmentScopeRequest;
import com.canghai.api.dto.req.EnvironmentUpdateRequest;
import com.canghai.api.dto.req.EnvironmentVariableListRequest;
import com.canghai.api.dto.req.EnvironmentVariableSaveRequest;
import com.canghai.api.dto.req.EnvironmentVariableUpdateRequest;
import com.canghai.api.dto.req.IdRequest;
import com.canghai.api.entity.User;
import com.canghai.api.service.EnvironmentService;
import com.canghai.api.util.RequestContext;
import jakarta.validation.Valid;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.web.bind.annotation.PostMapping;
import org.springframework.web.bind.annotation.RequestBody;
import org.springframework.web.bind.annotation.RequestMapping;
import org.springframework.web.bind.annotation.RestController;

/**
 * 环境路由：/api/v1/environment/**
 * 环境与环境变量均按 online / offline 双份存储，由请求体 mode 字段区分。
 * 登录校验由 AuthAspect 切面统一处理，当前用户从 RequestContext 获取。
 */
@RestController
@RequestMapping("/api/v1/environment")
public class EnvironmentController extends BaseController {

    @Autowired
    private EnvironmentService environmentService;

    /**
     * 环境列表
     * POST /api/v1/environment/list
     */
    @PostMapping("/list")
    public ApiResult<?> list(@RequestBody(required = false) EnvironmentScopeRequest q) {
        try {
            User user = RequestContext.getUser();
            return environmentService.getEnvironments(q, user);
        } finally {
            clear();
        }
    }

    /**
     * 环境分组列表
     * POST /api/v1/environment/groups
     */
    @PostMapping("/groups")
    public ApiResult<?> groups(@RequestBody(required = false) EnvironmentScopeRequest q) {
        try {
            User user = RequestContext.getUser();
            return environmentService.getEnvironmentGroups(q, user);
        } finally {
            clear();
        }
    }

    /**
     * 新增环境
     * POST /api/v1/environment/save
     */
    @PostMapping("/save")
    public ApiResult<?> save(@Valid @RequestBody(required = false) EnvironmentSaveRequest q) {
        try {
            User user = RequestContext.getUser();
            return environmentService.saveEnvironment(q, user);
        } finally {
            clear();
        }
    }

    /**
     * 修改环境
     * POST /api/v1/environment/update
     */
    @PostMapping("/update")
    public ApiResult<?> update(@Valid @RequestBody(required = false) EnvironmentUpdateRequest q) {
        try {
            User user = RequestContext.getUser();
            return environmentService.updateEnvironment(q, user);
        } finally {
            clear();
        }
    }

    /**
     * 删除环境
     * POST /api/v1/environment/delete
     */
    @PostMapping("/delete")
    public ApiResult<?> delete(@Valid @RequestBody(required = false) IdRequest q) {
        try {
            User user = RequestContext.getUser();
            return environmentService.deleteEnvironment(q, user);
        } finally {
            clear();
        }
    }

    /**
     * 设置当前启用环境
     * POST /api/v1/environment/set-active
     */
    @PostMapping("/set-active")
    public ApiResult<?> setActive(@Valid @RequestBody(required = false) IdRequest q) {
        try {
            User user = RequestContext.getUser();
            return environmentService.setActiveEnvironment(q, user);
        } finally {
            clear();
        }
    }

    /**
     * 当前启用环境
     * POST /api/v1/environment/active
     */
    @PostMapping("/active")
    public ApiResult<?> active(@RequestBody(required = false) EnvironmentScopeRequest q) {
        try {
            User user = RequestContext.getUser();
            return environmentService.getActiveEnvironment(q, user);
        } finally {
            clear();
        }
    }

    /**
     * 当前启用环境的变量
     * POST /api/v1/environment/active-vars
     */
    @PostMapping("/active-vars")
    public ApiResult<?> activeVars(@RequestBody(required = false) EnvironmentScopeRequest q) {
        try {
            User user = RequestContext.getUser();
            return environmentService.getActiveEnvVariables(q, user);
        } finally {
            clear();
        }
    }

    /**
     * 新增环境分组
     * POST /api/v1/environment/groups/save
     */
    @PostMapping("/groups/save")
    public ApiResult<?> saveGroup(@Valid @RequestBody(required = false) EnvironmentGroupSaveRequest q) {
        try {
            User user = RequestContext.getUser();
            return environmentService.saveEnvironmentGroup(q, user);
        } finally {
            clear();
        }
    }

    /**
     * 修改环境分组
     * POST /api/v1/environment/groups/update
     */
    @PostMapping("/groups/update")
    public ApiResult<?> updateGroup(@Valid @RequestBody(required = false) EnvironmentGroupUpdateRequest q) {
        try {
            User user = RequestContext.getUser();
            return environmentService.updateEnvironmentGroup(q, user);
        } finally {
            clear();
        }
    }

    /**
     * 删除环境分组
     * POST /api/v1/environment/groups/delete
     */
    @PostMapping("/groups/delete")
    public ApiResult<?> deleteGroup(@Valid @RequestBody(required = false) IdRequest q) {
        try {
            User user = RequestContext.getUser();
            return environmentService.deleteEnvironmentGroup(q, user);
        } finally {
            clear();
        }
    }

    /**
     * 环境变量列表
     * POST /api/v1/environment/variables/list
     */
    @PostMapping("/variables/list")
    public ApiResult<?> variables(@RequestBody(required = false) EnvironmentVariableListRequest q) {
        try {
            User user = RequestContext.getUser();
            return environmentService.getEnvironmentVariables(q, user);
        } finally {
            clear();
        }
    }

    /**
     * 新增/保存环境变量（带 id 视为更新）
     * POST /api/v1/environment/variables/save
     */
    @PostMapping("/variables/save")
    public ApiResult<?> saveVariable(@Valid @RequestBody(required = false) EnvironmentVariableSaveRequest q) {
        try {
            User user = RequestContext.getUser();
            return environmentService.saveEnvironmentVariable(q, user);
        } finally {
            clear();
        }
    }

    /**
     * 修改环境变量
     * POST /api/v1/environment/variables/update
     */
    @PostMapping("/variables/update")
    public ApiResult<?> updateVariable(@Valid @RequestBody(required = false) EnvironmentVariableUpdateRequest q) {
        try {
            User user = RequestContext.getUser();
            return environmentService.updateEnvironmentVariable(q, user);
        } finally {
            clear();
        }
    }

    /**
     * 删除环境变量
     * POST /api/v1/environment/variables/delete
     */
    @PostMapping("/variables/delete")
    public ApiResult<?> deleteVariable(@Valid @RequestBody(required = false) IdRequest q) {
        try {
            User user = RequestContext.getUser();
            return environmentService.deleteEnvironmentVariable(q, user);
        } finally {
            clear();
        }
    }
}
