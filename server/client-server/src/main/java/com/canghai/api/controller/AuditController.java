package com.canghai.api.controller;

import com.canghai.api.common.ApiResult;
import com.canghai.api.dto.AuditQuery;
import com.canghai.api.entity.User;
import com.canghai.api.service.AuditService;
import com.canghai.api.util.RequestContext;
import jakarta.servlet.http.HttpServletRequest;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.web.bind.annotation.PostMapping;
import org.springframework.web.bind.annotation.RequestBody;
import org.springframework.web.bind.annotation.RequestMapping;
import org.springframework.web.bind.annotation.RestController;

/**
 * 审计路由：/api/v1/audit/**
 */
@RestController
@RequestMapping("/api/v1/audit")
public class AuditController extends BaseController {

    @Autowired
    private AuditService auditService;

    /**
     * 查询审计日志
     * POST /api/v1/audit/query
     * 登录校验由 AuthAspect 切面统一处理，当前用户从 RequestContext 获取。
     */
    @PostMapping("/query")
    public ApiResult<?> query(HttpServletRequest req, @RequestBody AuditQuery q) {
        try {
            User user = RequestContext.getUser();
            if (q == null) {
                q = new AuditQuery();
            }
            return auditService.query(q, user);
        } finally {
            clear();
        }
    }
}
