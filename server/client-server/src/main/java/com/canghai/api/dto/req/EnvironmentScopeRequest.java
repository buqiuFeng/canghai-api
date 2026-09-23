package com.canghai.api.dto.req;

import lombok.Data;

/**
 * 环境/环境分组的查询范围：项目 + 数据模式。
 */
@Data
public class EnvironmentScopeRequest implements DataModeAware {

    /** 项目 ID，空串表示个人空间 */
    private String projectId;

    /** 数据模式：online（默认）/ offline */
    private String mode;
}
