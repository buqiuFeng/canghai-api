package com.canghai.api.dto.req;

/**
 * 携带数据模式（data_mode）的请求体标记。
 *
 * <p>环境相关接口按 online / offline 双份存储数据，凡需要区分模式的请求体实现该接口，
 * 由服务层统一从 {@code getMode()} 解析，避免逐个类型判断。
 */
public interface DataModeAware {

    /** 数据模式：online / offline */
    String getMode();
}
