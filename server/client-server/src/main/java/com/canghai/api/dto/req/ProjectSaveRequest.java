package com.canghai.api.dto.req;

import jakarta.validation.constraints.NotBlank;
import lombok.Data;

/**
 * 保存项目请求体（POST /api/v1/project/save）。
 * id 为空表示新建。
 */
@Data
public class ProjectSaveRequest {

    /** 项目 ID；为空表示新建 */
    private String id;

    /** 项目名称（必填） */
    @NotBlank(message = "项目名称不能为空")
    private String name;

    /** 项目描述 */
    private String description;
}
