package com.canghai.api.service;

import com.canghai.api.common.ApiResult;
import com.canghai.api.common.ErrorCode;
import com.canghai.api.dto.req.CategoryBatchSaveRequest;
import com.canghai.api.dto.req.ProjectIdRequest;
import com.canghai.api.entity.Category;
import com.canghai.api.entity.User;
import com.canghai.api.mapper.CategoryMapper;
import com.baomidou.mybatisplus.core.conditions.update.UpdateWrapper;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.stereotype.Service;
import org.springframework.transaction.annotation.Transactional;

import java.util.ArrayList;
import java.util.HashSet;
import java.util.List;
import java.util.Set;

/**
 * 分类管理。路由前缀：/api/category
 */
@Service
public class CategoryService extends BaseService {

    @Autowired
    private CategoryMapper categoryMapper;


    public ApiResult<List<Category>> list(ProjectIdRequest req, User user) {
        String projectId = req != null ? req.getProjectId() : null;
        if (projectId == null || projectId.isBlank()) return ApiResult.fail(ErrorCode.PARAM_INVALID, "缺少 projectId");
        if (!isProjectAccessible(projectId, user)) {
            return ApiResult.fail(ErrorCode.FORBIDDEN, "无权访问该项目");
        }
        List<Category> rows = categoryMapper.selectList(
                qw(Category.class).eq("project_id", projectId).eq("deleted", 0).orderByAsc("sort_order"));
        return ApiResult.ok(rows);
    }

    public ApiResult<Category> create(Category req, User user) {
        if (req == null || req.getProjectId() == null || req.getName() == null) {
            return ApiResult.fail(ErrorCode.PARAM_INVALID, "缺少必填字段 projectId/name");
        }
        if (!isProjectAccessible(req.getProjectId(), user)) {
            return ApiResult.fail(ErrorCode.FORBIDDEN, "无权访问该项目");
        }
        String now = now();
        String id = req.getId() != null && !req.getId().isBlank() ? req.getId() : uuid();
        Category c = new Category();
        c.setId(id);
        c.setProjectId(req.getProjectId());
        c.setName(req.getName());
        c.setParentId(req.getParentId());
        c.setSortOrder(req.getSortOrder() != null ? req.getSortOrder() : 0);
        c.setExpanded(req.getExpanded() != null && req.getExpanded());
        c.setCreateTime(now);
        c.setUpdateTime(now);
        c.setDeleted(false);
        categoryMapper.insert(c);
        c.setCreateTime(now);
        c.setUpdateTime(now);
        audit(req.getProjectId(), "create", "category", id, req.getName(), c, user);
        return ApiResult.ok(c);
    }

    public ApiResult<?> update(Category req, User user) {
        if (req == null || req.getId() == null) return ApiResult.fail(ErrorCode.PARAM_INVALID, "缺少 id");
        String pid = resolveAndCheckProject(req.getProjectId(), req.getId(), categoryMapper, user);
        if ("-".equals(pid)) return ApiResult.fail(ErrorCode.FORBIDDEN, "无权访问该项目");
        // 审计 before 快照：必须在 UPDATE 之前读取
        Object beforeSnapshot = snapshot(categoryMapper, req.getId());
        String now = now();
        categoryMapper.update(null, new UpdateWrapper<Category>().eq("id", req.getId())
                .set("name", req.getName())
                .set("parent_id", req.getParentId())
                .set("sort_order", req.getSortOrder())
                .set("expanded", req.getExpanded() != null && req.getExpanded())
                .set("update_time", now));
        req.setUpdateTime(now);
        audit(pid, "update", "category", req.getId(), req.getName(), beforeSnapshot, req, user);
        return ApiResult.ok(req);
    }

    public ApiResult<?> delete(Category req, User user) {
        if (req == null || req.getId() == null) return ApiResult.fail(ErrorCode.PARAM_INVALID, "缺少 id");
        String pid = resolveAndCheckProject(req.getProjectId(), req.getId(), categoryMapper, user);
        if ("-".equals(pid)) return ApiResult.fail(ErrorCode.FORBIDDEN, "无权访问该项目");
        Object beforeSnapshot = snapshot(categoryMapper, req.getId());
        softDelete(categoryMapper, req.getId());
        audit(pid, "delete", "category", req.getId(), "", beforeSnapshot, null, user);
        return ApiResult.ok();
    }

    /**
     * 批量保存（同步用）— 全量同步语义：按 id upsert，并软删除该 project 下客户端未上报的旧记录。
     */
    @Transactional(rollbackFor = Exception.class)
    public ApiResult<?> batchSave(CategoryBatchSaveRequest req, User user) {
        if (req == null) return ApiResult.fail(ErrorCode.PARAM_INVALID, "请求体解析失败");
        String projectId = req.getProjectId();
        if (projectId == null || projectId.isBlank()) projectId = req.getWorkspaceId();
        if (projectId == null || projectId.isBlank()) return ApiResult.fail(ErrorCode.PARAM_INVALID, "缺少 projectId");
        if (!isProjectAccessible(projectId, user)) {
            return ApiResult.fail(ErrorCode.FORBIDDEN, "无权访问该项目");
        }
        List<Category> items = req.getItems() != null ? req.getItems() : new ArrayList<>();

        String now = now();
        Set<String> incomingIds = new HashSet<>();
        for (Category c : items) {
            if (c == null || c.getId() == null) continue;
            incomingIds.add(c.getId());
            Category existing = categoryMapper.selectById(c.getId());
            Category entity = new Category();
            entity.setId(c.getId());
            entity.setProjectId(projectId);
            entity.setName(c.getName());
            entity.setParentId(c.getParentId());
            entity.setSortOrder(c.getSortOrder() != null ? c.getSortOrder() : 0);
            entity.setExpanded(c.getExpanded() != null && c.getExpanded());
            entity.setCreateTime(c.getCreateTime() != null ? c.getCreateTime() : now);
            entity.setUpdateTime(c.getUpdateTime() != null ? c.getUpdateTime() : now);
            entity.setDeleted(false);
            if (existing == null) {
                categoryMapper.insert(entity);
            } else {
                categoryMapper.update(null, new UpdateWrapper<Category>().eq("id", c.getId())
                        .set("name", entity.getName())
                        .set("parent_id", entity.getParentId())
                        .set("sort_order", entity.getSortOrder())
                        .set("expanded", entity.getExpanded())
                        .set("update_time", entity.getUpdateTime()));
            }
        }

        List<Category> stale = categoryMapper.selectList(
                qw(Category.class).eq("project_id", projectId).eq("deleted", 0).select("id"));
        for (Category r : stale) {
            if (!incomingIds.contains(r.getId())) {
                softDelete(categoryMapper, r.getId());
            }
        }
        return ApiResult.ok();
    }
}
