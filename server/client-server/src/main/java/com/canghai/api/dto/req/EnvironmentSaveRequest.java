package com.canghai.api.dto.req;

import jakarta.validation.constraints.NotBlank;
import lombok.Data;

/**
 * 新增环境请求体（POST /api/v1/environment/save）。
 */
@Data
public class EnvironmentSaveRequest implements DataModeAware {

    /** 项目 ID，空表示个人空间 */
    private String projectId;

    /** 环境名称（必填） */
    @NotBlank(message = "缺少必填字段 name")
    private String name;

    /** 所属分组 ID */
    private String groupId;

    /** 排序号 */
    private Integer sortOrder;

    /** 操作人 ID */
    private String userId;

    /** 数据模式：online（默认）/ offline */
    private String mode;
}
