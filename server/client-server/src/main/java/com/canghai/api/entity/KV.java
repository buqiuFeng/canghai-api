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

    public boolean isEnabled() {
        return enabled == null || enabled;
    }
}
