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
import com.fasterxml.jackson.annotation.JsonInclude;
import lombok.Data;

import java.util.List;

/**
 * 同步响应体（服务端下发）
 *
 * <p>统一 {@code @JsonInclude(NON_NULL)}：未赋值的字段（如「用户无可见项目」时提前返回、
 * 结构/内容列表均未 set 的场景）直接<b>不序列化</b>，而不是下发 {@code null}。
 * 客户端（Rust serde）对「字段缺失」有 {@code #[serde(default)]} 兜底，但对显式
 * {@code null} 会报 {@code invalid type: null}，从而整包解析失败。
 */
@Data
@JsonInclude(JsonInclude.Include.NON_NULL)
public class SyncResponse {
    private Long syncAt;
    /**
     * 本次响应的服务端时间游标（yyyy-MM-dd HH:mm:ss.SSS）。
     * 客户端应持久化该值，并在下次 sync/upload、sync/pull 时作为 lastSyncTime 回传，
     * 服务端据此只下发 server_update_time >= lastSyncTime 的增量数据。
     */
    private String serverTime;
    /** 本次是否为增量下发（true=仅返回变更数据；false=全量） */
    private Boolean incremental;
    /**
     * 增量区间内被服务端删除的实体 id（墓碑）。仅 incremental=true 时有值，
     * 客户端应对这些 id 做本地删除，否则增量同步无法感知删除。
     */
    private List<String> deletedIds;
    private List<Project> projects;
    private List<Category> categories;
    private List<SavedRequest> requests;
    private List<EnvironmentGroup> environmentGroups;
    private List<Environment> environments;
    private List<ProjectMember> projectMembers;
    private List<Team> teams;
    private List<TeamMember> teamMembers;
    private List<EnvironmentVariable> environmentVariables;
}
