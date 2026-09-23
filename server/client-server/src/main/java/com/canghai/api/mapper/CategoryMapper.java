package com.canghai.api.mapper;

import com.baomidou.mybatisplus.core.mapper.BaseMapper;
import com.canghai.api.entity.Category;
import org.apache.ibatis.annotations.Insert;
import org.apache.ibatis.annotations.Mapper;
import org.apache.ibatis.annotations.Param;

import java.util.List;

@Mapper
public interface CategoryMapper extends BaseMapper<Category> {

    /**
     * 批量 upsert（Phase 7.3）：一次 round-trip 写入多行，取代同步合并中的逐行 insert/update。
     *
     * <p>冲突时只更新业务列，**不覆盖 create_time / create_by**；`sync_version` 由
     * BEFORE UPDATE 触发器自增、`server_update_time` 由列定义自动刷新，与逐行写路径一致。
     */
    @Insert("<script>" +
            "INSERT INTO ch_categories (id, project_id, name, parent_id, sort_order, expanded, create_time, create_by, update_time, update_by, deleted) VALUES " +
            "<foreach collection='list' item='e' separator=','>" +
            "(#{e.id}, #{e.projectId}, #{e.name}, #{e.parentId}, #{e.sortOrder}, #{e.expanded}, #{e.createTime}, #{e.createBy}, #{e.updateTime}, #{e.updateBy}, #{e.deleted})" +
            "</foreach> " +
            "ON DUPLICATE KEY UPDATE project_id=VALUES(project_id), name=VALUES(name), parent_id=VALUES(parent_id), " +
            "sort_order=VALUES(sort_order), expanded=VALUES(expanded), update_time=VALUES(update_time), " +
            "update_by=VALUES(update_by), deleted=VALUES(deleted)" +
            "</script>")
    int batchUpsert(@Param("list") List<Category> list);
}
