package com.canghai.api.mapper;

import com.baomidou.mybatisplus.core.mapper.BaseMapper;
import com.canghai.api.entity.EnvironmentGroup;
import org.apache.ibatis.annotations.Insert;
import org.apache.ibatis.annotations.Mapper;
import org.apache.ibatis.annotations.Param;

import java.util.List;

@Mapper
public interface EnvironmentGroupMapper extends BaseMapper<EnvironmentGroup> {

    /**
     * 批量 upsert（Phase 7.3）：一次 round-trip 写入多行，取代同步合并中的逐行 insert/update。
     * 冲突时只更新业务列，不覆盖 create_time/create_by（`data_mode` 走列默认值，与逐行写路径一致）。
     */
    @Insert("<script>" +
            "INSERT INTO ch_environment_groups (id, project_id, name, sort_order, expanded, create_time, create_by, update_time, update_by, deleted) VALUES " +
            "<foreach collection='list' item='e' separator=','>" +
            "(#{e.id}, #{e.projectId}, #{e.name}, #{e.sortOrder}, #{e.expanded}, #{e.createTime}, #{e.createBy}, #{e.updateTime}, #{e.updateBy}, #{e.deleted})" +
            "</foreach> " +
            "ON DUPLICATE KEY UPDATE project_id=VALUES(project_id), name=VALUES(name), sort_order=VALUES(sort_order), " +
            "expanded=VALUES(expanded), update_time=VALUES(update_time), update_by=VALUES(update_by), deleted=VALUES(deleted)" +
            "</script>")
    int batchUpsert(@Param("list") List<EnvironmentGroup> list);
}
