package com.canghai.api.entity;

import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;

/**
 * 通用键值对（与客户端 KV 类型对齐）
 */
@Data
@NoArgsConstructor
@AllArgsConstructor
@Builder
public class KV {

    private String key;
    private String value;
    private Boolean enabled;
    /** 字段描述（仅用于说明用途，不参与实际请求）。缺失该字段会导致在线模式保存/同步时描述被丢弃。 */
    private String description;

    public boolean isEnabled() {
        return enabled == null || enabled;
    }
}
