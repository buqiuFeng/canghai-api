package com.canghai.api.controller;

import com.canghai.api.common.ApiResult;
import com.canghai.api.entity.User;
import com.canghai.api.model.SyncData;
import com.canghai.api.service.SyncService;
import com.canghai.api.util.RequestContext;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.web.bind.annotation.PostMapping;
import org.springframework.web.bind.annotation.RequestBody;
import org.springframework.web.bind.annotation.RequestMapping;
import org.springframework.web.bind.annotation.RestController;

/**
 * 同步路由：/api/v1/sync/**
 * POST /api/v1/sync/upload → 客户端上报数据，服务端合并后返回全量
 * POST /api/v1/sync/pull   → 拉取服务端最新数据
 * 登录校验由 AuthAspect 切面统一处理，当前用户从 RequestContext 获取。
 */
@RestController
@RequestMapping("/api/v1/sync")
public class SyncController extends BaseController {

    @Autowired
    private SyncService syncService;

    /**
     * 上报同步数据
     * POST /api/v1/sync/upload
     */
    @PostMapping("/upload")
    public ApiResult<?> upload(@RequestBody(required = false) SyncData q) {
        try {
            User user = RequestContext.getUser();
            return syncService.upload(q, user);
        } finally {
            clear();
        }
    }

    /**
     * 拉取服务端数据
     * POST /api/v1/sync/pull
     */
    @PostMapping("/pull")
    public ApiResult<?> pull(@RequestBody(required = false) SyncData q) {
        try {
            User user = RequestContext.getUser();
            return syncService.pull(q, user);
        } finally {
            clear();
        }
    }
}
