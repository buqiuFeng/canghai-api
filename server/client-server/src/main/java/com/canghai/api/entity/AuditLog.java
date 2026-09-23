package com.canghai.api.entity;

import com.baomidou.mybatisplus.annotation.IdType;
import com.baomidou.mybatisplus.annotation.TableId;
import com.baomidou.mybatisplus.annotation.TableName;
import lombok.Data;

/**
 * 操作审计日志
 */
@Data
@TableName("ch_audit_logs")
public class AuditLog {

    @TableId(type = IdType.INPUT)
    private String id;
    private String projectId;
    private String userId;
    private String username;
    private String action;
    private String entityType;
    private String entityId;
    private String entityName;
    private String beforeJson;
    private String afterJson;
    private String ip;
    private String createTime;
    private Boolean deleted;
}
