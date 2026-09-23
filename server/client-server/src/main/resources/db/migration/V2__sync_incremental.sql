-- ============================================================
-- V2 增量同步：server_update_time 游标 + version 版本号
--
-- 背景：Phase 4 增量同步。此前 /api/v1/sync/pull 每次返回全量数据，
--       客户端也是全量上传，数据量随项目增长线性膨胀。
--
-- 设计：
--   1. server_update_time：服务端行变更时间，**由 MySQL 自动维护**
--      （插入取 CURRENT_TIMESTAMP(3)，更新自动刷新），因此业务 CRUD、
--      同步合并不需要改动一行 Java 代码即可覆盖全部写路径。
--      → 增量拉取条件：server_update_time >= lastSyncTime
--        （用 >= 而非 >：DATETIME(3) 有毫秒精度，宁可重复下发也不可漏；
--          客户端合并是幂等的按 id upsert，重复无害）
--   2. sync_version：服务端版本号，由 BEFORE UPDATE 触发器自增。
--      命名说明：不叫 version 是因为 ch_saved_requests 的历史"请求快照版本"已占用该名，
--      两者语义不同（快照版本是本地概念，sync_version 是服务端概念）。
--      用途：客户端上报时携带自己上次看到的 version，服务端在
--      update_time 相同（秒级精度撞车）时以 version 大者为准做 tie-break；
--      前端也可用 version 变化判断"服务端已被他人修改"。
--   3. 只对参与同步的 5 张业务表加列；项目/团队/成员数据量小且变更少，
--      仍全量下发。
-- ============================================================

-- ---------- 1. 新增列与索引 ----------
ALTER TABLE ch_categories
    ADD COLUMN server_update_time DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3) COMMENT '服务端行变更时间（增量同步游标，DB 自动维护）',
    ADD COLUMN sync_version INT NOT NULL DEFAULT 1 COMMENT '服务端版本号（sync_version），每次 UPDATE 自增，用于同秒并发的冲突判定',
    ADD INDEX idx_cat_sut (server_update_time);

ALTER TABLE ch_saved_requests
    ADD COLUMN server_update_time DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3) COMMENT '服务端行变更时间（增量同步游标，DB 自动维护）',
    ADD COLUMN sync_version INT NOT NULL DEFAULT 1 COMMENT '服务端版本号（sync_version），每次 UPDATE 自增，用于同秒并发的冲突判定',
    ADD INDEX idx_req_sut (server_update_time);

ALTER TABLE ch_environment_groups
    ADD COLUMN server_update_time DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3) COMMENT '服务端行变更时间（增量同步游标，DB 自动维护）',
    ADD COLUMN sync_version INT NOT NULL DEFAULT 1 COMMENT '服务端版本号（sync_version），每次 UPDATE 自增，用于同秒并发的冲突判定',
    ADD INDEX idx_envg_sut (server_update_time);

ALTER TABLE ch_environments
    ADD COLUMN server_update_time DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3) COMMENT '服务端行变更时间（增量同步游标，DB 自动维护）',
    ADD COLUMN sync_version INT NOT NULL DEFAULT 1 COMMENT '服务端版本号（sync_version），每次 UPDATE 自增，用于同秒并发的冲突判定',
    ADD INDEX idx_env_sut (server_update_time);

ALTER TABLE ch_environment_variables
    ADD COLUMN server_update_time DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3) COMMENT '服务端行变更时间（增量同步游标，DB 自动维护）',
    ADD COLUMN sync_version INT NOT NULL DEFAULT 1 COMMENT '服务端版本号（sync_version），每次 UPDATE 自增，用于同秒并发的冲突判定',
    ADD INDEX idx_envv_sut (server_update_time);

-- ---------- 2. 存量数据回填 ----------
-- 说明：UPDATE 语句中显式给 server_update_time 赋值时，MySQL 以显式值为准，
--       不会触发 ON UPDATE CURRENT_TIMESTAMP，因此可安全回填历史时间。
--       update_time 非法或为空时回退为当前时间（等价于"待客户端下次全量对齐"）。
UPDATE ch_categories
    SET server_update_time = COALESCE(STR_TO_DATE(update_time, '%Y-%m-%d %H:%i:%s'), NOW(3));
UPDATE ch_saved_requests
    SET server_update_time = COALESCE(STR_TO_DATE(update_time, '%Y-%m-%d %H:%i:%s'), NOW(3));
UPDATE ch_environment_groups
    SET server_update_time = COALESCE(STR_TO_DATE(update_time, '%Y-%m-%d %H:%i:%s'), NOW(3));
UPDATE ch_environments
    SET server_update_time = COALESCE(STR_TO_DATE(update_time, '%Y-%m-%d %H:%i:%s'), NOW(3));
UPDATE ch_environment_variables
    SET server_update_time = COALESCE(STR_TO_DATE(update_time, '%Y-%m-%d %H:%i:%s'), NOW(3));

-- ---------- 3. version 自增触发器（回填后再建，避免存量行被平白 +1） ----------
CREATE TRIGGER trg_ch_categories_bu BEFORE UPDATE ON ch_categories FOR EACH ROW SET NEW.sync_version = OLD.sync_version + 1;
CREATE TRIGGER trg_ch_saved_requests_bu BEFORE UPDATE ON ch_saved_requests FOR EACH ROW SET NEW.sync_version = OLD.sync_version + 1;
CREATE TRIGGER trg_ch_environment_groups_bu BEFORE UPDATE ON ch_environment_groups FOR EACH ROW SET NEW.sync_version = OLD.sync_version + 1;
CREATE TRIGGER trg_ch_environments_bu BEFORE UPDATE ON ch_environments FOR EACH ROW SET NEW.sync_version = OLD.sync_version + 1;
CREATE TRIGGER trg_ch_environment_variables_bu BEFORE UPDATE ON ch_environment_variables FOR EACH ROW SET NEW.sync_version = OLD.sync_version + 1;
