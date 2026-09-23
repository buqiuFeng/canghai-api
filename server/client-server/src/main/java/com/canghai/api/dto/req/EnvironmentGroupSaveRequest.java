package com.canghai.api.dto.req;

import jakarta.validation.constraints.NotBlank;
import lombok.Data;

/**
 * 新增环境分组请求体（POST /api/v1/environment/groups/save）。
 */
@Data
public class EnvironmentGroupSaveRequest implements DataModeAware {

    /** 项目 ID，空表示个人空间 */
    private String projectId;

    /** 分组名称（必填） */
    @NotBlank(message = "缺少必填字段 name")
    private String name;

    private Integer sortOrder;

    /** 是否展开（客户端以布尔值传递） */
    private Boolean expanded;

    /** 操作人 ID */
    private String userId;

    /** 数据模式：online（默认）/ offline */
    private String mode;
}
