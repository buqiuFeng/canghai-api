package com.canghai.api.controller;

import com.canghai.api.common.ApiResult;
import org.springframework.web.bind.annotation.GetMapping;
import org.springframework.web.bind.annotation.RestController;

/**
 * 连通性探测 — 无需鉴权
 */
@RestController
public class CommonController {

    @GetMapping("/api/v1/ping")
    public ApiResult<Boolean> ping() {
        return ApiResult.ok(true);
    }

}
