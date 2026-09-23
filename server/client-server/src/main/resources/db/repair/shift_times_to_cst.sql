-- ============================================================
-- 一次性数据迁移：时间基准由 UTC 平移为 Asia/Shanghai（北京时间，UTC+8）
--
-- 背景：
--   三端时间基准已统一为 Asia/Shanghai —— Java（CanghaiApiApplication 钉死 JVM 时区
--   + 连接串 connectionTimeZone/forceConnectionTimeZoneToSession）、Rust
--   （db::now_timestamp）、前端（utils.now）。切换后**新写入**的数据是北京时间，
--   而历史数据是按 UTC 写入的，两者相差 8 小时。
--
-- 为什么必须迁（以及为什么可以放心迁）：
--   时间戳在同步合并 / 冲突判定中只做**字符串字典序**比较。+8 是统一平移，
--   新老混用不会打乱先后顺序（工程上是安全的），但列表里老数据会显示成 8 小时前，
--   同一行 create_time 与 server_update_time 也会看起来"打架"，故做一次性对齐。
--
-- 需要区分的两类历史数据：
--   1) server_update_time（DATETIME(3)，MySQL 自维护）在旧会话下写的是 UTC → 一律 +8h；
--   2) 业务列 create_time / update_time（VARCHAR(19)）绝大多数是 UTC → +8h；
--      但历史上有少量行是**旧客户端按本地时间（CST）**写入的（比 server_update_time
--      超前约 8 小时），这类行本身就是北京时间，必须**保持不动**，否则会被多加 8 小时。
--      判定：update_time >= server_update_time + INTERVAL 1 HOUR → 视为已是北京时间。
--
-- ⚠️ 实现关键（两次踩坑后的结论，勿改）：
--   * 平移业务列会触发 server_update_time 的 ON UPDATE CURRENT_TIMESTAMP 自动刷新，
--     若在之后再做「server_update_time += 8h」，实际叠加的是「当时的 NOW()」，
--     结果会依赖执行时的**会话时区**、且不可重复；
--   * 因此本脚本先把原始 server_update_time 与「是否已是北京时间」**快照到临时表**，
--     业务列平移（可能引发自动刷新）之后，最后按快照显式写回 `原始值 + 8h`。
--     显式赋值优先级高于 ON UPDATE，故最终结果与执行会话的时区无关。
--
-- 执行前提：建议**停掉后端**再执行，避免快照与写回之间夹入并发写入。
--
-- 使用方式（**不会自动执行**，确认后手工运行）：
--   docker exec -i mysql mysql -uroot -p<密码> canghai_api < shift_times_to_cst.sql
--   或： mysql -h127.0.0.1 -uroot -p canghai_api < shift_times_to_cst.sql
--
-- 备注：本地 SQLite 缓存无需迁移——服务端值 +8h 后一定"更新"，
--       下一次同步会按 update_time 覆盖本地副本；仅本地独有的 ch_history 会
--       保留 8 小时偏差（最多 20 条，属可接受的展示噪音）。
-- ============================================================


-- ============================================================
-- 一、预览（先看这个：确认各表时间列当前值）
-- ============================================================
SELECT 'ch_saved_requests' AS tbl, id, create_time, update_time, server_update_time
  FROM ch_saved_requests ORDER BY server_update_time DESC LIMIT 5;
SELECT 'ch_categories' AS tbl, id, create_time, update_time, server_update_time
  FROM ch_categories ORDER BY server_update_time DESC LIMIT 5;
SELECT 'ch_projects' AS tbl, id, create_time, update_time, NULL AS server_update_time
  FROM ch_projects ORDER BY create_time DESC LIMIT 5;


-- ============================================================
-- 二、快照原始 server_update_time，并标记「已是北京时间」的行
-- ============================================================
DROP TEMPORARY TABLE IF EXISTS tz_sut;
CREATE TEMPORARY TABLE tz_sut (
  tbl    VARCHAR(64) NOT NULL,
  id     VARCHAR(64) NOT NULL,
  sut    DATETIME(3) NULL COMMENT '原始 server_update_time（UTC 基准）',
  is_cst TINYINT     NOT NULL DEFAULT 0 COMMENT '1 = 该行业务时间本就是北京时间，勿再 +8h',
  PRIMARY KEY (tbl, id)
) ENGINE=InnoDB;

INSERT INTO tz_sut (tbl, id, sut, is_cst)
SELECT 'ch_categories', id, server_update_time,
       IFNULL(update_time >= DATE_ADD(server_update_time, INTERVAL 1 HOUR), 0)
  FROM ch_categories;
INSERT INTO tz_sut (tbl, id, sut, is_cst)
SELECT 'ch_saved_requests', id, server_update_time,
       IFNULL(update_time >= DATE_ADD(server_update_time, INTERVAL 1 HOUR), 0)
  FROM ch_saved_requests;
INSERT INTO tz_sut (tbl, id, sut, is_cst)
SELECT 'ch_environment_groups', id, server_update_time,
       IFNULL(update_time >= DATE_ADD(server_update_time, INTERVAL 1 HOUR), 0)
  FROM ch_environment_groups;
INSERT INTO tz_sut (tbl, id, sut, is_cst)
SELECT 'ch_environments', id, server_update_time,
       IFNULL(update_time >= DATE_ADD(server_update_time, INTERVAL 1 HOUR), 0)
  FROM ch_environments;
INSERT INTO tz_sut (tbl, id, sut, is_cst)
SELECT 'ch_environment_variables', id, server_update_time,
       IFNULL(update_time >= DATE_ADD(server_update_time, INTERVAL 1 HOUR), 0)
  FROM ch_environment_variables;

SELECT tbl, COUNT(*) AS rows_total, SUM(is_cst) AS rows_already_cst FROM tz_sut GROUP BY tbl;


-- ============================================================
-- 三、业务列平移 · A 组：无 server_update_time 的表
--     历史上全部由 Java now() 以 UTC 写入，统一 +8h。
-- ============================================================

UPDATE ch_users SET
  create_time = DATE_FORMAT(DATE_ADD(create_time, INTERVAL 8 HOUR), '%Y-%m-%d %H:%i:%s'),
  update_time = DATE_FORMAT(DATE_ADD(update_time, INTERVAL 8 HOUR), '%Y-%m-%d %H:%i:%s')
WHERE create_time REGEXP '^[0-9]{4}-[0-9]{2}-[0-9]{2} [0-9]{2}:[0-9]{2}:[0-9]{2}$'
  AND update_time REGEXP '^[0-9]{4}-[0-9]{2}-[0-9]{2} [0-9]{2}:[0-9]{2}:[0-9]{2}$';

UPDATE ch_user_tokens SET
  create_time = DATE_FORMAT(DATE_ADD(create_time, INTERVAL 8 HOUR), '%Y-%m-%d %H:%i:%s'),
  update_time = DATE_FORMAT(DATE_ADD(update_time, INTERVAL 8 HOUR), '%Y-%m-%d %H:%i:%s')
WHERE create_time REGEXP '^[0-9]{4}-[0-9]{2}-[0-9]{2} [0-9]{2}:[0-9]{2}:[0-9]{2}$'
  AND update_time REGEXP '^[0-9]{4}-[0-9]{2}-[0-9]{2} [0-9]{2}:[0-9]{2}:[0-9]{2}$';

UPDATE ch_teams SET
  create_time = DATE_FORMAT(DATE_ADD(create_time, INTERVAL 8 HOUR), '%Y-%m-%d %H:%i:%s'),
  update_time = DATE_FORMAT(DATE_ADD(update_time, INTERVAL 8 HOUR), '%Y-%m-%d %H:%i:%s')
WHERE create_time REGEXP '^[0-9]{4}-[0-9]{2}-[0-9]{2} [0-9]{2}:[0-9]{2}:[0-9]{2}$'
  AND update_time REGEXP '^[0-9]{4}-[0-9]{2}-[0-9]{2} [0-9]{2}:[0-9]{2}:[0-9]{2}$';

UPDATE ch_team_members SET
  create_time = DATE_FORMAT(DATE_ADD(create_time, INTERVAL 8 HOUR), '%Y-%m-%d %H:%i:%s'),
  update_time = DATE_FORMAT(DATE_ADD(update_time, INTERVAL 8 HOUR), '%Y-%m-%d %H:%i:%s')
WHERE create_time REGEXP '^[0-9]{4}-[0-9]{2}-[0-9]{2} [0-9]{2}:[0-9]{2}:[0-9]{2}$'
  AND update_time REGEXP '^[0-9]{4}-[0-9]{2}-[0-9]{2} [0-9]{2}:[0-9]{2}:[0-9]{2}$';

UPDATE ch_projects SET
  create_time = DATE_FORMAT(DATE_ADD(create_time, INTERVAL 8 HOUR), '%Y-%m-%d %H:%i:%s'),
  update_time = DATE_FORMAT(DATE_ADD(update_time, INTERVAL 8 HOUR), '%Y-%m-%d %H:%i:%s')
WHERE create_time REGEXP '^[0-9]{4}-[0-9]{2}-[0-9]{2} [0-9]{2}:[0-9]{2}:[0-9]{2}$'
  AND update_time REGEXP '^[0-9]{4}-[0-9]{2}-[0-9]{2} [0-9]{2}:[0-9]{2}:[0-9]{2}$';

UPDATE ch_project_members SET
  create_time = DATE_FORMAT(DATE_ADD(create_time, INTERVAL 8 HOUR), '%Y-%m-%d %H:%i:%s'),
  update_time = DATE_FORMAT(DATE_ADD(update_time, INTERVAL 8 HOUR), '%Y-%m-%d %H:%i:%s')
WHERE create_time REGEXP '^[0-9]{4}-[0-9]{2}-[0-9]{2} [0-9]{2}:[0-9]{2}:[0-9]{2}$'
  AND update_time REGEXP '^[0-9]{4}-[0-9]{2}-[0-9]{2} [0-9]{2}:[0-9]{2}:[0-9]{2}$';

UPDATE ch_audit_logs SET
  create_time = DATE_FORMAT(DATE_ADD(create_time, INTERVAL 8 HOUR), '%Y-%m-%d %H:%i:%s')
WHERE create_time REGEXP '^[0-9]{4}-[0-9]{2}-[0-9]{2} [0-9]{2}:[0-9]{2}:[0-9]{2}$';


-- ============================================================
-- 四、业务列平移 · B 组：有 server_update_time 的表
--     仅处理 is_cst = 0（UTC 基准）的行；is_cst = 1 的行本就是北京时间，保持不动。
-- ============================================================

UPDATE ch_categories r
  JOIN tz_sut s ON s.tbl = 'ch_categories' AND s.id = r.id AND s.is_cst = 0
   SET r.create_time = DATE_FORMAT(DATE_ADD(r.create_time, INTERVAL 8 HOUR), '%Y-%m-%d %H:%i:%s'),
       r.update_time = DATE_FORMAT(DATE_ADD(r.update_time, INTERVAL 8 HOUR), '%Y-%m-%d %H:%i:%s')
 WHERE r.create_time REGEXP '^[0-9]{4}-[0-9]{2}-[0-9]{2} [0-9]{2}:[0-9]{2}:[0-9]{2}$'
   AND r.update_time REGEXP '^[0-9]{4}-[0-9]{2}-[0-9]{2} [0-9]{2}:[0-9]{2}:[0-9]{2}$';

UPDATE ch_saved_requests r
  JOIN tz_sut s ON s.tbl = 'ch_saved_requests' AND s.id = r.id AND s.is_cst = 0
   SET r.create_time = DATE_FORMAT(DATE_ADD(r.create_time, INTERVAL 8 HOUR), '%Y-%m-%d %H:%i:%s'),
       r.update_time = DATE_FORMAT(DATE_ADD(r.update_time, INTERVAL 8 HOUR), '%Y-%m-%d %H:%i:%s')
 WHERE r.create_time REGEXP '^[0-9]{4}-[0-9]{2}-[0-9]{2} [0-9]{2}:[0-9]{2}:[0-9]{2}$'
   AND r.update_time REGEXP '^[0-9]{4}-[0-9]{2}-[0-9]{2} [0-9]{2}:[0-9]{2}:[0-9]{2}$';

UPDATE ch_environment_groups r
  JOIN tz_sut s ON s.tbl = 'ch_environment_groups' AND s.id = r.id AND s.is_cst = 0
   SET r.create_time = DATE_FORMAT(DATE_ADD(r.create_time, INTERVAL 8 HOUR), '%Y-%m-%d %H:%i:%s'),
       r.update_time = DATE_FORMAT(DATE_ADD(r.update_time, INTERVAL 8 HOUR), '%Y-%m-%d %H:%i:%s')
 WHERE r.create_time REGEXP '^[0-9]{4}-[0-9]{2}-[0-9]{2} [0-9]{2}:[0-9]{2}:[0-9]{2}$'
   AND r.update_time REGEXP '^[0-9]{4}-[0-9]{2}-[0-9]{2} [0-9]{2}:[0-9]{2}:[0-9]{2}$';

UPDATE ch_environments r
  JOIN tz_sut s ON s.tbl = 'ch_environments' AND s.id = r.id AND s.is_cst = 0
   SET r.create_time = DATE_FORMAT(DATE_ADD(r.create_time, INTERVAL 8 HOUR), '%Y-%m-%d %H:%i:%s'),
       r.update_time = DATE_FORMAT(DATE_ADD(r.update_time, INTERVAL 8 HOUR), '%Y-%m-%d %H:%i:%s')
 WHERE r.create_time REGEXP '^[0-9]{4}-[0-9]{2}-[0-9]{2} [0-9]{2}:[0-9]{2}:[0-9]{2}$'
   AND r.update_time REGEXP '^[0-9]{4}-[0-9]{2}-[0-9]{2} [0-9]{2}:[0-9]{2}:[0-9]{2}$';

UPDATE ch_environment_variables r
  JOIN tz_sut s ON s.tbl = 'ch_environment_variables' AND s.id = r.id AND s.is_cst = 0
   SET r.create_time = DATE_FORMAT(DATE_ADD(r.create_time, INTERVAL 8 HOUR), '%Y-%m-%d %H:%i:%s'),
       r.update_time = DATE_FORMAT(DATE_ADD(r.update_time, INTERVAL 8 HOUR), '%Y-%m-%d %H:%i:%s')
 WHERE r.create_time REGEXP '^[0-9]{4}-[0-9]{2}-[0-9]{2} [0-9]{2}:[0-9]{2}:[0-9]{2}$'
   AND r.update_time REGEXP '^[0-9]{4}-[0-9]{2}-[0-9]{2} [0-9]{2}:[0-9]{2}:[0-9]{2}$';


-- ============================================================
-- 五、server_update_time 写回：一律 = 快照原值 + 8h
--     显式赋值优先级高于 ON UPDATE CURRENT_TIMESTAMP，因此不受第四步触发的自动刷新影响，
--     也与执行会话的时区无关（纯值运算）。
--     写回后所有行的时间都变大 → 客户端下次同步会重新拉取，本地缓存随之对齐北京时间。
-- ============================================================

UPDATE ch_categories r
  JOIN tz_sut s ON s.tbl = 'ch_categories' AND s.id = r.id AND s.sut IS NOT NULL
   SET r.server_update_time = DATE_ADD(s.sut, INTERVAL 8 HOUR);

UPDATE ch_saved_requests r
  JOIN tz_sut s ON s.tbl = 'ch_saved_requests' AND s.id = r.id AND s.sut IS NOT NULL
   SET r.server_update_time = DATE_ADD(s.sut, INTERVAL 8 HOUR);

UPDATE ch_environment_groups r
  JOIN tz_sut s ON s.tbl = 'ch_environment_groups' AND s.id = r.id AND s.sut IS NOT NULL
   SET r.server_update_time = DATE_ADD(s.sut, INTERVAL 8 HOUR);

UPDATE ch_environments r
  JOIN tz_sut s ON s.tbl = 'ch_environments' AND s.id = r.id AND s.sut IS NOT NULL
   SET r.server_update_time = DATE_ADD(s.sut, INTERVAL 8 HOUR);

UPDATE ch_environment_variables r
  JOIN tz_sut s ON s.tbl = 'ch_environment_variables' AND s.id = r.id AND s.sut IS NOT NULL
   SET r.server_update_time = DATE_ADD(s.sut, INTERVAL 8 HOUR);


-- ============================================================
-- 六、复核：同一行的 create_time / update_time 应与 server_update_time 同一基准
--     （业务时间不应再比 server_update_time 超前一个时区，也不应落后 8 小时以上）
-- ============================================================
SELECT 'ch_saved_requests' AS tbl, id, create_time, update_time, server_update_time
  FROM ch_saved_requests ORDER BY server_update_time DESC LIMIT 5;
SELECT 'ch_categories' AS tbl, id, create_time, update_time, server_update_time
  FROM ch_categories ORDER BY server_update_time DESC LIMIT 5;

DROP TEMPORARY TABLE IF EXISTS tz_sut;
