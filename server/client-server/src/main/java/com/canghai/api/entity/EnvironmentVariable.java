package com.canghai.api.entity;

import com.baomidou.mybatisplus.annotation.IdType;
import com.baomidou.mybatisplus.annotation.TableField;
import com.baomidou.mybatisplus.annotation.TableId;
import com.baomidou.mybatisplus.annotation.TableName;
import com.fasterxml.jackson.annotation.JsonProperty;
import lombok.Data;

/**
 * 环境变量
 *
 * <p>说明：数据库列名为 {@code var_key}（避免 MySQL 保留字 key），JSON 对外仍为 {@code key}。
 *
 * <p>注意：Java 属性名必须写成 {@code varKey} 而不是 {@code key}。MyBatis-Plus 在
 * 「属性名 ≠ 列名转驼峰」时会自动补别名，生成 {@code var_key AS key}；而 {@code key}
 * 是保留字、别名未加反引号 → MySQL 直接语法错误（环境变量读写/同步全部失败）。
 * 属性与列名同形后不再生成别名；对外 JSON 由 {@link JsonProperty} 保持不变。
 */
@Data
@TableName("ch_environment_variables")
public class EnvironmentVariable {

    @TableId(type = IdType.INPUT)
    private String id;
    private String environmentId;
    @TableField("var_key")
    @JsonProperty("key")
    private String varKey;
    private String value;
    private Boolean enabled;
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
