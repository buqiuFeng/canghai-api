-- ============================================================
-- V6 项目成员表记录「人员名称」：ch_project_members.member_name
--
-- 背景：
--   成员名称此前只在查询时用 `CASE member_type WHEN 'team' THEN t.name ELSE u.username END`
--   JOIN ch_users / ch_teams 派生，表本身不落库。由此带来两个问题：
--     1) 结构类同步（SyncResponse.projectMembers 下发实体原文）没有名称字段，
--        客户端本地 ch_project_members 只能存空名称，离线时成员列表退回显示 member_id（UUID）；
--     2) 被引用行（用户/团队）物理删除后名称彻底取不到。
--
-- 策略：
--   新增 member_name 列保存「写入时刻的名称快照」（user → ch_users.username，
--   team → ch_teams.name），并回填存量数据；
--   查询以本列为准、取不到时回退 JOIN（见 ProjectMemberMapper.selectMembersWithName），
--   因此两种路径都不会出现空名称。
--
-- 幂等：新库由 V1 建表时已含该列，此处仅当列不存在时执行 ALTER（同 V5 的写法）。
-- ============================================================

SET @db_name := DATABASE();

SET @has_col := (SELECT COUNT(*) FROM information_schema.COLUMNS
                 WHERE TABLE_SCHEMA = @db_name
                   AND TABLE_NAME = 'ch_project_members'
                   AND COLUMN_NAME = 'member_name');

SET @ddl := IF(@has_col = 0,
    'ALTER TABLE ch_project_members ADD COLUMN member_name VARCHAR(100) NOT NULL DEFAULT '''' COMMENT ''成员名称快照（user→ch_users.username，team→ch_teams.name）'' AFTER member_id',
    'SELECT 1');

PREPARE add_member_name FROM @ddl;
EXECUTE add_member_name;
DEALLOCATE PREPARE add_member_name;

-- 存量回填：只填空名称行，可重复执行
UPDATE ch_project_members m
    JOIN ch_users u ON m.member_type = 'user' AND m.member_id = u.id
SET m.member_name = COALESCE(u.username, '')
WHERE m.member_name = '';

UPDATE ch_project_members m
    JOIN ch_teams t ON m.member_type = 'team' AND m.member_id = t.id
SET m.member_name = COALESCE(t.name, '')
WHERE m.member_name = '';
