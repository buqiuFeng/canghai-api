package com.canghai.api.entity;

import com.baomidou.mybatisplus.annotation.IdType;
import com.baomidou.mybatisplus.annotation.TableId;
import com.baomidou.mybatisplus.annotation.TableName;
import lombok.Data;

/**
 * 项目成员（member_type 区分 team / user）
 */
@Data
@TableName("ch_project_members")
public class ProjectMember {

    public static final String TYPE_TEAM = "team";
    public static final String TYPE_USER = "user";

    public static final String ROLE_OWNER = "owner";
    public static final String ROLE_ADMIN = "admin";
    public static final String ROLE_READWRITE = "readwrite";
    public static final String ROLE_READONLY = "readonly";
    public static final String ROLE_INHERIT = "inherit";

    @TableId(type = IdType.INPUT)
    private String id;
    private String projectId;
    private String memberType;
    private String memberId;
    /**
     * 成员名称快照：user → ch_users.username，team → ch_teams.name。
     * 写入时落库，使结构类同步（SyncResponse.projectMembers）能携带名称，
     * 客户端离线展示无需再回查用户/团队表。
     */
    private String memberName;
    private String role;
    private String createTime;
    private String createBy;
    private String updateTime;
    private String updateBy;
    private Boolean deleted;

    @Data
    public static class MemberInfo {
        private String id;
        private String projectId;
        private String memberType;
        private String memberId;
        private String memberName;
        private String role;
        private String createTime;
    }
}
