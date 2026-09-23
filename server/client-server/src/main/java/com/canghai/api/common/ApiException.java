package com.canghai.api.common;

/**
 * 业务异常：携带错误码（401 未登录 / 403 越权 / 404 未知 / 409 冲突 / 400 参数）
 */
public class ApiException extends RuntimeException {

    private final int code;

    public ApiException(int code, String message) {
        super(message);
        this.code = code;
    }

    public int getCode() {
        return code;
    }
}
