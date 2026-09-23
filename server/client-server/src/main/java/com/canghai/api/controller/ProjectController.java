package com.canghai.api.controller;

import com.canghai.api.common.ApiResult;
import com.canghai.api.dto.req.IdRequest;
import com.canghai.api.dto.req.ProjectSaveRequest;
import com.canghai.api.entity.User;
import com.canghai.api.service.ProjectService;
import com.canghai.api.util.RequestContext;
import jakarta.validation.Valid;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.web.bind.annotation.PostMapping;
import org.springframework.web.bind.annotation.RequestBody;
import org.springframework.web.bind.annotation.RequestMapping;
import org.springframework.web.bind.annotation.RestController;

/**
 * 项目路由：/api/v1/project/**
 * 登录校验由 AuthAspect 切面统一处理，当前用户从 RequestContext 获取。
 */
@RestController
@RequestMapping("/api/v1/project")
public class ProjectController extends BaseController {

    @Autowired
    private ProjectService projectService;

    /**
     * 保存项目（id 为空表示新建）
     * POST /api/v1/project/save
     */
    @PostMapping("/save")
    public ApiResult<?> save(@Valid @RequestBody(required = false) ProjectSaveRequest q) {
        try {
            User user = RequestContext.getUser();
            return projectService.save(q, user);
        } finally {
            clear();
        }
    }

    /**
     * 删除项目
     * POST /api/v1/project/delete
     */
    @PostMapping("/delete")
    public ApiResult<?> delete(@Valid @RequestBody(required = false) IdRequest q) {
        try {
            User user = RequestContext.getUser();
            return projectService.delete(q, user);
        } finally {
            clear();
        }
    }
}
