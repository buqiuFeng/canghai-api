package com.canghai.api.dto.req;

import lombok.Data;

/**
 * 令牌请求体（POST /api/v1/auth/me）。
 */
@Data
public class TokenRequest {

    /** JWT 令牌 */
    private String token;
}
