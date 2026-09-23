package com.canghai.api.entity;

import com.baomidou.mybatisplus.annotation.IdType;
import com.baomidou.mybatisplus.annotation.TableId;
import com.baomidou.mybatisplus.annotation.TableName;
import com.fasterxml.jackson.annotation.JsonIgnore;
import lombok.Data;

import java.util.List;

/**
 * 用户
 */
@Data
@TableName("ch_users")
public class User {

    @TableId(type = IdType.INPUT)
    private String id;
    private String username;
    private String nickname;
    private String email;
    @JsonIgnore
    private String passwordHash;
    private String createTime;
    private String createBy;
    private String updateTime;
    private String updateBy;
    private Boolean deleted;

}
