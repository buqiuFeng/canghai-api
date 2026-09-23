-- ============================================================
-- V4 性能索引：收敛全表扫描热点（Phase 7.9）
--
-- 背景：现有索引只覆盖单列（如 idx_req_project(project_id)），但业务查询几乎
--       总是「project_id + deleted=0」组合过滤；ch_projects 更是完全没有可用于
--       「按归属/软删过滤」的索引，数据量增长后 EXPLAIN 退化为全表扫描。
--
-- 设计：
--   1. 复合索引列顺序按「等值过滤列在前、范围/排序列在后」排列：
--      (project_id, deleted) 可同时命中 project_id 等值与 deleted 等值；
--      (project_id, create_time) 供审计日志按项目 + 时间排序分页。
--   2. 显式命名 idx_*，与 V1/V2 命名风格一致，便于后续 DROP/排障。
--   3. 只新增、不删除既有索引：单列索引仍可服务仅按 project_id 的查询，
--      且删除索引会带来回滚成本，此处不做清理。
--
-- 注意：MySQL 8 不支持 CREATE INDEX IF NOT EXISTS，本脚本由 Flyway 保证只执行一次；
--       存量库通过 baseline-on-migrate 从 v1 起增量执行，同样只跑一次。
-- ============================================================

-- 项目：按软删 + 归属人过滤（getVisibleProjectIds / listByCreateBy 等）
ALTER TABLE ch_projects
    ADD INDEX idx_proj_deleted_createby (deleted, create_by);

-- 分类：按项目 + 软删过滤（listCategories）
ALTER TABLE ch_categories
    ADD INDEX idx_cat_project_deleted (project_id, deleted);

-- 接口：按项目 + 软删过滤（listRequests，按 sort_order 展示）
ALTER TABLE ch_saved_requests
    ADD INDEX idx_req_project_deleted (project_id, deleted);

-- 环境：按项目 + 软删过滤（listEnvironments）
ALTER TABLE ch_environments
    ADD INDEX idx_env_project_deleted (project_id, deleted);

-- 环境分组：按项目 + 软删过滤（listGroups）
ALTER TABLE ch_environment_groups
    ADD INDEX idx_envg_project_deleted (project_id, deleted);

-- 环境变量：按环境 + 软删过滤（listVariables，同步收集热点）
ALTER TABLE ch_environment_variables
    ADD INDEX idx_envv_env_deleted (environment_id, deleted);

-- 团队成员：按团队 + 软删过滤（getVisibleProjectIds 子查询）
ALTER TABLE ch_team_members
    ADD INDEX idx_tm_team_deleted (team_id, deleted);

-- 审计日志：按项目 + 创建时间倒序分页（/api/v1/audit/query）
ALTER TABLE ch_audit_logs
    ADD INDEX idx_audit_project_time (project_id, create_time);
