package com.canghai.api.dto.req;

import jakarta.validation.constraints.NotBlank;
import lombok.Data;

/**
 * 环境变量列表请求体（POST /api/v1/environment/variables/list）。
 */
@Data
public class EnvironmentVariableListRequest implements DataModeAware {

    /** 所属环境 ID（必填） */
    @NotBlank(message = "缺少 environmentId")
    private String environmentId;

    /** 数据模式：online（默认）/ offline */
    private String mode;
}
