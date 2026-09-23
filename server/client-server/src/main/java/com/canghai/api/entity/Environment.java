package com.canghai.api.entity;

import com.baomidou.mybatisplus.annotation.IdType;
import com.baomidou.mybatisplus.annotation.TableId;
import com.baomidou.mybatisplus.annotation.TableName;
import lombok.Data;

/**
 * 环境
 */
@Data
@TableName("ch_environments")
public class Environment {

    @TableId(type = IdType.INPUT)
    private String id;
    private String projectId;
    private String name;
    private String groupId;
    private Boolean isActive;
    private Integer sortOrder;
    private String dataMode;
    private String createTime;
    private String createBy;
    private String updateTime;
    private String updateBy;
    private Boolean deleted;
    /** 服务端行变更时间（增量同步游标，由 DB 自动维护；Java 侧只读） */
    private String serverUpdateTime;
    /** 服务端版本号（由 DB 触发器自增；Java 侧只读，用于冲突判定） */
    private Integer syncVersion;
}
