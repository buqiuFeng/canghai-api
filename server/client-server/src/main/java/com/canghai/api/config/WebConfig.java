package com.canghai.api.config;

import org.springframework.context.annotation.Configuration;
import org.springframework.web.servlet.config.annotation.CorsRegistration;
import org.springframework.web.servlet.config.annotation.CorsRegistry;
import org.springframework.web.servlet.config.annotation.WebMvcConfigurer;

/**
 * 全局跨域配置（开发环境放行所有来源，生产环境请按需收紧）。
 *
 * <p>客户端接口（{@code /api/**}）与管理后台接口（{@code /admin/**}）分别注册：
 * 两者前缀独立（见 {@code AdminController} 类注释），CORS 规则也各自显式声明，
 * 避免后续调整客户端跨域策略时误伤或误放开管理端。
 */
@Configuration
public class WebConfig implements WebMvcConfigurer {

    @Override
    public void addCorsMappings(CorsRegistry registry) {
        cors(registry.addMapping("/api/**"));
        cors(registry.addMapping("/admin/**"));
    }

    /** 统一的 CORS 规则（客户端与管理端共用；生产环境请按来源收紧）。 */
    private void cors(CorsRegistration registration) {
        registration.allowedOriginPatterns("*")
                .allowedMethods("GET", "POST", "PUT", "DELETE", "OPTIONS", "HEAD")
                .allowedHeaders("*")
                .exposedHeaders("Authorization", "X-Token")
                .allowCredentials(true)
                .maxAge(3600);
    }
}
