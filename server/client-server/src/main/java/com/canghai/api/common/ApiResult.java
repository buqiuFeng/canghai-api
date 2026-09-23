package com.canghai.api.common;

import lombok.Data;

import java.io.Serializable;

/**
 * 统一响应包装 {success, code, msg, data}
 */
@Data
public class ApiResult<T> implements Serializable {

    private boolean success;
    private int code;
    private String msg;
    private T data;

    private ApiResult(boolean success, int code, String msg, T data) {
        this.success = success;
        this.code = code;
        this.msg = msg;
        this.data = data;
    }

    public static <T> ApiResult<T> ok(T data) {
        return new ApiResult<>(true, 0, "", data);
    }

    public static <T> ApiResult<T> ok() {
        return new ApiResult<>(true, 0, "", null);
    }

    public static <T> ApiResult<T> fail(String msg) {
        return new ApiResult<>(false, -1, msg, null);
    }

    public static <T> ApiResult<T> fail(int code, String msg) {
        return new ApiResult<>(false, code, msg, null);
    }

    public static <T> ApiResult<T> fail(ErrorCode errorCode) {
        return new ApiResult<>(false, errorCode.getCode(), errorCode.getMessage(), null);
    }

    /**
     * 失败响应（带业务明细文案）。
     *
     * <p>code 取统一错误码，msg 直接用业务文案覆盖（不拼标准文案前缀），
     * 保证向用户展示的是「团队名称不能为空」这类具体提示，而非「参数无效：团队名称不能为空」。
     */
    public static <T> ApiResult<T> fail(ErrorCode errorCode, String detail) {
        String msg = (detail == null || detail.isBlank()) ? errorCode.getMessage() : detail;
        return new ApiResult<>(false, errorCode.getCode(), msg, null);
    }
}
