package com.canghai.api.dto.req;

import jakarta.validation.constraints.NotBlank;
import lombok.Data;

/**
 * 按主键操作的通用请求体（删除 / 详情 / 启用的环境等）。
 * 未使用到的字段留空即可，便于客户端统一封装。
 */
@Data
public class IdRequest implements DataModeAware {

    /** 记录 ID（必填） */
    @NotBlank(message = "缺少 id")
    private String id;

    /** 所属项目 ID，用于越权校验；为空时按 id 反查 */
    private String projectId;

    /** 操作人 ID，用于审计字段 */
    private String userId;

    /** 数据模式：online（默认）/ offline */
    private String mode;
}
