package com.canghai.api.mapper;

import com.baomidou.mybatisplus.core.mapper.BaseMapper;
import com.canghai.api.entity.EnvironmentVariable;
import org.apache.ibatis.annotations.Insert;
import org.apache.ibatis.annotations.Mapper;
import org.apache.ibatis.annotations.Param;
import org.apache.ibatis.annotations.Select;

import java.util.List;

@Mapper
public interface EnvironmentVariableMapper extends BaseMapper<EnvironmentVariable> {

    @Select("SELECT v.* FROM ch_environment_variables v " +
            "JOIN ch_environments e ON v.environment_id = e.id " +
            "WHERE e.project_id = #{projectId} AND e.is_active = 1 AND e.deleted = 0 " +
            "AND v.deleted = 0 AND v.enabled = 1 AND v.data_mode = #{dataMode} " +
            "ORDER BY v.sort_order ASC")
    List<EnvironmentVariable> selectActiveVariables(@Param("projectId") String projectId,
                                                   @Param("dataMode") String dataMode);

    /**
     * 批量 upsert（Phase 7.3）：一次 round-trip 写入多行，取代同步合并中的逐行 insert/update。
     *
     * <p>列名 `var_key`（避开 MySQL 保留字）与 `value` 需反引号；冲突时只更新业务列，
     * 不覆盖 create_time/create_by。
     */
    @Insert("<script>" +
            "INSERT INTO ch_environment_variables (id, environment_id, var_key, `value`, enabled, sort_order, create_time, create_by, update_time, update_by, deleted) VALUES " +
            "<foreach collection='list' item='e' separator=','>" +
            "(#{e.id}, #{e.environmentId}, #{e.varKey}, #{e.value}, #{e.enabled}, #{e.sortOrder}, #{e.createTime}, #{e.createBy}, #{e.updateTime}, #{e.updateBy}, #{e.deleted})" +
            "</foreach> " +
            "ON DUPLICATE KEY UPDATE environment_id=VALUES(environment_id), var_key=VALUES(var_key), " +
            "`value`=VALUES(`value`), enabled=VALUES(enabled), sort_order=VALUES(sort_order), " +
            "update_time=VALUES(update_time), update_by=VALUES(update_by), deleted=VALUES(deleted)" +
            "</script>")
    int batchUpsert(@Param("list") List<EnvironmentVariable> list);
}
