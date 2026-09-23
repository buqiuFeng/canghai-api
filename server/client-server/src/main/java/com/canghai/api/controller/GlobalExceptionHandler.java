package com.canghai.api.controller;

import com.canghai.api.common.ApiException;
import com.canghai.api.common.ApiResult;
import com.canghai.api.common.ErrorCode;
import jakarta.validation.ConstraintViolationException;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.validation.FieldError;
import org.springframework.web.bind.MethodArgumentNotValidException;
import org.springframework.web.bind.annotation.ExceptionHandler;
import org.springframework.web.bind.annotation.RestControllerAdvice;
import org.springframework.web.method.annotation.HandlerMethodValidationException;
import org.springframework.web.multipart.MaxUploadSizeExceededException;

/**
 * 全局异常处理器 —— 三端唯一异常收敛点。
 *
 * <p>约定：控制器/AOP/Service 不再各自 try-catch，异常统一冒泡到此处，
 * 转为统一响应信封 {@code ApiResult{success,code,msg,data}}：
 * <ul>
 *   <li>{@link ApiException}：业务异常，透传其 code/msg；</li>
 *   <li>参数校验异常：归一到 {@link ErrorCode#PARAM_INVALID}，回吐首个字段级提示；</li>
 *   <li>其它异常：记录完整堆栈到日志，仅向客户端返回通用文案（不泄露 SQL/栈信息）。</li>
 * </ul>
 */
@RestControllerAdvice
public class GlobalExceptionHandler {

    private static final Logger log = LoggerFactory.getLogger(GlobalExceptionHandler.class);

    @ExceptionHandler(ApiException.class)
    public ApiResult<?> handleApiException(ApiException e) {
        return ApiResult.fail(e.getCode(), e.getMessage());
    }

    /** {@code @Valid @RequestBody} 校验失败：取首个字段错误作为提示。 */
    @ExceptionHandler(MethodArgumentNotValidException.class)
    public ApiResult<?> handleMethodArgumentNotValid(MethodArgumentNotValidException e) {
        FieldError first = e.getBindingResult().getFieldError();
        String detail = first != null ? first.getDefaultMessage() : "请求体校验失败";
        return ApiResult.fail(ErrorCode.PARAM_INVALID, detail);
    }

    /**
     * 方法级校验失败（Spring 6.1+ {@code @Validated} 控制器上的参数校验）。
     * 仅在 {@code @Validated} 场景触发，此处归一到参数无效。
     */
    @ExceptionHandler(HandlerMethodValidationException.class)
    public ApiResult<?> handleHandlerMethodValidation(HandlerMethodValidationException e) {
        log.warn("参数校验失败: {}", e.getMessage());
        return ApiResult.fail(ErrorCode.PARAM_INVALID, "请求参数校验失败");
    }

    /** Bean Validation 约束违反（如手动 {@code Validator} 调用）。 */
    @ExceptionHandler(ConstraintViolationException.class)
    public ApiResult<?> handleConstraintViolation(ConstraintViolationException e) {
        String detail = e.getConstraintViolations().stream()
                .findFirst()
                .map(v -> v.getMessage())
                .orElse("请求参数校验失败");
        return ApiResult.fail(ErrorCode.PARAM_INVALID, detail);
    }

    /**
     * 上传文件超过 {@code spring.servlet.multipart.max-file-size}（默认 16MB）。
     *
     * <p>该异常在 multipart 解析阶段就抛出，早于控制器逻辑，若不单独处理会落到
     * 「未处理异常」分支，给用户返回「服务器内部错误」——实际只是文件过大。
     */
    @ExceptionHandler(MaxUploadSizeExceededException.class)
    public ApiResult<?> handleMaxUploadSize(MaxUploadSizeExceededException e) {
        log.warn("上传文件超限: {}", e.getMessage());
        return ApiResult.fail(ErrorCode.PARAM_INVALID, "文件过大，单文件上限 16MB");
    }

    @ExceptionHandler(Exception.class)
    public ApiResult<?> handleException(Exception e) {
        // 明细写日志，不回吐 e.getMessage()，避免泄露内部实现细节
        log.error("未处理异常", e);
        return ApiResult.fail(ErrorCode.INTERNAL_ERROR);
    }
}
