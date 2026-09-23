package com.canghai.api.entity;

import com.baomidou.mybatisplus.annotation.IdType;
import com.baomidou.mybatisplus.annotation.TableId;
import com.baomidou.mybatisplus.annotation.TableName;
import lombok.Data;

/**
 * 项目（隶属团队，是环境/分类等的二级实体）
 */
@Data
@TableName("ch_projects")
public class Project {

    @TableId(type = IdType.INPUT)
    private String id;
    private String name;
    private String description;
    private String createTime;
    private String createBy;
    private String updateTime;
    private String updateBy;
    private Boolean deleted;
}
