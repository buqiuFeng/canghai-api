package com.canghai.api.entity;

import com.baomidou.mybatisplus.annotation.IdType;
import com.baomidou.mybatisplus.annotation.TableId;
import com.baomidou.mybatisplus.annotation.TableName;
import lombok.Data;

/**
 * 团队成员
 * 角色体系（权限递减）：owner > admin > readwrite > readonly
 */
@Data
@TableName("ch_team_members")
public class TeamMember {

    public static final String ROLE_OWNER = "owner";
    public static final String ROLE_ADMIN = "admin";
    public static final String ROLE_READWRITE = "readwrite";
    public static final String ROLE_READONLY = "readonly";

    @TableId(type = IdType.INPUT)
    private String id;
    private String teamId;
    private String userId;
    private String role;
    private String createTime;
    private String createBy;
    private String updateTime;
    private String updateBy;
    private Boolean deleted;

    @Data
    public static class InviteRequest {
        private String teamId;
        private String username;
        private String role;
    }

    @Data
    public static class TransferRequest {
        private String teamId;
        private String newOwnerUsername;
    }

    @Data
    public static class ChangeRoleRequest {
        private String teamId;
        private String userId;
        private String role;
    }

    @Data
    public static class RemoveMemberRequest {
        private String teamId;
        private String userId;
    }

    /** 成员信息（含用户名） */
    @Data
    public static class MemberInfo {
        private String id;
        private String userId;
        private String username;
        private String role;
        private String createTime;
    }
}
