-- ============================================================
-- V7 已保存接口「字段描述表」：ch_saved_requests.request_fields / response_fields
--
-- 背景：
--   参考 apipost，为接口增加 请求字段表 / 响应字段表（字段名/类型/必填/描述），
--   随接口一起落库并参与云同步、导入导出与版本历史。
--
-- 策略：
--   新增两列 LONGTEXT（与 params/headers 一致，存 FieldDoc 数组 JSON），
--   允许 NULL：存量行无描述即视为空表，客户端以空数组兜底。
--
-- 幂等：仅当列不存在时执行 ALTER（同 V5/V6 写法），全新建库与存量库均可安全升级。
--       注：不改动 V1 基线，避免变更其 Flyway checksum。
-- ============================================================

SET @db_name := DATABASE();

SET @has_col := (SELECT COUNT(*) FROM information_schema.COLUMNS
                 WHERE TABLE_SCHEMA = @db_name
                   AND TABLE_NAME = 'ch_saved_requests'
                   AND COLUMN_NAME = 'request_fields');
SET @ddl := IF(@has_col = 0,
    'ALTER TABLE ch_saved_requests ADD COLUMN request_fields LONGTEXT COMMENT ''请求字段描述表 (FieldDoc 数组 JSON)'' AFTER form_body',
    'SELECT 1');
PREPARE add_request_fields FROM @ddl;
EXECUTE add_request_fields;
DEALLOCATE PREPARE add_request_fields;

SET @has_col := (SELECT COUNT(*) FROM information_schema.COLUMNS
                 WHERE TABLE_SCHEMA = @db_name
                   AND TABLE_NAME = 'ch_saved_requests'
                   AND COLUMN_NAME = 'response_fields');
SET @ddl := IF(@has_col = 0,
    'ALTER TABLE ch_saved_requests ADD COLUMN response_fields LONGTEXT COMMENT ''响应字段描述表 (FieldDoc 数组 JSON)'' AFTER request_fields',
    'SELECT 1');
PREPARE add_response_fields FROM @ddl;
EXECUTE add_response_fields;
DEALLOCATE PREPARE add_response_fields;
