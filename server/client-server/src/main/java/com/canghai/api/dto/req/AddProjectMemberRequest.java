package com.canghai.api.dto.req;

import jakarta.validation.constraints.NotBlank;
import lombok.Data;

/**
 * 添加项目成员请求体（POST /api/v1/project-member/add）。
 */
@Data
public class AddProjectMemberRequest {

    /** 项目 ID（必填） */
    @NotBlank(message = "缺少必填参数 projectId")
    private String projectId;

    /** 成员类型：team / user（必填） */
    @NotBlank(message = "缺少必填参数 memberType")
    private String memberType;

    /** 成员 ID；user 类型下可传用户名（必填） */
    @NotBlank(message = "缺少必填参数 memberId")
    private String memberId;

    /** 角色；team 类型强制为 inherit，user 类型默认 readwrite */
    private String role;
}
