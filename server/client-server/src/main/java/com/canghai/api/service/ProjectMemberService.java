package com.canghai.api.service;

import com.canghai.api.common.ApiResult;
import com.canghai.api.common.ErrorCode;
import com.canghai.api.dto.req.AddProjectMemberRequest;
import com.canghai.api.dto.req.ProjectIdRequest;
import com.canghai.api.dto.req.RemoveProjectMemberRequest;
import com.canghai.api.dto.req.TeamIdRequest;
import com.canghai.api.entity.Project;
import com.canghai.api.entity.ProjectMember;
import com.canghai.api.entity.TeamMember;
import com.canghai.api.entity.User;
import com.canghai.api.mapper.ProjectMapper;
import com.canghai.api.mapper.ProjectMemberMapper;
import com.canghai.api.mapper.TeamMapper;
import com.canghai.api.mapper.TeamMemberMapper;
import com.canghai.api.mapper.UserMapper;
import com.baomidou.mybatisplus.core.conditions.query.QueryWrapper;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.stereotype.Service;

import java.util.ArrayList;
import java.util.HashSet;
import java.util.LinkedHashMap;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Map;
import java.util.Set;

/**
 * 项目成员服务。团队与项目已脱钩，关联通过 ch_project_members 表达。
 * 路由前缀：/api/project-member
 */
@Service
public class ProjectMemberService extends BaseService {

    @Autowired
    private ProjectMemberMapper projectMemberMapper;
    @Autowired
    private ProjectMapper projectMapper;
    @Autowired
    private TeamMemberMapper teamMemberMapper;
    @Autowired
    private UserMapper userMapper;
    @Autowired
    private TeamMapper teamMapper;


    private boolean isValidMemberType(String t) {
        return ProjectMember.TYPE_TEAM.equals(t) || ProjectMember.TYPE_USER.equals(t);
    }

    private boolean isValidRole(String r) {
        return ProjectMember.ROLE_OWNER.equals(r) || ProjectMember.ROLE_ADMIN.equals(r)
                || ProjectMember.ROLE_READWRITE.equals(r) || ProjectMember.ROLE_READONLY.equals(r)
                || ProjectMember.ROLE_INHERIT.equals(r);
    }

    public ApiResult<?> addMember(AddProjectMemberRequest req, User user) {
        if (req == null) req = new AddProjectMemberRequest();
        String projectId = req.getProjectId();
        String memberType = req.getMemberType();
        String memberId = req.getMemberId();
        if (projectId == null || projectId.isBlank() || memberType == null || !isValidMemberType(memberType)
                || memberId == null || memberId.isBlank()) {
            return ApiResult.fail(ErrorCode.PARAM_INVALID, "projectId / memberType(team|user) / memberId 不能为空");
        }
        String myRole = getUserRoleInProject(projectId, user);
        if (!ProjectMember.ROLE_OWNER.equals(myRole) && !ProjectMember.ROLE_ADMIN.equals(myRole)) {
            return ApiResult.fail(ErrorCode.FORBIDDEN, "仅项目所有者或管理员可添加成员");
        }
        String role = req.getRole();
        if (ProjectMember.TYPE_TEAM.equals(memberType)) {
            role = ProjectMember.ROLE_INHERIT;
        } else {
            if (role == null || role.isBlank()) role = ProjectMember.ROLE_READWRITE;
            if (!isValidRole(role)) return ApiResult.fail(ErrorCode.PARAM_INVALID, "非法的角色: " + role);
        }

        // 成员名称随成员关系一并落库（ch_project_members.member_name），
        // 使结构类同步能把名称带到客户端，离线展示无需回查用户/团队表。
        String memberName;
        if (ProjectMember.TYPE_USER.equals(memberType)) {
            com.canghai.api.entity.User target = userMapper.selectOne(
                    qw(User.class).eq("username", memberId).eq("deleted", 0).last("LIMIT 1"));
            if (target == null) {
                return ApiResult.fail(ErrorCode.NOT_FOUND, "用户不存在或已停用: " + memberId);
            }
            memberId = target.getId();
            memberName = target.getUsername();
        } else {
            com.canghai.api.entity.Team team = teamMapper.selectOne(
                    qw(com.canghai.api.entity.Team.class).eq("id", memberId).eq("deleted", 0).last("LIMIT 1"));
            memberName = team != null ? team.getName() : "";
        }

        if (ProjectMember.TYPE_TEAM.equals(memberType)) {
            Long existingTeam = projectMemberMapper.selectCount(
                    qw(ProjectMember.class).eq("project_id", projectId)
                            .eq("member_type", "team").eq("deleted", 0));
            if (existingTeam != null && existingTeam > 0) {
                return ApiResult.fail(ErrorCode.CONFLICT, "项目已关联团队，单团队模式下不可重复关联");
            }
        }

        String now = now();
        ProjectMember exist = projectMemberMapper.selectOne(
                qw(ProjectMember.class).eq("project_id", projectId)
                        .eq("member_type", memberType).eq("member_id", memberId).last("LIMIT 1"));
        if (exist != null) {
            projectMemberMapper.update(null, new com.baomidou.mybatisplus.core.conditions.update.UpdateWrapper<ProjectMember>()
                    .eq("id", exist.getId()).set("deleted", 0).set("role", role)
                    .set("member_name", memberName)
                    .set("update_time", now).set("update_by", user.getId()));
            return ApiResult.ok("成员已添加");
        }

        ProjectMember pm = new ProjectMember();
        pm.setId(uuid());
        pm.setProjectId(projectId);
        pm.setMemberType(memberType);
        pm.setMemberId(memberId);
        pm.setMemberName(memberName);
        pm.setRole(role);
        pm.setCreateTime(now);
        pm.setCreateBy(user.getId());
        pm.setUpdateTime(now);
        pm.setUpdateBy(user.getId());
        pm.setDeleted(false);
        projectMemberMapper.insert(pm);
        return ApiResult.ok("成员已添加");
    }

    public ApiResult<?> removeMember(RemoveProjectMemberRequest req, User user) {
        if (req == null) req = new RemoveProjectMemberRequest();
        String projectId = req.getProjectId();
        String memberType = req.getMemberType();
        String memberId = req.getMemberId();
        if (projectId == null || projectId.isBlank() || memberType == null || !isValidMemberType(memberType)
                || memberId == null || memberId.isBlank()) {
            return ApiResult.fail(ErrorCode.PARAM_INVALID, "projectId / memberType / memberId 不能为空");
        }
        boolean removingSelf = ProjectMember.TYPE_USER.equals(memberType) && memberId.equals(user.getId());
        if (!removingSelf) {
            String myRole = getUserRoleInProject(projectId, user);
            if (!ProjectMember.ROLE_OWNER.equals(myRole) && !ProjectMember.ROLE_ADMIN.equals(myRole)) {
                return ApiResult.fail(ErrorCode.FORBIDDEN, "仅项目所有者或管理员可移除其他成员");
            }
        }
        projectMemberMapper.update(null, new com.baomidou.mybatisplus.core.conditions.update.UpdateWrapper<ProjectMember>()
                .eq("project_id", projectId).eq("member_type", memberType).eq("member_id", memberId)
                .set("deleted", 1).set("update_time", now()).set("update_by", user.getId()));
        return ApiResult.ok(removingSelf ? "已退出项目" : "成员已移除");
    }

    /** 取当前用户在指定项目中的有效角色 */
    private String getUserRoleInProject(String projectId, User user) {
        ProjectMember m = projectMemberMapper.selectOne(
                qw(ProjectMember.class).eq("project_id", projectId)
                        .eq("member_type", "user").eq("member_id", user.getId()).eq("deleted", 0).last("LIMIT 1"));
        if (m != null) {
            String role = m.getRole();
            if (role != null && !role.isBlank()) return role;
        }
        Long t = projectMemberMapper.selectCount(
                qw(ProjectMember.class).eq("project_id", projectId).eq("member_type", "team")
                        .apply("member_id IN (SELECT team_id FROM ch_team_members WHERE user_id = {0} AND deleted = 0)", user.getId())
                        .eq("deleted", 0));
        if (t != null && t > 0) return ProjectMember.ROLE_INHERIT;
        return null;
    }

    public ApiResult<?> listMembers(ProjectIdRequest req, User user) {
        String projectId = req != null ? req.getProjectId() : null;
        if (projectId == null || projectId.isBlank()) {
            return ApiResult.fail(ErrorCode.PARAM_INVALID, "projectId 不能为空");
        }
        if (!isProjectAccessible(projectId, user)) {
            return ApiResult.fail(ErrorCode.FORBIDDEN, "无权访问该项目");
        }
        List<Map<String, Object>> rows = projectMemberMapper.selectMembersWithName(projectId);
        List<ProjectMember.MemberInfo> list = new ArrayList<>();
        for (Map<String, Object> row : rows) {
            ProjectMember.MemberInfo info = new ProjectMember.MemberInfo();
            info.setId(str(row.get("id")));
            info.setProjectId(str(row.get("projectId")));
            info.setMemberType(str(row.get("memberType")));
            info.setMemberId(str(row.get("memberId")));
            info.setMemberName(str(row.get("memberName")));
            info.setRole(str(row.get("role")));
            info.setCreateTime(str(row.get("createTime")));
            list.add(info);
        }
        return ApiResult.ok(list);
    }

    public ApiResult<?> listProjectsByTeam(TeamIdRequest req, User user) {
        String teamId = req != null ? req.getTeamId() : null;
        if (teamId == null || teamId.isBlank()) {
            return ApiResult.fail(ErrorCode.PARAM_INVALID, "teamId 不能为空");
        }
        // 仅返回「该团队关联」且「当前用户可见」的项目，避免凭 teamId 越权枚举
        Set<String> visible = new HashSet<>(getVisibleProjectIds(user));
        List<Project> list = new ArrayList<>();
        for (Project p : projectMemberMapper.selectByTeam(teamId)) {
            if (p != null && visible.contains(p.getId())) {
                list.add(p);
            }
        }
        return ApiResult.ok(list);
    }

    /**
     * 当前用户拥有权限的所有 project_id（单一事实来源）。
     * 来源：1) user 类型成员直接关联；2) 用户所在团队经 team 类型成员间接关联。
     */
    public List<String> getVisibleProjectIds(User user) {
        Set<String> projectIds = new LinkedHashSet<>();
        List<ProjectMember> uRows = projectMemberMapper.selectList(
                qw(ProjectMember.class).eq("deleted", 0).eq("member_type", "user")
                        .eq("member_id", user.getId()).select("project_id"));
        for (ProjectMember r : uRows) projectIds.add(r.getProjectId());
        List<ProjectMember> tRows = projectMemberMapper.selectList(
                qw(ProjectMember.class).eq("deleted", 0).eq("member_type", "team")
                        .apply("member_id IN (SELECT team_id FROM ch_team_members WHERE user_id = {0} AND deleted = 0)", user.getId())
                        .select("project_id"));
        for (ProjectMember r : tRows) projectIds.add(r.getProjectId());
        return new ArrayList<>(projectIds);
    }

    /**
     * 当前用户拥有「可写」角色的项目 id（用于同步上传的写作用域裁剪）：
     * 1) user 类型成员，角色为 OWNER / ADMIN / READWRITE；
     * 2) team 类型成员（INHERIT），且用户在该团队角色非 READONLY（等价于 TeamService.canWrite 的语义）。
     * 只读（READONLY）成员、以及仅以 INHERIT 关联但其团队角色为 READONLY 的情况，均不在此列。
     */
    public List<String> getWritableProjectIds(User user) {
        Set<String> projectIds = new LinkedHashSet<>();
        List<ProjectMember> uRows = projectMemberMapper.selectList(
                qw(ProjectMember.class).eq("deleted", 0).eq("member_type", "user")
                        .eq("member_id", user.getId())
                        .in("role", ProjectMember.ROLE_OWNER, ProjectMember.ROLE_ADMIN, ProjectMember.ROLE_READWRITE)
                        .select("project_id"));
        for (ProjectMember r : uRows) if (r.getProjectId() != null) projectIds.add(r.getProjectId());

        List<TeamMember> tMembers = teamMemberMapper.selectList(
                qw(TeamMember.class).eq("deleted", 0).eq("user_id", user.getId())
                        .ne("role", TeamMember.ROLE_READONLY).select("team_id"));
        if (tMembers != null && !tMembers.isEmpty()) {
            Set<String> teamIds = new LinkedHashSet<>();
            for (TeamMember tm : tMembers) if (tm.getTeamId() != null) teamIds.add(tm.getTeamId());
            List<ProjectMember> tRows = projectMemberMapper.selectList(
                    qw(ProjectMember.class).eq("deleted", 0).eq("member_type", "team")
                            .in("member_id", teamIds).select("project_id"));
            for (ProjectMember r : tRows) if (r.getProjectId() != null) projectIds.add(r.getProjectId());
        }
        return new ArrayList<>(projectIds);
    }

    /**
     * 用户「曾经可见」的项目 id（**不排除成员记录已软删的项目**）。
     *
     * <p>用途：下发删除墓碑。项目删除时 {@code ProjectService.delete} 会把该项目及其
     * 全部 {@code ch_project_members} 记录一并软删，此后 {@link #getVisibleProjectIds}
     * 不再返回该 id；若墓碑范围仍取「可见项目」，客户端就再也收不到删除通知，
     * 本地缓存的项目会一直残留（表现为「服务器删了、客户端还在」）。
     *
     * <p>因此这里刻意<b>不过滤 {@code deleted}</b>，并同时保留已解散团队的历史成员关系。
     */
    public List<String> getFormerlyVisibleProjectIds(User user) {
        Set<String> projectIds = new LinkedHashSet<>();
        List<ProjectMember> uRows = projectMemberMapper.selectList(
                qw(ProjectMember.class).eq("member_type", "user")
                        .eq("member_id", user.getId()).select("project_id"));
        for (ProjectMember r : uRows) if (r.getProjectId() != null) projectIds.add(r.getProjectId());
        List<ProjectMember> tRows = projectMemberMapper.selectList(
                qw(ProjectMember.class).eq("member_type", "team")
                        .apply("member_id IN (SELECT team_id FROM ch_team_members WHERE user_id = {0})", user.getId())
                        .select("project_id"));
        for (ProjectMember r : tRows) if (r.getProjectId() != null) projectIds.add(r.getProjectId());
        return new ArrayList<>(projectIds);
    }

    /** 当前用户可见项目（无需请求体，用户即查询条件） */
    public ApiResult<?> listVisibleProjects(User user) {
        Map<String, String> map = new LinkedHashMap<>();
        List<ProjectMember> uRows = projectMemberMapper.selectList(
                qw(ProjectMember.class).eq("deleted", 0).eq("member_type", "user")
                        .eq("member_id", user.getId()).select("project_id, role"));
        for (ProjectMember r : uRows) {
            String role = r.getRole();
            if (role == null || role.isBlank()) role = ProjectMember.ROLE_READWRITE;
            map.put(r.getProjectId(), role);
        }
        List<ProjectMember> tRows = projectMemberMapper.selectList(
                qw(ProjectMember.class).eq("deleted", 0).eq("member_type", "team")
                        .apply("member_id IN (SELECT team_id FROM ch_team_members WHERE user_id = {0} AND deleted = 0)", user.getId())
                        .select("project_id"));
        for (ProjectMember r : tRows) map.putIfAbsent(r.getProjectId(), ProjectMember.ROLE_INHERIT);

        List<Map<String, String>> res = new ArrayList<>();
        for (Map.Entry<String, String> e : map.entrySet()) {
            Map<String, String> item = new LinkedHashMap<>();
            item.put("projectId", e.getKey());
            item.put("role", e.getValue());
            res.add(item);
        }
        return ApiResult.ok(res);
    }

    private static String str(Object o) {
        return o == null ? null : o.toString();
    }
}
