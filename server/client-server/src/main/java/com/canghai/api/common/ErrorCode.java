package com.canghai.api.common;

/**
 * 统一业务错误码。与 docs/API_CONTRACT.md 中的错误码表保持一致。
 * 约定：0=成功；4xxxx=客户端/业务错误；5xxxx=服务端错误。
 */
public enum ErrorCode {

    SUCCESS(0, "成功"),

    // 4xxxx 客户端 / 业务错误
    PARAM_INVALID(40000, "参数无效"),
    PARAM_MISSING(40001, "缺少必填参数"),
    BAD_CREDENTIALS(40101, "用户名或密码错误"),
    UNAUTHORIZED(40100, "未登录或登录已过期"),
    TOKEN_EXPIRED(40102, "登录已过期，请重新登录"),
    FORBIDDEN(40103, "无访问权限"),
    NOT_FOUND(40400, "请求的资源不存在"),
    CONFLICT(40900, "资源冲突（可能已存在）"),
    BUSINESS_ERROR(42200, "业务处理失败"),

    // 5xxxx 服务端错误
    INTERNAL_ERROR(50000, "服务器内部错误"),
    DB_ERROR(50001, "数据库操作失败"),
    DOWNSTREAM_ERROR(50002, "下游服务异常");

    private final int code;
    private final String message;

    ErrorCode(int code, String message) {
        this.code = code;
        this.message = message;
    }

    public int getCode() {
        return code;
    }

    public String getMessage() {
        return message;
    }
}
