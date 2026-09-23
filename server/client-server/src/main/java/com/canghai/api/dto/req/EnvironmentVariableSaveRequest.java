package com.canghai.api.dto.req;

import jakarta.validation.constraints.NotBlank;
import lombok.Data;

/**
 * 新增/保存环境变量请求体（POST /api/v1/environment/variables/save）。
 * 携带 id 时按更新处理，否则新建。
 */
@Data
public class EnvironmentVariableSaveRequest implements DataModeAware {

    /** 变量 ID；为空表示新建 */
    private String id;

    /** 所属环境 ID（必填） */
    @NotBlank(message = "缺少必填字段 environmentId/key")
    private String environmentId;

    /** 变量名（必填，对应列 var_key） */
    @NotBlank(message = "缺少必填字段 environmentId/key")
    private String key;

    private String value;

    private Boolean enabled;

    private Integer sortOrder;

    /** 操作人 ID */
    private String userId;

    /** 数据模式：online（默认）/ offline */
    private String mode;
}
