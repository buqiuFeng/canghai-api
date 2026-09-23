package com.canghai.api.service;

import com.canghai.api.common.ApiResult;
import com.canghai.api.common.ErrorCode;
import com.canghai.api.dto.req.IdRequest;
import com.canghai.api.dto.req.ProjectSaveRequest;
import com.canghai.api.entity.Project;
import com.canghai.api.entity.ProjectMember;
import com.canghai.api.entity.User;
import com.canghai.api.mapper.ProjectMapper;
import com.canghai.api.mapper.ProjectMemberMapper;
import com.baomidou.mybatisplus.core.conditions.update.UpdateWrapper;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.stereotype.Service;
import org.springframework.transaction.annotation.Transactional;

/**
 * 项目服务（实时接口）。路由前缀：/api/project
 */
@Service
public class ProjectService extends BaseService {

    @Autowired
    private ProjectMapper projectMapper;
    @Autowired
    private ProjectMemberMapper projectMemberMapper;


    /** 保存项目：新建时同时写入 owner 成员，需事务保证原子性 */
    @Transactional(rollbackFor = Exception.class)
    public ApiResult<?> save(ProjectSaveRequest req, User user) {
        if (req == null) req = new ProjectSaveRequest();
        String id = req.getId();
        String name = req.getName();
        if (name == null || name.isBlank()) {
            return ApiResult.fail(ErrorCode.PARAM_INVALID, "项目名称不能为空");
        }
        String description = req.getDescription();
        if (description == null) description = "";
        String now = now();

        boolean isNew = (id == null || id.isBlank());
        if (!isNew) {
            Project existing = projectMapper.selectById(id);
            isNew = (existing == null);
        }
        if (isNew && (id == null || id.isBlank())) {
            id = uuid();
        }

        if (isNew) {
            Project p = new Project();
            p.setId(id);
            p.setName(name);
            p.setDescription(description);
            p.setCreateTime(now);
            p.setCreateBy(user.getId());
            p.setUpdateTime(now);
            p.setUpdateBy(user.getId());
            p.setDeleted(false);
            projectMapper.insert(p);
            ProjectMember pm = new ProjectMember();
            pm.setId(uuid());
            pm.setProjectId(id);
            pm.setMemberType(ProjectMember.TYPE_USER);
            pm.setMemberId(user.getId());
            // 同步落库成员名称，供结构类同步下发到客户端（离线展示用）
            pm.setMemberName(user.getUsername());
            pm.setRole(ProjectMember.ROLE_OWNER);
            pm.setCreateTime(now);
            pm.setCreateBy(user.getId());
            pm.setUpdateTime(now);
            pm.setUpdateBy(user.getId());
            pm.setDeleted(false);
            projectMemberMapper.insert(pm);
        } else {
            if (!isProjectAccessible(id, user)) {
                return ApiResult.fail(ErrorCode.FORBIDDEN, "无权修改该项目");
            }
            if (description.isBlank()) {
                projectMapper.update(null, new UpdateWrapper<Project>().eq("id", id)
                        .set("name", name).set("update_time", now).set("update_by", user.getId()));
            } else {
                projectMapper.update(null, new UpdateWrapper<Project>().eq("id", id)
                        .set("name", name).set("description", description)
                        .set("update_time", now).set("update_by", user.getId()));
            }
        }

        Project p = new Project();
        p.setId(id);
        p.setName(name);
        p.setDescription(description);
        p.setCreateTime(now);
        p.setCreateBy(user.getId());
        p.setUpdateTime(now);
        p.setUpdateBy(user.getId());
        p.setDeleted(false);
        return ApiResult.ok(p);
    }

    /** 删除项目：项目与其成员记录一并软删，需事务保证原子性 */
    @Transactional(rollbackFor = Exception.class)
    public ApiResult<?> delete(IdRequest req, User user) {
        String id = req != null ? req.getId() : null;
        if (id == null || id.isBlank()) {
            return ApiResult.fail(ErrorCode.PARAM_INVALID, "项目 id 不能为空");
        }
        if (!isProjectAccessible(id, user)) {
            return ApiResult.fail(ErrorCode.FORBIDDEN, "无权删除该项目");
        }
        String now = now();
        projectMapper.update(null, new UpdateWrapper<Project>().eq("id", id)
                .set("deleted", 1).set("update_time", now).set("update_by", user.getId()));
        projectMemberMapper.update(null, new UpdateWrapper<ProjectMember>().eq("project_id", id)
                .set("deleted", 1).set("update_time", now).set("update_by", user.getId()));
        return ApiResult.ok("项目已删除");
    }
}
