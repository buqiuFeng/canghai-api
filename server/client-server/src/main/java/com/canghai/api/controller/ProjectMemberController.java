package com.canghai.api.controller;

import com.canghai.api.common.ApiResult;
import com.canghai.api.dto.req.AddProjectMemberRequest;
import com.canghai.api.dto.req.ProjectIdRequest;
import com.canghai.api.dto.req.RemoveProjectMemberRequest;
import com.canghai.api.dto.req.TeamIdRequest;
import com.canghai.api.entity.User;
import com.canghai.api.service.ProjectMemberService;
import com.canghai.api.util.RequestContext;
import jakarta.validation.Valid;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.web.bind.annotation.PostMapping;
import org.springframework.web.bind.annotation.RequestBody;
import org.springframework.web.bind.annotation.RequestMapping;
import org.springframework.web.bind.annotation.RestController;

/**
 * 项目成员路由：/api/v1/project-member/**
 * 登录校验由 AuthAspect 切面统一处理，当前用户从 RequestContext 获取。
 */
@RestController
@RequestMapping("/api/v1/project-member")
public class ProjectMemberController extends BaseController {

    @Autowired
    private ProjectMemberService projectMemberService;

    /**
     * 添加项目成员
     * POST /api/v1/project-member/add
     */
    @PostMapping("/add")
    public ApiResult<?> add(@Valid @RequestBody(required = false) AddProjectMemberRequest q) {
        try {
            User user = RequestContext.getUser();
            return projectMemberService.addMember(q, user);
        } finally {
            clear();
        }
    }

    /**
     * 移除项目成员
     * POST /api/v1/project-member/remove
     */
    @PostMapping("/remove")
    public ApiResult<?> remove(@Valid @RequestBody(required = false) RemoveProjectMemberRequest q) {
        try {
            User user = RequestContext.getUser();
            return projectMemberService.removeMember(q, user);
        } finally {
            clear();
        }
    }

    /**
     * 项目成员列表
     * POST /api/v1/project-member/list
     */
    @PostMapping("/list")
    public ApiResult<?> list(@RequestBody(required = false) ProjectIdRequest q) {
        try {
            User user = RequestContext.getUser();
            return projectMemberService.listMembers(q, user);
        } finally {
            clear();
        }
    }

    /**
     * 按团队查询关联项目
     * POST /api/v1/project-member/by-team
     */
    @PostMapping("/by-team")
    public ApiResult<?> byTeam(@RequestBody(required = false) TeamIdRequest q) {
        try {
            User user = RequestContext.getUser();
            return projectMemberService.listProjectsByTeam(q, user);
        } finally {
            clear();
        }
    }

    /**
     * 当前用户可见项目（用户即查询条件，无需请求体）
     * POST /api/v1/project-member/visible
     */
    @PostMapping("/visible")
    public ApiResult<?> visible() {
        try {
            User user = RequestContext.getUser();
            return projectMemberService.listVisibleProjects(user);
        } finally {
            clear();
        }
    }
}
