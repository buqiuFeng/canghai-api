package com.canghai.api.mapper;

import com.baomidou.mybatisplus.core.mapper.BaseMapper;
import com.canghai.api.entity.TeamMember;
import org.apache.ibatis.annotations.Mapper;
import org.apache.ibatis.annotations.Param;
import org.apache.ibatis.annotations.Select;

import java.util.List;
import java.util.Map;

@Mapper
public interface TeamMemberMapper extends BaseMapper<TeamMember> {

    @Select("SELECT m.id, m.user_id AS userId, u.username AS username, m.role AS role, m.create_time AS createTime " +
            "FROM ch_team_members m LEFT JOIN ch_users u ON m.user_id = u.id " +
            "WHERE m.team_id = #{teamId} AND m.deleted = 0")
    List<Map<String, Object>> selectMembersWithUsername(@Param("teamId") String teamId);
}
