package com.canghai.api.model;

import lombok.Data;

/**
 * 文件导入结果（回给客户端做提示）。
 *
 * <p>字段与前端 {@code importRequests} 的返回值语义对齐：
 * {@code imported} 接口数、{@code skipped} 跳过数、{@code envImported} 环境数、
 * {@code envVarImported} 环境变量数。
 */
@Data
public class ImportResult {

    /** 识别出的来源格式：openapi / postman / curl / apipost / canghai */
    private String source;

    /** 成功导入的接口数 */
    private int imported;

    /**
     * 跳过的接口数。
     *
     * <p>服务端导入是「单事务 + 批量 upsert」：任一环节失败会整体回滚并作为错误返回，
     * 不存在「部分成功部分跳过」，因此该值恒为 0。保留字段是为了与客户端既有提示
     * 逻辑（{@code 成功 X，跳过 Y}）兼容。
     */
    private int skipped;

    /** 新建的分类数 */
    private int categories;

    /** 新建的环境数 */
    private int envImported;

    /** 新建的环境变量数 */
    private int envVarImported;
}
