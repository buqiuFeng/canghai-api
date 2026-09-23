package com.canghai.api.dto.req;

import jakarta.validation.constraints.NotBlank;
import lombok.Data;

/**
 * 移除项目成员请求体（POST /api/v1/project-member/remove）。
 */
@Data
public class RemoveProjectMemberRequest {

    /** 项目 ID（必填） */
    @NotBlank(message = "缺少必填参数 projectId")
    private String projectId;

    /** 成员类型：team / user（必填） */
    @NotBlank(message = "缺少必填参数 memberType")
    private String memberType;

    /** 成员 ID（必填） */
    @NotBlank(message = "缺少必填参数 memberId")
    private String memberId;
}
