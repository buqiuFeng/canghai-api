package com.canghai.api.dto.req;

import lombok.Data;

/**
 * 仅需要 teamId 的通用请求体（POST /api/v1/project-member/by-team）。
 */
@Data
public class TeamIdRequest {

    /** 团队 ID */
    private String teamId;
}
