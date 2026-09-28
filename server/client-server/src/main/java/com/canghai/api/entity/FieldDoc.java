package com.canghai.api.entity;

import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;

/**
 * 接口字段描述（与客户端 FieldDoc / Rust FieldDoc 对齐）：
 * 字段名 / 类型 / 必填 / 描述，用于请求字段表与响应字段表。
 */
@Data
@NoArgsConstructor
@AllArgsConstructor
@Builder
public class FieldDoc {

    /** 字段名（或路径，如 data.list[].id） */
    private String key;
    /** 字段类型：string/number/integer/boolean/object/array/file/null/any */
    private String fieldType;
    /** 是否必填 */
    private Boolean required;
    /** 字段描述 */
    private String description;
}
