package com.canghai.api.dto.req;

import jakarta.validation.constraints.NotBlank;
import lombok.Data;

/**
 * 更新环境变量请求体（POST /api/v1/environment/variables/update）。
 */
@Data
public class EnvironmentVariableUpdateRequest implements DataModeAware {

    /** 变量 ID（必填） */
    @NotBlank(message = "缺少 id")
    private String id;

    /** 变量名（对应列 var_key） */
    private String key;

    private String value;

    private Boolean enabled;

    private Integer sortOrder;

    /** 操作人 ID */
    private String userId;

    /** 数据模式：online（默认）/ offline */
    private String mode;
}
