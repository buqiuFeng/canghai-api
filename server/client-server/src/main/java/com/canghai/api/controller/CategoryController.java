package com.canghai.api.controller;

import com.canghai.api.common.ApiResult;
import com.canghai.api.dto.req.CategoryBatchSaveRequest;
import com.canghai.api.dto.req.ProjectIdRequest;
import com.canghai.api.entity.Category;
import com.canghai.api.entity.User;
import com.canghai.api.service.CategoryService;
import com.canghai.api.util.RequestContext;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.web.bind.annotation.PostMapping;
import org.springframework.web.bind.annotation.RequestBody;
import org.springframework.web.bind.annotation.RequestMapping;
import org.springframework.web.bind.annotation.RestController;

/**
 * 分类路由：/api/v1/category/**
 * 登录校验由 AuthAspect 切面统一处理，当前用户从 RequestContext 获取。
 */
@RestController
@RequestMapping("/api/v1/category")
public class CategoryController extends BaseController {

    @Autowired
    private CategoryService categoryService;

    /**
     * 查询分类列表
     * POST /api/v1/category/list
     */
    @PostMapping("/list")
    public ApiResult<?> list(@RequestBody(required = false) ProjectIdRequest q) {
        try {
            User user = RequestContext.getUser();
            return categoryService.list(q, user);
        } finally {
            clear();
        }
    }

    /**
     * 新增分类
     * POST /api/v1/category/create
     */
    @PostMapping("/create")
    public ApiResult<?> create(@RequestBody(required = false) Category q) {
        try {
            User user = RequestContext.getUser();
            return categoryService.create(q, user);
        } finally {
            clear();
        }
    }

    /**
     * 修改分类
     * POST /api/v1/category/update
     */
    @PostMapping("/update")
    public ApiResult<?> update(@RequestBody(required = false) Category q) {
        try {
            User user = RequestContext.getUser();
            return categoryService.update(q, user);
        } finally {
            clear();
        }
    }

    /**
     * 删除分类
     * POST /api/v1/category/delete
     */
    @PostMapping("/delete")
    public ApiResult<?> delete(@RequestBody(required = false) Category q) {
        try {
            User user = RequestContext.getUser();
            return categoryService.delete(q, user);
        } finally {
            clear();
        }
    }

    /**
     * 批量保存分类（同步用，全量语义）
     * POST /api/v1/category/batch-save
     */
    @PostMapping("/batch-save")
    public ApiResult<?> batchSave(@RequestBody(required = false) CategoryBatchSaveRequest q) {
        try {
            User user = RequestContext.getUser();
            return categoryService.batchSave(q, user);
        } finally {
            clear();
        }
    }
}
