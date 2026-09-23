package com.canghai.api.controller;

import com.canghai.api.common.ApiResult;
import com.canghai.api.dto.req.IdRequest;
import com.canghai.api.dto.req.ProjectIdRequest;
import com.canghai.api.entity.SavedRequest;
import com.canghai.api.entity.User;
import com.canghai.api.service.SavedRequestService;
import com.canghai.api.util.RequestContext;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.web.bind.annotation.PostMapping;
import org.springframework.web.bind.annotation.RequestBody;
import org.springframework.web.bind.annotation.RequestMapping;
import org.springframework.web.bind.annotation.RestController;

/**
 * 接口请求路由：/api/v1/request/**
 * 登录校验由 AuthAspect 切面统一处理，当前用户从 RequestContext 获取。
 */
@RestController
@RequestMapping("/api/v1/request")
public class RequestController extends BaseController {

    @Autowired
    private SavedRequestService savedRequestService;

    /**
     * 接口列表
     * POST /api/v1/request/list
     */
    @PostMapping("/list")
    public ApiResult<?> list(@RequestBody(required = false) ProjectIdRequest q) {
        try {
            User user = RequestContext.getUser();
            return savedRequestService.list(q, user);
        } finally {
            clear();
        }
    }

    /**
     * 接口详情
     * POST /api/v1/request/detail
     */
    @PostMapping("/detail")
    public ApiResult<?> detail(@RequestBody(required = false) IdRequest q) {
        try {
            User user = RequestContext.getUser();
            return savedRequestService.detail(q, user);
        } finally {
            clear();
        }
    }

    /**
     * 新增接口
     * POST /api/v1/request/create
     */
    @PostMapping("/create")
    public ApiResult<?> create(@RequestBody(required = false) SavedRequest q) {
        try {
            User user = RequestContext.getUser();
            return savedRequestService.create(q, user);
        } finally {
            clear();
        }
    }

    /**
     * 修改接口
     * POST /api/v1/request/update
     */
    @PostMapping("/update")
    public ApiResult<?> update(@RequestBody(required = false) SavedRequest q) {
        try {
            User user = RequestContext.getUser();
            return savedRequestService.update(q, user);
        } finally {
            clear();
        }
    }

    /**
     * 删除接口
     * POST /api/v1/request/delete
     */
    @PostMapping("/delete")
    public ApiResult<?> delete(@RequestBody(required = false) IdRequest q) {
        try {
            User user = RequestContext.getUser();
            return savedRequestService.delete(q, user);
        } finally {
            clear();
        }
    }
}
