package com.canghai.api.dto.req;

import jakarta.validation.constraints.NotBlank;
import lombok.Data;

/**
 * 更新环境分组请求体（POST /api/v1/environment/groups/update）。
 */
@Data
public class EnvironmentGroupUpdateRequest implements DataModeAware {

    /** 分组 ID（必填） */
    @NotBlank(message = "缺少 id")
    private String id;

    /** 所属项目 ID，用于越权校验 */
    private String projectId;

    private String name;

    private Integer sortOrder;

    /** 是否展开（客户端以布尔值传递） */
    private Boolean expanded;

    /** 操作人 ID */
    private String userId;

    /** 数据模式：online（默认）/ offline */
    private String mode;
}
