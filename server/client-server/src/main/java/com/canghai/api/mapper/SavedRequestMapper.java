package com.canghai.api.mapper;

import com.baomidou.mybatisplus.core.mapper.BaseMapper;
import com.canghai.api.entity.SavedRequest;
import org.apache.ibatis.annotations.Insert;
import org.apache.ibatis.annotations.Mapper;
import org.apache.ibatis.annotations.Param;

import java.util.List;

@Mapper
public interface SavedRequestMapper extends BaseMapper<SavedRequest> {

    /**
     * 批量 upsert（Phase 7.3）：一次 round-trip 写入多行，取代同步合并中的逐行 insert/update。
     *
     * <p>`params`/`headers`/`form_body` 为 JSON 列，显式指定 {@code JacksonTypeHandler}，
     * 与实体上 {@code @TableField(typeHandler=...)} 的逐行写路径保持一致。
     * 冲突时只更新业务列，不覆盖 create_time/create_by。
     */
    @Insert("<script>" +
            "INSERT INTO ch_saved_requests (id, project_id, name, method, url, params, headers, body_type, body, form_body, " +
            "category_id, pre_script, post_script, sort_order, create_time, create_by, update_time, update_by, deleted) VALUES " +
            "<foreach collection='list' item='e' separator=','>" +
            "(#{e.id}, #{e.projectId}, #{e.name}, #{e.method}, #{e.url}, " +
            "#{e.params,typeHandler=com.baomidou.mybatisplus.extension.handlers.JacksonTypeHandler}, " +
            "#{e.headers,typeHandler=com.baomidou.mybatisplus.extension.handlers.JacksonTypeHandler}, " +
            "#{e.bodyType}, #{e.body}, " +
            "#{e.formBody,typeHandler=com.baomidou.mybatisplus.extension.handlers.JacksonTypeHandler}, " +
            "#{e.categoryId}, #{e.preScript}, #{e.postScript}, #{e.sortOrder}, #{e.createTime}, #{e.createBy}, " +
            "#{e.updateTime}, #{e.updateBy}, #{e.deleted})" +
            "</foreach> " +
            "ON DUPLICATE KEY UPDATE project_id=VALUES(project_id), name=VALUES(name), method=VALUES(method), url=VALUES(url), " +
            "params=VALUES(params), headers=VALUES(headers), body_type=VALUES(body_type), body=VALUES(body), " +
            "form_body=VALUES(form_body), category_id=VALUES(category_id), pre_script=VALUES(pre_script), " +
            "post_script=VALUES(post_script), sort_order=VALUES(sort_order), update_time=VALUES(update_time), " +
            "update_by=VALUES(update_by), deleted=VALUES(deleted)" +
            "</script>")
    int batchUpsert(@Param("list") List<SavedRequest> list);
}
