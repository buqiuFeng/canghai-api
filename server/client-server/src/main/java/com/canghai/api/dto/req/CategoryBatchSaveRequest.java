package com.canghai.api.dto.req;

import com.canghai.api.entity.Category;
import lombok.Data;

import java.util.ArrayList;
import java.util.List;

/**
 * 分类批量保存请求体（POST /api/v1/category/batch-save，同步场景）。
 * 语义为全量同步：按 id upsert，并软删除该 project 下未上报的旧记录。
 */
@Data
public class CategoryBatchSaveRequest {

    /** 项目 ID（必填） */
    private String projectId;

    /** 兼容旧客户端的工作区字段，缺失时回退到 projectId */
    private String workspaceId;

    /** 批量分类；缺省时视为空列表 */
    private List<Category> items = new ArrayList<>();
}
