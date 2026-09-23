package com.canghai.api.entity;

import com.baomidou.mybatisplus.annotation.IdType;
import com.baomidou.mybatisplus.annotation.TableId;
import com.baomidou.mybatisplus.annotation.TableName;
import lombok.Data;

/**
 * 团队
 */
@Data
@TableName("ch_teams")
public class Team {

    @TableId(type = IdType.INPUT)
    private String id;
    private String name;
    private String description;
    private String ownerId;
    private String userId;
    private String createTime;
    private String createBy;
    private String updateTime;
    private String updateBy;
    private Boolean deleted;
}
