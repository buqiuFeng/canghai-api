package com.canghai.api.entity;

import com.baomidou.mybatisplus.annotation.IdType;
import com.baomidou.mybatisplus.annotation.TableId;
import com.baomidou.mybatisplus.annotation.TableName;
import lombok.Data;

/**
 * 用户令牌（兼容表，在线模式主要使用 JWT）
 */
@Data
@TableName("ch_user_tokens")
public class UserToken {

    @TableId(type = IdType.INPUT)
    private String id;
    private String userId;
    private String token;
    private Long expiresAt;
    private String createTime;
    private String createBy;
    private String updateTime;
    private String updateBy;
    private Boolean deleted;
}
