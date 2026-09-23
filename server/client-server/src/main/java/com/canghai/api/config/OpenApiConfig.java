package com.canghai.api.config;

import io.swagger.v3.oas.annotations.OpenAPIDefinition;
import io.swagger.v3.oas.annotations.enums.SecuritySchemeIn;
import io.swagger.v3.oas.annotations.enums.SecuritySchemeType;
import io.swagger.v3.oas.annotations.info.Info;
import io.swagger.v3.oas.annotations.security.SecurityRequirement;
import io.swagger.v3.oas.annotations.security.SecurityScheme;
import org.springframework.context.annotation.Configuration;

/**
 * OpenAPI 3 文档配置（springdoc-openapi）。
 *
 * <p>文档地址：
 * <ul>
 *   <li>Swagger UI：{@code /swagger-ui.html}</li>
 *   <li>OpenAPI JSON：{@code /v3/api-docs}</li>
 * </ul>
 *
 * <p>说明：
 * <ul>
 *   <li>仅扫描 {@code /api/**}（见 application.yaml 的 springdoc.paths-to-match），运维端点不进文档。</li>
 *   <li>除免登录白名单接口外，其余接口需携带 JWT；{@link AuthAspect} 依次接受
 *       {@code Authorization: Bearer xxx}、{@code X-Token}、body/query 的 {@code token} 字段，
 *       此处统一以 HTTP Bearer 声明。</li>
 *   <li>以上路径已在 {@code canghai.auth.whitelist} 放行，避免被鉴权切面拦截。</li>
 * </ul>
 */
@Configuration
@OpenAPIDefinition(
        info = @Info(
                title = "沧海 API 调试工具 · 服务端接口",
                version = "v1",
                description = "统一响应信封 ApiResult{success, code, msg, data}；"
                        + "除认证与心跳接口外，其余接口需携带登录返回的 JWT。"
        ),
        security = @SecurityRequirement(name = OpenApiConfig.BEARER_AUTH)
)
@SecurityScheme(
        name = OpenApiConfig.BEARER_AUTH,
        type = SecuritySchemeType.HTTP,
        scheme = "bearer",
        bearerFormat = "JWT",
        in = SecuritySchemeIn.HEADER,
        description = "登录返回的 JWT（服务端同时接受 X-Token / body.token / query.token）"
)
public class OpenApiConfig {

    /** 全局安全方案名称 */
    public static final String BEARER_AUTH = "bearerAuth";

}
