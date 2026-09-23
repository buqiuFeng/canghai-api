package com.canghai.api.entity;

import com.baomidou.mybatisplus.annotation.IdType;
import com.baomidou.mybatisplus.annotation.TableField;
import com.baomidou.mybatisplus.annotation.TableId;
import com.baomidou.mybatisplus.annotation.TableName;
import com.baomidou.mybatisplus.extension.handlers.JacksonTypeHandler;
import lombok.Data;

import java.util.List;

/**
 * 保存的接口请求（与客户端 SavedRequest 对齐）
 *
 * <p>注意：{@code autoResultMap = true} 是 MyBatis-Plus 的硬性要求 —— 否则
 * {@code @TableField(typeHandler = JacksonTypeHandler.class)} 只在写入时生效，
 * **查询结果不会走 typeHandler**，导致 params/headers/form_body 回读恒为空
 * （同步下发会把客户端的请求参数/请求头/表单体清空）。
 */
@Data
@TableName(value = "ch_saved_requests", autoResultMap = true)
public class SavedRequest {

    @TableId(type = IdType.INPUT)
    private String id;
    private String projectId;
    private String name;
    private String method;
    private String url;
    @TableField(typeHandler = JacksonTypeHandler.class)
    private List<KV> params;
    @TableField(typeHandler = JacksonTypeHandler.class)
    private List<KV> headers;
    private String bodyType;
    private String body;
    @TableField(typeHandler = JacksonTypeHandler.class)
    private List<KV> formBody;
    private String categoryId;
    private String preScript;
    private String postScript;
    private Integer sortOrder;
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
