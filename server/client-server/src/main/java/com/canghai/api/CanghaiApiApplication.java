package com.canghai.api;

import org.springframework.boot.SpringApplication;
import org.springframework.boot.autoconfigure.SpringBootApplication;

import java.util.TimeZone;

/**
 * 沧海 API 文档服务端（Spring Boot + MyBatis-Plus 实现）
 */
@SpringBootApplication
public class CanghaiApiApplication {

    public static void main(String[] args) {
        // ==================== 时区基准：全链路 Asia/Shanghai（UTC+8）====================
        // 必须在 Spring 容器启动之前设置，才能覆盖 JDBC 驱动初始化、Jackson、
        // 以及所有 LocalDateTime.now() 调用点，避免出现「同一行数据两个时钟」。
        //
        // 契约（三端一致，基准为北京时间 UTC+8）：
        //   - MySQL 连接会话时区由连接串的 connectionTimeZone + forceConnectionTimeZoneToSession
        //     钉在 Asia/Shanghai，因此 server_update_time（DATETIME(3)，MySQL 自维护）
        //     与同步游标 NOW(3) 同源；
        //   - 业务时间列 create_time / update_time 是 VARCHAR(19)，由 BaseService.now()
        //     以北京时间字面量写入；
        //   - Rust 端（db::now_timestamp）与前端（utils.now）同样输出北京时间。
        // 中国不使用夏令时、UTC+8 是固定偏移，故不存在 DST 跳变风险；
        // 若三端基准不一致，同步游标与冲突判定会整体偏移 8 小时（表现为「差 8 小时」）。
        TimeZone.setDefault(TimeZone.getTimeZone("Asia/Shanghai"));

        SpringApplication.run(CanghaiApiApplication.class, args);
        System.out.println("========================================");
        System.out.println("  沧海 API 文档服务端 (spring-boot) 已启动");
        System.out.println("  默认端口: 8092");
        System.out.println("========================================");
    }
}
