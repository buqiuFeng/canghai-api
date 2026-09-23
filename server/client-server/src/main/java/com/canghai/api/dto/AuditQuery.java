package com.canghai.api.dto;

import lombok.Data;

/**
 * 审计日志查询条件（对应 POST /api/audit/query 的请求体）
 */
@Data
public class AuditQuery {

    /** 所属项目 ID（必填） */
    private String projectId;

    /** 实体类型过滤，如 project / request / category */
    private String entityType;

    /** 实体 ID 过滤 */
    private String entityId;

    /** 关键字（按实体名称 / 操作人模糊匹配） */
    private String keyword;

    /** 分页页码，从 0 开始 */
    private int page = 0;

    /** 每页条数，默认 20，最大 100 */
    private int size = 20;
}
