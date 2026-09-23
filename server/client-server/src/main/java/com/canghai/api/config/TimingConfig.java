package com.canghai.api.config;

import jakarta.servlet.FilterChain;
import jakarta.servlet.ServletException;
import jakarta.servlet.http.HttpServletRequest;
import jakarta.servlet.http.HttpServletResponse;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.boot.web.servlet.FilterRegistrationBean;
import org.springframework.context.annotation.Bean;
import org.springframework.context.annotation.Configuration;
import org.springframework.web.filter.OncePerRequestFilter;

import java.io.IOException;

/**
 * 请求耗时埋点 — Phase 6「可观测」基础设施（无埋点则性能收益不可证伪）。
 *
 * <p>对 {@code /api/**} 每个请求记录 {@code uri / method / status / cost(ms)}，
 * 慢请求（≥ {@link #SLOW_MS}）以 WARN 输出，便于按日志统计 P50/P95 基线。
 * 仅记录元信息，不含请求体，避免日志泄露敏感数据。
 *
 * <p>顺序排在 {@link BodyCachingConfig} 之后（body 缓存 order = LOWEST-100），
 * 保证计时覆盖参数绑定与业务处理全过程。
 */
@Configuration
public class TimingConfig {

    /** 慢请求阈值（ms）：达到即 WARN，便于基线采集时快速捞取尾部延迟 */
    private static final long SLOW_MS = 500;

    @Bean
    public FilterRegistrationBean<RequestTimingFilter> requestTimingFilter() {
        FilterRegistrationBean<RequestTimingFilter> bean = new FilterRegistrationBean<>();
        bean.setFilter(new RequestTimingFilter());
        bean.addUrlPatterns("/api/*");
        bean.setOrder(FilterRegistrationBean.LOWEST_PRECEDENCE - 50);
        bean.setName("requestTimingFilter");
        return bean;
    }

    /** 请求耗时记录过滤器（由 {@link #requestTimingFilter()} 注册到 /api/*） */
    static class RequestTimingFilter extends OncePerRequestFilter {

        private static final Logger timing = LoggerFactory.getLogger("REQ_TIMING");

        @Override
        protected void doFilterInternal(HttpServletRequest request, HttpServletResponse response, FilterChain chain)
                throws ServletException, IOException {
            long start = System.nanoTime();
            try {
                chain.doFilter(request, response);
            } finally {
                long costMs = (System.nanoTime() - start) / 1_000_000;
                String uri = request.getRequestURI();
                String method = request.getMethod();
                int status = response.getStatus();
                if (costMs >= SLOW_MS) {
                    timing.warn("method={} uri={} status={} cost={}ms", method, uri, status, costMs);
                } else {
                    timing.info("method={} uri={} status={} cost={}ms", method, uri, status, costMs);
                }
            }
        }
    }
}
