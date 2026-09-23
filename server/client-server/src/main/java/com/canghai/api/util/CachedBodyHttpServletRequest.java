package com.canghai.api.util;

import jakarta.servlet.ReadListener;
import jakarta.servlet.ServletInputStream;
import jakarta.servlet.http.HttpServletRequest;
import jakarta.servlet.http.HttpServletRequestWrapper;

import java.io.BufferedReader;
import java.io.ByteArrayInputStream;
import java.io.IOException;
import java.io.InputStreamReader;
import java.nio.charset.StandardCharsets;

/**
 * 请求体可重复读取的包装器。
 *
 * <p>Servlet 输入流默认只能消费一次：{@code AuthAspect} 切面要先读 body 提取 token，
 * 之后 {@code @RequestBody} 再读就会得到空流。此处把首次读到的字节缓存下来，
 * 后续每次 {@link #getInputStream()} / {@link #getReader()} 都返回基于缓存的新流，
 * 从而让切面与参数绑定可以同时工作。
 */
public class CachedBodyHttpServletRequest extends HttpServletRequestWrapper {

    private final byte[] cachedBody;

    public CachedBodyHttpServletRequest(HttpServletRequest request, byte[] cachedBody) {
        super(request);
        this.cachedBody = cachedBody == null ? new byte[0] : cachedBody;
    }

    /** 缓存的原始请求体（UTF-8 解码），供 {@link RequestBodyUtil} 复用 */
    public String getCachedBody() {
        return new String(cachedBody, StandardCharsets.UTF_8);
    }

    @Override
    public ServletInputStream getInputStream() {
        return new CachedBodyInputStream(cachedBody);
    }

    @Override
    public BufferedReader getReader() {
        return new BufferedReader(new InputStreamReader(getInputStream(), StandardCharsets.UTF_8));
    }

    private static class CachedBodyInputStream extends ServletInputStream {

        private final ByteArrayInputStream source;

        CachedBodyInputStream(byte[] body) {
            this.source = new ByteArrayInputStream(body);
        }

        @Override
        public int read() {
            return source.read();
        }

        @Override
        public int read(byte[] b, int off, int len) {
            return source.read(b, off, len);
        }

        @Override
        public int available() {
            return source.available();
        }

        @Override
        public boolean isFinished() {
            return source.available() == 0;
        }

        @Override
        public boolean isReady() {
            return true;
        }

        @Override
        public void setReadListener(ReadListener readListener) {
            throw new UnsupportedOperationException("Cached body does not support async read listener");
        }

        @Override
        public void close() throws IOException {
            source.close();
        }
    }
}
