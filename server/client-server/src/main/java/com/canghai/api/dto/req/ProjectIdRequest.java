package com.canghai.api.dto.req;

import lombok.Data;

/**
 * 仅需要 projectId 的通用请求体（分类/接口列表、项目成员列表等）。
 */
@Data
public class ProjectIdRequest {

    /** 项目 ID */
    private String projectId;
}
