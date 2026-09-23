package com.canghai.api.mapper;

import com.baomidou.mybatisplus.core.mapper.BaseMapper;
import com.canghai.api.entity.Environment;
import org.apache.ibatis.annotations.Insert;
import org.apache.ibatis.annotations.Mapper;
import org.apache.ibatis.annotations.Param;

import java.util.List;

@Mapper
public interface EnvironmentMapper extends BaseMapper<Environment> {

    /**
     * 批量 upsert（Phase 7.3）：一次 round-trip 写入多行，取代同步合并中的逐行 insert/update。
     * 冲突时只更新业务列，不覆盖 create_time/create_by。
     */
    @Insert("<script>" +
            "INSERT INTO ch_environments (id, project_id, name, group_id, is_active, sort_order, create_time, create_by, update_time, update_by, deleted) VALUES " +
            "<foreach collection='list' item='e' separator=','>" +
            "(#{e.id}, #{e.projectId}, #{e.name}, #{e.groupId}, #{e.isActive}, #{e.sortOrder}, #{e.createTime}, #{e.createBy}, #{e.updateTime}, #{e.updateBy}, #{e.deleted})" +
            "</foreach> " +
            "ON DUPLICATE KEY UPDATE project_id=VALUES(project_id), name=VALUES(name), group_id=VALUES(group_id), " +
            "is_active=VALUES(is_active), sort_order=VALUES(sort_order), update_time=VALUES(update_time), " +
            "update_by=VALUES(update_by), deleted=VALUES(deleted)" +
            "</script>")
    int batchUpsert(@Param("list") List<Environment> list);
}
