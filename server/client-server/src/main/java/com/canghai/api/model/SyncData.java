package com.canghai.api.model;

import com.canghai.api.entity.Category;
import com.canghai.api.entity.Environment;
import com.canghai.api.entity.EnvironmentGroup;
import com.canghai.api.entity.EnvironmentVariable;
import com.canghai.api.entity.Project;
import com.canghai.api.entity.ProjectMember;
import com.canghai.api.entity.SavedRequest;
import com.canghai.api.entity.Team;
import com.canghai.api.entity.TeamMember;
import com.fasterxml.jackson.annotation.JsonAlias;
import lombok.Data;

import java.util.List;

/**
 * 同步请求体（客户端上报）
 */
@Data
public class SyncData {
    /**
     * 客户端上次同步成功时服务端返回的游标（服务端时间，格式 yyyy-MM-dd HH:mm:ss.SSS）。
     * 为空/空串表示全量拉取（首次同步或客户端未记录游标）。
     */
    private String lastSyncTime;
    private List<Project> projects;
    private List<Team> teams;
    private List<TeamMember> teamMembers;
    private List<Category> categories;
    /** 客户端上报的键名为 requests，同时兼容 savedRequests 写法 */
    @JsonAlias("requests")
    private List<SavedRequest> savedRequests;
    private List<Environment> environments;
    private List<EnvironmentGroup> environmentGroups;
    private List<EnvironmentVariable> environmentVariables;
    private List<ProjectMember> projectMembers;
}
