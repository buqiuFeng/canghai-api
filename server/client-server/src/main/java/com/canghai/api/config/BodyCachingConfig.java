package com.canghai.api.config;

import com.canghai.api.util.CachedBodyHttpServletRequest;
import jakarta.servlet.FilterChain;
import jakarta.servlet.ServletException;
import jakarta.servlet.http.HttpServletRequest;
import jakarta.servlet.http.HttpServletResponse;
import lombok.extern.slf4j.Slf4j;
import org.springframework.boot.web.servlet.FilterRegistrationBean;
import org.springframework.context.annotation.Bean;
import org.springframework.context.annotation.Configuration;
import org.springframework.util.StreamUtils;
import org.springframework.web.filter.OncePerRequestFilter;

import java.io.IOException;

/**
 * 请求体缓存过滤器 — 让 body 可以被多次读取。
 *
 * <p>职责：在进入 DispatcherServlet 之前把请求体一次性读入内存并包装 request，
 * 使 {@code AuthAspect} 切面与控制器 {@code @RequestBody} 参数绑定互不干扰。
 * 作用于 {@code /api/**}（客户端）与 {@code /admin/**}（管理后台）的 JSON 请求，
 * GET / multipart 请求直接放行。
 */
@Slf4j
@Configuration
public class BodyCachingConfig {

    /** 请求体缓存上限 16MB，与 spring.servlet.multipart.max-file-size 保持一致 */
    private static final int MAX_BODY_BYTES = 16 * 1024 * 1024;

    @Bean
    public FilterRegistrationBean<BodyCachingFilter> bodyCachingFilter() {
        FilterRegistrationBean<BodyCachingFilter> bean = new FilterRegistrationBean<>();
        bean.setFilter(new BodyCachingFilter());
        // /api/* 与 /admin/* 都是前缀匹配（覆盖子路径）：
        // 管理后台的 token 同样放在 body 中，必须一并缓存，否则 AuthAspect 取不到 token。
        bean.addUrlPatterns("/api/*", "/admin/*");
        bean.setOrder(FilterRegistrationBean.LOWEST_PRECEDENCE - 100);
        bean.setName("bodyCachingFilter");
        return bean;
    }

    /** 请求体包装过滤器（由 {@link #bodyCachingFilter()} 注册到 /api/* 与 /admin/*） */
    static class BodyCachingFilter extends OncePerRequestFilter {

        /** 属性键：与 {@link com.canghai.api.util.RequestBodyUtil#readBody} 共用同一份缓存 */
        static final String BODY_ATTR = "__body";

        @Override
        protected void doFilterInternal(HttpServletRequest request, HttpServletResponse response, FilterChain chain)
                throws ServletException, IOException {
            if (shouldSkip(request)) {
                chain.doFilter(request, response);
                return;
            }
            byte[] body = StreamUtils.copyToByteArray(request.getInputStream());
            if (body.length > MAX_BODY_BYTES) {
                body = new byte[0];
            }
            CachedBodyHttpServletRequest wrapped = new CachedBodyHttpServletRequest(request, body);
            // 与 RequestBodyUtil 共用缓存：切面读 body 时直接命中 attribute，不再消费流
            request.setAttribute(BODY_ATTR, wrapped.getCachedBody());
            chain.doFilter(wrapped, response);
        }

        /** 无请求体或 multipart 请求不包装（multipart 由 Spring 自行解析，避免破坏文件上传） */
        private boolean shouldSkip(HttpServletRequest request) {
            String method = request.getMethod();
            if ("GET".equals(method) || "HEAD".equals(method) || "OPTIONS".equals(method)) {
                return true;
            }
            String contentType = request.getContentType();
            return contentType != null && contentType.toLowerCase().startsWith("multipart/");
        }
    }
}
