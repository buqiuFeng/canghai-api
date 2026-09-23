package com.canghai.api.dto.req;

import jakarta.validation.constraints.NotBlank;
import lombok.Data;

/**
 * 更新环境请求体（POST /api/v1/environment/update）。
 */
@Data
public class EnvironmentUpdateRequest implements DataModeAware {

    /** 环境 ID（必填） */
    @NotBlank(message = "缺少 id")
    private String id;

    /** 所属项目 ID，用于越权校验 */
    private String projectId;

    private String name;

    private String groupId;

    private Integer sortOrder;

    /** 操作人 ID */
    private String userId;

    /** 数据模式：online（默认）/ offline */
    private String mode;
}
