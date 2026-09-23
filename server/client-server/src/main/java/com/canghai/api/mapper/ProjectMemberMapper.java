package com.canghai.api.mapper;

import com.baomidou.mybatisplus.core.mapper.BaseMapper;
import com.canghai.api.entity.ProjectMember;
import org.apache.ibatis.annotations.Mapper;
import org.apache.ibatis.annotations.Param;
import org.apache.ibatis.annotations.Select;

import java.util.List;
import java.util.Map;

@Mapper
public interface ProjectMemberMapper extends BaseMapper<ProjectMember> {

    /**
     * 成员列表（含名称）。
     * 名称以落库快照 {@code member_name} 为准；存量行（快照为空）回退 JOIN 派生，
     * 保证新旧数据都不会返回空名称。
     */
    @Select("SELECT m.id, m.project_id AS projectId, m.member_type AS memberType, m.member_id AS memberId, " +
            "COALESCE(NULLIF(m.member_name, ''), CASE m.member_type WHEN 'team' THEN t.name ELSE u.username END) AS memberName, " +
            "m.role AS role, m.create_time AS createTime " +
            "FROM ch_project_members m " +
            "LEFT JOIN ch_teams t ON m.member_type = 'team' AND m.member_id = t.id " +
            "LEFT JOIN ch_users u ON m.member_type = 'user' AND m.member_id = u.id " +
            "WHERE m.project_id = #{projectId} AND m.deleted = 0 ORDER BY m.member_type, m.create_time")
    List<Map<String, Object>> selectMembersWithName(@Param("projectId") String projectId);

    @Select("SELECT p.*, m.role AS memberRole FROM ch_projects p " +
            "JOIN ch_project_members m ON p.id = m.project_id AND m.member_type = 'team' " +
            "AND m.member_id = #{teamId} AND m.deleted = 0 " +
            "WHERE p.deleted = 0 ORDER BY p.create_time DESC")
    List<com.canghai.api.entity.Project> selectByTeam(@Param("teamId") String teamId);
}
