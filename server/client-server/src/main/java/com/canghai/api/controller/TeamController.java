package com.canghai.api.controller;

import com.canghai.api.common.ApiResult;
import com.canghai.api.dto.req.IdRequest;
import com.canghai.api.entity.Team;
import com.canghai.api.entity.TeamMember;
import com.canghai.api.entity.User;
import com.canghai.api.service.TeamService;
import com.canghai.api.util.RequestContext;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.web.bind.annotation.PostMapping;
import org.springframework.web.bind.annotation.RequestBody;
import org.springframework.web.bind.annotation.RequestMapping;
import org.springframework.web.bind.annotation.RestController;

/**
 * 团队路由：/api/v1/team/**
 * 登录校验由 AuthAspect 切面统一处理，当前用户从 RequestContext 获取。
 */
@RestController
@RequestMapping("/api/v1/team")
public class TeamController extends BaseController {

    @Autowired
    private TeamService teamService;

    /**
     * 我的团队列表（用户即查询条件，无需请求体）
     * POST /api/v1/team/mine
     */
    @PostMapping("/mine")
    public ApiResult<?> mine() {
        try {
            User user = RequestContext.getUser();
            return teamService.getMyTeams(user);
        } finally {
            clear();
        }
    }

    /**
     * 创建团队
     * POST /api/v1/team/create
     */
    @PostMapping("/create")
    public ApiResult<?> create(@RequestBody(required = false) Team q) {
        try {
            User user = RequestContext.getUser();
            return teamService.createTeam(q, user);
        } finally {
            clear();
        }
    }

    /**
     * 修改团队
     * POST /api/v1/team/update
     */
    @PostMapping("/update")
    public ApiResult<?> update(@RequestBody(required = false) Team q) {
        try {
            User user = RequestContext.getUser();
            return teamService.updateTeam(q, user);
        } finally {
            clear();
        }
    }

    /**
     * 删除团队
     * POST /api/v1/team/delete
     */
    @PostMapping("/delete")
    public ApiResult<?> delete(@RequestBody(required = false) IdRequest q) {
        try {
            User user = RequestContext.getUser();
            return teamService.deleteTeam(q, user);
        } finally {
            clear();
        }
    }

    /**
     * 团队成员列表
     * POST /api/v1/team/members
     */
    @PostMapping("/members")
    public ApiResult<?> members(@RequestBody(required = false) IdRequest q) {
        try {
            User user = RequestContext.getUser();
            return teamService.getMembers(q, user);
        } finally {
            clear();
        }
    }

    /**
     * 邀请成员
     * POST /api/v1/team/invite
     */
    @PostMapping("/invite")
    public ApiResult<?> invite(@RequestBody(required = false) TeamMember.InviteRequest q) {
        try {
            User user = RequestContext.getUser();
            return teamService.inviteMember(q, user);
        } finally {
            clear();
        }
    }

    /**
     * 修改成员角色
     * POST /api/v1/team/changeRole
     */
    @PostMapping("/changeRole")
    public ApiResult<?> changeRole(@RequestBody(required = false) TeamMember.ChangeRoleRequest q) {
        try {
            User user = RequestContext.getUser();
            return teamService.changeMemberRole(q, user);
        } finally {
            clear();
        }
    }

    /**
     * 移除成员
     * POST /api/v1/team/removeMember
     */
    @PostMapping("/removeMember")
    public ApiResult<?> removeMember(@RequestBody(required = false) TeamMember.RemoveMemberRequest q) {
        try {
            User user = RequestContext.getUser();
            return teamService.removeMember(q, user);
        } finally {
            clear();
        }
    }

    /**
     * 移交团队所有权
     * POST /api/v1/team/transfer
     */
    @PostMapping("/transfer")
    public ApiResult<?> transfer(@RequestBody(required = false) TeamMember.TransferRequest q) {
        try {
            User user = RequestContext.getUser();
            return teamService.transferOwnership(q, user);
        } finally {
            clear();
        }
    }
}
