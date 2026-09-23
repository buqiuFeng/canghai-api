package com.canghai.api.config;

import org.springframework.boot.context.properties.ConfigurationProperties;
import org.springframework.stereotype.Component;

import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;

/**
 * 鉴权相关配置。
 * 免登录白名单（Ant 风格路径）可在 application.yml 中通过 canghai.auth.whitelist 覆盖，
 * 默认已包含认证接口与心跳探测。
 */
@Component
@ConfigurationProperties(prefix = "canghai.auth")
public class AuthProperties {

    private List<String> whitelist = new ArrayList<>(Arrays.asList("/api/v1/auth/**", "/api/v1/ping"));

    public List<String> getWhitelist() {
        return whitelist;
    }

    public void setWhitelist(List<String> whitelist) {
        this.whitelist = whitelist;
    }
}
