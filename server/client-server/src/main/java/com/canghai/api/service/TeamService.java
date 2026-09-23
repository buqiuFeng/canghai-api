package com.canghai.api.service;

import com.canghai.api.common.ApiResult;
import com.canghai.api.common.ErrorCode;
import com.canghai.api.common.ApiException;
import com.canghai.api.dto.req.IdRequest;
import com.canghai.api.entity.Team;
import com.canghai.api.entity.TeamMember;
import com.canghai.api.entity.User;
import com.canghai.api.mapper.TeamMapper;
import com.canghai.api.mapper.TeamMemberMapper;
import com.canghai.api.mapper.UserMapper;
import com.baomidou.mybatisplus.core.conditions.query.QueryWrapper;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.stereotype.Service;
import org.springframework.transaction.annotation.Transactional;

import java.util.ArrayList;
import java.util.List;
import java.util.Map;

/**
 * 团队服务 — 团队的 CRUD、成员管理、权限校验。
 * 路由前缀：/api/team
 */
@Service
public class TeamService extends BaseService {

    @Autowired
    private TeamMapper teamMapper;
    @Autowired
    private TeamMemberMapper teamMemberMapper;
    @Autowired
    private UserMapper userMapper;


    private User findUserByUsername(String username) {
        return userMapper.selectOne(new QueryWrapper<User>().eq("username", username).eq("deleted", 0));
    }

    /** 我的团队列表（无需请求体，当前用户即查询条件） */
    public ApiResult<?> getMyTeams(User user) {
        String userId = user != null ? user.getId() : null;
        List<Team> teams = teamMapper.selectList(
                qw(Team.class)
                        .eq("deleted", 0)
                        .and(w -> w.eq("owner_id", userId).or()
                                .apply("id IN (SELECT team_id FROM ch_team_members WHERE user_id = {0} AND deleted = 0)", userId))
                        .orderByDesc("create_time"));
        return ApiResult.ok(teams);
    }

    public ApiResult<?> createTeam(Team req, User user) {
        if (req == null || req.getName() == null || req.getName().trim().isEmpty()) {
            return ApiResult.fail(ErrorCode.PARAM_INVALID, "团队名称不能为空");
        }
        String now = now();
        String teamId = uuid();
        Team team = new Team();
        team.setId(teamId);
        team.setName(req.getName().trim());
        team.setDescription(req.getDescription() != null ? req.getDescription() : "");
        team.setOwnerId(user.getId());
        team.setUserId(user.getId());
        team.setCreateTime(now);
        team.setCreateBy(user.getId());
        team.setUpdateTime(now);
        team.setUpdateBy(user.getId());
        team.setDeleted(false);
        teamMapper.insert(team);

        TeamMember member = new TeamMember();
        member.setId(uuid());
        member.setTeamId(teamId);
        member.setUserId(user.getId());
        member.setRole(TeamMember.ROLE_OWNER);
        member.setCreateTime(now);
        member.setCreateBy(user.getId());
        member.setUpdateTime(now);
        member.setUpdateBy(user.getId());
        member.setDeleted(false);
        teamMemberMapper.insert(member);
        return ApiResult.ok(team);
    }

    public ApiResult<?> updateTeam(Team req, User user) {
        if (req == null || req.getId() == null || req.getId().isEmpty()) {
            return ApiResult.fail(ErrorCode.PARAM_INVALID, "团队 ID 不能为空");
        }
        if (req.getName() == null || req.getName().trim().isEmpty()) {
            return ApiResult.fail(ErrorCode.PARAM_INVALID, "团队名称不能为空");
        }
        String role = getRole(req.getId(), user.getId());
        if (role == null || (!role.equals(TeamMember.ROLE_OWNER) && !role.equals(TeamMember.ROLE_ADMIN))) {
            return ApiResult.fail(ErrorCode.FORBIDDEN, "无权修改团队信息");
        }
        teamMapper.update(null, new com.baomidou.mybatisplus.core.conditions.update.UpdateWrapper<Team>()
                .eq("id", req.getId())
                .set("name", req.getName().trim())
                .set("description", req.getDescription() != null ? req.getDescription() : "")
                .set("update_time", now()).set("update_by", user.getId()));
        return ApiResult.ok("更新成功");
    }

    public ApiResult<?> deleteTeam(IdRequest req, User user) {
        String teamId = req != null ? req.getId() : null;
        if (teamId == null || teamId.isEmpty()) {
            return ApiResult.fail(ErrorCode.PARAM_INVALID, "团队 ID 不能为空");
        }
        String role = getRole(teamId, user.getId());
        if (role == null || !role.equals(TeamMember.ROLE_OWNER)) {
            return ApiResult.fail(ErrorCode.FORBIDDEN, "只有所有者可以删除团队");
        }
        String now = now();
        teamMapper.update(null, new com.baomidou.mybatisplus.core.conditions.update.UpdateWrapper<Team>()
                .eq("id", teamId).set("deleted", 1).set("update_time", now).set("update_by", user.getId()));
        teamMemberMapper.update(null, new com.baomidou.mybatisplus.core.conditions.update.UpdateWrapper<TeamMember>()
                .eq("team_id", teamId).set("deleted", 1).set("update_time", now).set("update_by", user.getId()));
        return ApiResult.ok("删除成功");
    }

    public ApiResult<?> getMembers(IdRequest req, User user) {
        String userId = user != null ? user.getId() : null;
        String teamId = req != null ? req.getId() : null;
        if (teamId == null || teamId.isEmpty()) {
            return ApiResult.fail(ErrorCode.PARAM_INVALID, "团队 ID 不能为空");
        }
        if (getRole(teamId, userId) == null) {
            return ApiResult.fail(ErrorCode.FORBIDDEN, "无权访问该团队");
        }
        List<Map<String, Object>> rows = teamMemberMapper.selectMembersWithUsername(teamId);
        List<TeamMember.MemberInfo> members = new ArrayList<>();
        for (Map<String, Object> row : rows) {
            TeamMember.MemberInfo info = new TeamMember.MemberInfo();
            info.setId(str(row.get("id")));
            info.setUserId(str(row.get("userId")));
            info.setUsername(str(row.get("username")));
            info.setRole(str(row.get("role")));
            info.setCreateTime(str(row.get("createTime")));
            members.add(info);
        }
        return ApiResult.ok(members);
    }

    public ApiResult<?> inviteMember(TeamMember.InviteRequest req, User user) {
        if (req == null || req.getTeamId() == null || req.getTeamId().isEmpty()
                || req.getUsername() == null || req.getUsername().trim().isEmpty()) {
            return ApiResult.fail(ErrorCode.PARAM_INVALID, "团队 ID 和用户名不能为空");
        }
        String role = getRole(req.getTeamId(), user.getId());
        if (role == null || (!role.equals(TeamMember.ROLE_OWNER) && !role.equals(TeamMember.ROLE_ADMIN))) {
            return ApiResult.fail(ErrorCode.FORBIDDEN, "无权邀请成员");
        }
        String targetRole = req.getRole();
        if (targetRole == null || targetRole.isEmpty()) targetRole = TeamMember.ROLE_READWRITE;
        if (!isValidRole(targetRole)) {
            return ApiResult.fail(ErrorCode.PARAM_INVALID, "非法的角色: " + targetRole);
        }
        if (!role.equals(TeamMember.ROLE_OWNER) && isHigherRole(targetRole, role)) {
            return ApiResult.fail(ErrorCode.FORBIDDEN, "无权分配高于自身权限的角色");
        }
        User target = findUserByUsername(req.getUsername().trim());
        if (target == null) {
            return ApiResult.fail(ErrorCode.NOT_FOUND, "用户不存在: " + req.getUsername());
        }
        Long exists = teamMemberMapper.selectCount(qw(TeamMember.class)
                .eq("team_id", req.getTeamId()).eq("user_id", target.getId()).eq("deleted", 0));
        if (exists != null && exists > 0) {
            return ApiResult.fail(ErrorCode.CONFLICT, "该用户已是团队成员");
        }
        String now = now();
        TeamMember member = new TeamMember();
        member.setId(uuid());
        member.setTeamId(req.getTeamId());
        member.setUserId(target.getId());
        member.setRole(targetRole);
        member.setCreateTime(now);
        member.setCreateBy(user.getId());
        member.setUpdateTime(now);
        member.setUpdateBy(user.getId());
        member.setDeleted(false);
        teamMemberMapper.insert(member);
        return ApiResult.ok("邀请成功");
    }

    public ApiResult<?> changeMemberRole(TeamMember.ChangeRoleRequest req, User user) {
        if (req == null || req.getTeamId() == null || req.getTeamId().isEmpty()
                || req.getUserId() == null || req.getUserId().isEmpty()
                || req.getRole() == null || req.getRole().trim().isEmpty()) {
            return ApiResult.fail(ErrorCode.PARAM_INVALID, "参数不能为空");
        }
        String role = getRole(req.getTeamId(), user.getId());
        if (role == null || (!role.equals(TeamMember.ROLE_OWNER) && !role.equals(TeamMember.ROLE_ADMIN))) {
            return ApiResult.fail(ErrorCode.FORBIDDEN, "无权修改成员角色");
        }
        if (!isValidRole(req.getRole())) {
            return ApiResult.fail(ErrorCode.PARAM_INVALID, "非法的角色: " + req.getRole());
        }
        if (!role.equals(TeamMember.ROLE_OWNER) && isHigherRole(req.getRole(), role)) {
            return ApiResult.fail(ErrorCode.FORBIDDEN, "无权分配高于自身权限的角色");
        }
        String targetCurrentRole = getRole(req.getTeamId(), req.getUserId());
        if (TeamMember.ROLE_OWNER.equals(targetCurrentRole)) {
            return ApiResult.fail(ErrorCode.FORBIDDEN, "不能修改所有者的角色");
        }
        teamMemberMapper.update(null, new com.baomidou.mybatisplus.core.conditions.update.UpdateWrapper<TeamMember>()
                .eq("id", getMemberId(req.getTeamId(), req.getUserId()))
                .set("role", req.getRole()).set("update_time", now()).set("update_by", user.getId()));
        return ApiResult.ok("修改成功");
    }

    public ApiResult<?> removeMember(TeamMember.RemoveMemberRequest req, User user) {
        if (req == null || req.getTeamId() == null || req.getTeamId().isEmpty()
                || req.getUserId() == null || req.getUserId().isEmpty()) {
            return ApiResult.fail(ErrorCode.PARAM_INVALID, "参数不能为空");
        }
        String role = getRole(req.getTeamId(), user.getId());
        if (role == null || (!role.equals(TeamMember.ROLE_OWNER) && !role.equals(TeamMember.ROLE_ADMIN))) {
            return ApiResult.fail(ErrorCode.FORBIDDEN, "无权移除成员");
        }
        String targetRole = getRole(req.getTeamId(), req.getUserId());
        if (targetRole == null) {
            return ApiResult.fail(ErrorCode.NOT_FOUND, "成员不存在");
        }
        if (TeamMember.ROLE_OWNER.equals(targetRole)) {
            return ApiResult.fail(ErrorCode.FORBIDDEN, "不能移除所有者");
        }
        if (role.equals(TeamMember.ROLE_ADMIN)) {
            if (TeamMember.ROLE_ADMIN.equals(targetRole)) {
                return ApiResult.fail(ErrorCode.FORBIDDEN, "管理员不能移除其他管理员");
            }
            if (req.getUserId().equals(user.getId())) {
                return ApiResult.fail(ErrorCode.FORBIDDEN, "管理员不能移除自己");
            }
        }
        teamMemberMapper.update(null, new com.baomidou.mybatisplus.core.conditions.update.UpdateWrapper<TeamMember>()
                .eq("id", getMemberId(req.getTeamId(), req.getUserId()))
                .set("deleted", 1).set("update_time", now()).set("update_by", user.getId()));
        return ApiResult.ok("移除成功");
    }

    /** 移交所有权：新 owner 升权、旧 owner 降权、团队 owner 变更三步必须原子 */
    @Transactional(rollbackFor = Exception.class)
    public ApiResult<?> transferOwnership(TeamMember.TransferRequest req, User user) {
        if (req == null || req.getTeamId() == null || req.getTeamId().isEmpty()
                || req.getNewOwnerUsername() == null || req.getNewOwnerUsername().trim().isEmpty()) {
            return ApiResult.fail(ErrorCode.PARAM_INVALID, "参数不能为空");
        }
        String role = getRole(req.getTeamId(), user.getId());
        if (role == null || !role.equals(TeamMember.ROLE_OWNER)) {
            return ApiResult.fail(ErrorCode.FORBIDDEN, "只有所有者可以移交团队");
        }
        User newOwner = findUserByUsername(req.getNewOwnerUsername().trim());
        if (newOwner == null) {
            return ApiResult.fail(ErrorCode.NOT_FOUND, "用户不存在: " + req.getNewOwnerUsername());
        }
        String now = now();
        String targetMemberId = getMemberId(req.getTeamId(), newOwner.getId());
        if (targetMemberId == null) {
            TeamMember member = new TeamMember();
            member.setId(uuid());
            member.setTeamId(req.getTeamId());
            member.setUserId(newOwner.getId());
            member.setRole(TeamMember.ROLE_OWNER);
            member.setCreateTime(now);
            member.setCreateBy(user.getId());
            member.setUpdateTime(now);
            member.setUpdateBy(user.getId());
            member.setDeleted(false);
            teamMemberMapper.insert(member);
        } else {
            teamMemberMapper.update(null, new com.baomidou.mybatisplus.core.conditions.update.UpdateWrapper<TeamMember>()
                    .eq("id", targetMemberId).set("role", TeamMember.ROLE_OWNER)
                    .set("update_time", now).set("update_by", user.getId()));
        }
        String oldOwnerMemberId = getMemberId(req.getTeamId(), user.getId());
        if (oldOwnerMemberId != null) {
            teamMemberMapper.update(null, new com.baomidou.mybatisplus.core.conditions.update.UpdateWrapper<TeamMember>()
                    .eq("id", oldOwnerMemberId).set("role", TeamMember.ROLE_ADMIN)
                    .set("update_time", now).set("update_by", user.getId()));
        }
        teamMapper.update(null, new com.baomidou.mybatisplus.core.conditions.update.UpdateWrapper<Team>()
                .eq("id", req.getTeamId()).set("owner_id", newOwner.getId())
                .set("update_time", now).set("update_by", user.getId()));
        return ApiResult.ok("移交成功");
    }

    // ==================== 权限校验 ====================

    public String getRole(String teamId, String userId) {
        TeamMember m = teamMemberMapper.selectOne(new QueryWrapper<TeamMember>()
                .eq("team_id", teamId).eq("user_id", userId).eq("deleted", 0));
        return m != null ? m.getRole() : null;
    }

    public boolean isMember(String teamId, String userId) {
        return getRole(teamId, userId) != null;
    }

    public boolean canWrite(String teamId, String userId) {
        String role = getRole(teamId, userId);
        return role != null && !role.equals(TeamMember.ROLE_READONLY);
    }

    public boolean isAdminOrAbove(String teamId, String userId) {
        String role = getRole(teamId, userId);
        return TeamMember.ROLE_OWNER.equals(role) || TeamMember.ROLE_ADMIN.equals(role);
    }

    public boolean isOwner(String teamId, String userId) {
        return TeamMember.ROLE_OWNER.equals(getRole(teamId, userId));
    }

    private String getMemberId(String teamId, String userId) {
        TeamMember m = teamMemberMapper.selectOne(new QueryWrapper<TeamMember>()
                .eq("team_id", teamId).eq("user_id", userId).eq("deleted", 0)
                .select("id"));
        return m != null ? m.getId() : null;
    }

    private boolean isValidRole(String role) {
        return TeamMember.ROLE_OWNER.equals(role) || TeamMember.ROLE_ADMIN.equals(role)
                || TeamMember.ROLE_READWRITE.equals(role) || TeamMember.ROLE_READONLY.equals(role);
    }

    private boolean isHigherRole(String roleA, String roleB) {
        return roleLevel(roleA) > roleLevel(roleB);
    }

    private int roleLevel(String role) {
        return switch (role) {
            case TeamMember.ROLE_OWNER -> 4;
            case TeamMember.ROLE_ADMIN -> 3;
            case TeamMember.ROLE_READWRITE -> 2;
            case TeamMember.ROLE_READONLY -> 1;
            default -> 0;
        };
    }

    private static String str(Object o) {
        return o == null ? null : o.toString();
    }
}
