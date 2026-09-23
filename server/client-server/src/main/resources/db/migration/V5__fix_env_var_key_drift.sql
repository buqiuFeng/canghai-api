-- ============================================================
-- V5 修复环境变量列名漂移：`key` → `var_key`
--
-- 背景（Phase 6.2 遗留的真实 DDL 漂移）：
--   Phase 3 之前源码存在两份 DDL —— schema.sql 用 `var_key`、db.sql 用 `` `key` ``。
--   合并为 V1 时统一定为 `var_key`，但**存量库**首次启动走的是
--   Flyway `baseline-on-migrate`（baseline 到 v1，不重建表），
--   因此由旧 db.sql 建库的环境依然保留 `key` 列。
--
--   结果：实体 @TableField("var_key")、EnvironmentService 手写 SQL、同步合并语句
--   全部报 `Unknown column 'var_key' in 'field list'` —— 环境变量读写/同步整体不可用。
--
-- 策略：
--   仅当「旧列 `key` 存在」且「新列 `var_key` 不存在」时执行重命名；
--   否则执行 `SELECT 1` 空操作。幂等、可重复执行，不破坏已正确的库。
-- ============================================================

SET @db_name := DATABASE();

SET @has_old := (SELECT COUNT(*) FROM information_schema.COLUMNS
                 WHERE TABLE_SCHEMA = @db_name
                   AND TABLE_NAME = 'ch_environment_variables'
                   AND COLUMN_NAME = 'key');

SET @has_new := (SELECT COUNT(*) FROM information_schema.COLUMNS
                 WHERE TABLE_SCHEMA = @db_name
                   AND TABLE_NAME = 'ch_environment_variables'
                   AND COLUMN_NAME = 'var_key');

SET @ddl := IF(@has_old > 0 AND @has_new = 0,
    'ALTER TABLE ch_environment_variables CHANGE COLUMN `key` var_key VARCHAR(200) NOT NULL COMMENT ''变量名''',
    'SELECT 1');

PREPARE fix_var_key FROM @ddl;
EXECUTE fix_var_key;
DEALLOCATE PREPARE fix_var_key;
