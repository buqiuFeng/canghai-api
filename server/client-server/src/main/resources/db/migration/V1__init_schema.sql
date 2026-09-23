-- ============================================================
-- 沧海 API 文档服务端（spring-boot-server）— Flyway 基线迁移 V1
--
-- 说明：
--   1. 本文件是 DDL 的**唯一事实源**，取代原先互不一致的 schema.sql 与 db.sql。
--      - 环境变量列统一为 var_key（KEY 为 MySQL 保留字，避免全链路加反引号）
--      - JSON 类列（params/headers/form_body）统一为 LONGTEXT（配合 MyBatis-Plus JacksonTypeHandler）
--      - 术语统一为「团队 / 项目 / 环境」，密码算法统一为 PBKDF2
--   2. Flyway 不负责创建数据库，运行前需先手动建库：
--        CREATE DATABASE IF NOT EXISTS canghai_api
--          DEFAULT CHARACTER SET utf8mb4 DEFAULT COLLATE utf8mb4_unicode_ci;
--   3. 全部语句均为 CREATE TABLE IF NOT EXISTS，可重复执行（幂等）。
--   4. 存量库（已手工建表）请保持 spring.flyway.baseline-on-migrate=true，
--      Flyway 会将其标记为 baseline 而不重复建表。
-- ============================================================

-- ============================================================
-- 用户
-- ============================================================
CREATE TABLE IF NOT EXISTS ch_users (
    id            VARCHAR(36)  NOT NULL PRIMARY KEY COMMENT '用户唯一标识',
    username      VARCHAR(50)  NOT NULL COMMENT '用户名',
    nickname      VARCHAR(50)  DEFAULT '' COMMENT '昵称',
    email         VARCHAR(200) DEFAULT '' COMMENT '邮箱',
    password_hash VARCHAR(200) NOT NULL COMMENT '密码哈希 (PBKDF2-HMAC-SHA256，格式 pbkdf2$iterations$salt$hash)',
    create_time   VARCHAR(19)  NOT NULL COMMENT '创建时间 (yyyy-MM-dd HH:mm:ss)',
    create_by     VARCHAR(36)  DEFAULT NULL COMMENT '创建人',
    update_time   VARCHAR(19)  NOT NULL COMMENT '修改时间 (yyyy-MM-dd HH:mm:ss)',
    update_by     VARCHAR(36)  DEFAULT NULL COMMENT '修改人',
    deleted       TINYINT(1)   NOT NULL DEFAULT 0 COMMENT '软删除标记',
    UNIQUE INDEX idx_user_username (username)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci COMMENT='用户';

-- ============================================================
-- 用户令牌（登录会话）
-- ============================================================
CREATE TABLE IF NOT EXISTS ch_user_tokens (
    id          VARCHAR(36)  NOT NULL PRIMARY KEY COMMENT '令牌唯一标识',
    user_id     VARCHAR(36)  NOT NULL COMMENT '所属用户',
    token       VARCHAR(200) NOT NULL COMMENT '令牌值',
    expires_at  BIGINT       NOT NULL COMMENT '过期时间 (Unix 毫秒)',
    create_time VARCHAR(19)  NOT NULL COMMENT '创建时间 (yyyy-MM-dd HH:mm:ss)',
    create_by   VARCHAR(36)  DEFAULT NULL COMMENT '创建人',
    update_time VARCHAR(19)  NOT NULL COMMENT '修改时间 (yyyy-MM-dd HH:mm:ss)',
    update_by   VARCHAR(36)  DEFAULT NULL COMMENT '修改人',
    deleted     TINYINT(1)   NOT NULL DEFAULT 0 COMMENT '软删除标记',
    INDEX idx_token_user (user_id),
    INDEX idx_token_value (token)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci COMMENT='用户令牌';

-- ============================================================
-- 团队（数据隔离单元）
-- ============================================================
CREATE TABLE IF NOT EXISTS ch_teams (
    id          VARCHAR(36)  NOT NULL PRIMARY KEY COMMENT '团队唯一标识',
    name        VARCHAR(100) NOT NULL COMMENT '团队名称',
    description VARCHAR(500) DEFAULT '' COMMENT '团队描述',
    owner_id    VARCHAR(36)  DEFAULT NULL COMMENT '所有者用户 ID',
    user_id     VARCHAR(36)  DEFAULT '' COMMENT '归属用户 ID（与 owner_id 一致，冗余便于用户级过滤）',
    create_time VARCHAR(19)  NOT NULL COMMENT '创建时间 (yyyy-MM-dd HH:mm:ss)',
    create_by   VARCHAR(36)  DEFAULT NULL COMMENT '创建人',
    update_time VARCHAR(19)  NOT NULL COMMENT '修改时间 (yyyy-MM-dd HH:mm:ss)',
    update_by   VARCHAR(36)  DEFAULT NULL COMMENT '修改人',
    deleted     TINYINT(1)   NOT NULL DEFAULT 0 COMMENT '软删除标记',
    INDEX idx_ws_owner (owner_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci COMMENT='团队';

-- ============================================================
-- 团队成员
-- ============================================================
CREATE TABLE IF NOT EXISTS ch_team_members (
    id          VARCHAR(36)  NOT NULL PRIMARY KEY COMMENT '成员关系唯一标识',
    team_id     VARCHAR(36)  NOT NULL COMMENT '团队 ID',
    user_id     VARCHAR(36)  NOT NULL COMMENT '用户 ID',
    role        VARCHAR(20)  NOT NULL DEFAULT 'readwrite' COMMENT '角色: owner / admin / readwrite / readonly',
    create_time VARCHAR(19)  NOT NULL COMMENT '加入时间 (yyyy-MM-dd HH:mm:ss)',
    create_by   VARCHAR(36)  DEFAULT NULL COMMENT '创建人',
    update_time VARCHAR(19)  NOT NULL COMMENT '修改时间 (yyyy-MM-dd HH:mm:ss)',
    update_by   VARCHAR(36)  DEFAULT NULL COMMENT '修改人',
    deleted     TINYINT(1)   NOT NULL DEFAULT 0 COMMENT '软删除标记',
    UNIQUE INDEX idx_tm_team_user (team_id, user_id),
    INDEX idx_wm_user (user_id),
    INDEX idx_tm_team (team_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci COMMENT='团队成员';

-- ============================================================
-- 项目
-- ============================================================
CREATE TABLE IF NOT EXISTS ch_projects (
    id          VARCHAR(36)  NOT NULL PRIMARY KEY COMMENT '项目唯一标识',
    name        VARCHAR(100) NOT NULL COMMENT '项目名称',
    description VARCHAR(500) DEFAULT '' COMMENT '项目描述',
    create_time VARCHAR(19)  NOT NULL COMMENT '创建时间 (yyyy-MM-dd HH:mm:ss)',
    create_by   VARCHAR(36)  DEFAULT NULL COMMENT '创建人',
    update_time VARCHAR(19)  NOT NULL COMMENT '修改时间 (yyyy-MM-dd HH:mm:ss)',
    update_by   VARCHAR(36)  DEFAULT NULL COMMENT '修改人',
    deleted     TINYINT(1)   NOT NULL DEFAULT 0 COMMENT '软删除标记'
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci COMMENT='项目';

-- ============================================================
-- 项目成员
--   member_type 区分：
--     team - 团队成员（引用 ch_teams.id）
--     user - 用户成员（引用 ch_users.id）
-- ============================================================
CREATE TABLE IF NOT EXISTS ch_project_members (
    id          VARCHAR(36)  NOT NULL PRIMARY KEY COMMENT '成员关系唯一标识',
    project_id  VARCHAR(36)  NOT NULL COMMENT '所属项目 ID',
    member_type VARCHAR(10)  NOT NULL DEFAULT 'user' COMMENT '成员类型: team(团队) / user(用户)',
    member_id   VARCHAR(36)  NOT NULL COMMENT '成员 ID（team 类型对应 ch_teams.id，user 类型对应 ch_users.id）',
    member_name VARCHAR(100) NOT NULL DEFAULT '' COMMENT '成员名称快照（user→ch_users.username，team→ch_teams.name）',
    role        VARCHAR(20)  NOT NULL DEFAULT 'readwrite' COMMENT '角色: owner / admin / readwrite / readonly',
    create_time VARCHAR(19)  NOT NULL COMMENT '加入时间 (yyyy-MM-dd HH:mm:ss)',
    create_by   VARCHAR(36)  DEFAULT NULL COMMENT '创建人',
    update_time VARCHAR(19)  NOT NULL COMMENT '修改时间 (yyyy-MM-dd HH:mm:ss)',
    update_by   VARCHAR(36)  DEFAULT NULL COMMENT '修改人',
    deleted     TINYINT(1)   NOT NULL DEFAULT 0 COMMENT '软删除标记',
    UNIQUE INDEX idx_pm_project_member (project_id, member_type, member_id),
    INDEX idx_pm_project (project_id),
    INDEX idx_pm_member (member_type, member_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci COMMENT='项目成员';

-- ============================================================
-- 分类目录
-- ============================================================
CREATE TABLE IF NOT EXISTS ch_categories (
    id          VARCHAR(36)  NOT NULL PRIMARY KEY COMMENT '分类唯一标识',
    project_id  VARCHAR(36)  DEFAULT NULL COMMENT '所属项目（团队经项目成员表关联）',
    name        VARCHAR(200) NOT NULL COMMENT '分类名称',
    parent_id   VARCHAR(36)  DEFAULT NULL COMMENT '父分类 ID',
    sort_order  INT          NOT NULL DEFAULT 0 COMMENT '排序序号',
    expanded    TINYINT(1)   NOT NULL DEFAULT 1 COMMENT '是否展开',
    create_time VARCHAR(19)  NOT NULL COMMENT '创建时间 (yyyy-MM-dd HH:mm:ss)',
    create_by   VARCHAR(36)  DEFAULT NULL COMMENT '创建人',
    update_time VARCHAR(19)  NOT NULL COMMENT '修改时间 (yyyy-MM-dd HH:mm:ss)',
    update_by   VARCHAR(36)  DEFAULT NULL COMMENT '修改人',
    deleted     TINYINT(1)   NOT NULL DEFAULT 0 COMMENT '软删除标记',
    INDEX idx_cat_project (project_id),
    INDEX idx_parent (parent_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci COMMENT='分类目录';

-- ============================================================
-- 保存的接口请求
-- ============================================================
CREATE TABLE IF NOT EXISTS ch_saved_requests (
    id          VARCHAR(36)   NOT NULL PRIMARY KEY COMMENT '接口唯一标识',
    project_id  VARCHAR(36)   DEFAULT NULL COMMENT '所属项目（团队经项目成员表关联）',
    name        VARCHAR(300)  NOT NULL COMMENT '接口名称',
    method      VARCHAR(10)   NOT NULL DEFAULT 'GET' COMMENT 'HTTP 方法',
    url         VARCHAR(4000) NOT NULL DEFAULT '' COMMENT '请求 URL',
    params      LONGTEXT      COMMENT 'Query 参数 (KV 数组 JSON)',
    headers     LONGTEXT      COMMENT '请求头 (KV 数组 JSON)',
    body_type   VARCHAR(10)   NOT NULL DEFAULT 'none' COMMENT '请求体类型: none/json/form/text',
    body        MEDIUMTEXT    COMMENT '请求体内容',
    form_body   LONGTEXT      COMMENT '表单请求体 (KV 数组 JSON)',
    category_id VARCHAR(36)   DEFAULT NULL COMMENT '所属分类',
    pre_script  MEDIUMTEXT    COMMENT '前置脚本',
    post_script MEDIUMTEXT    COMMENT '后置脚本',
    sort_order  INT           NOT NULL DEFAULT 0 COMMENT '排序序号',
    create_time VARCHAR(19)   NOT NULL COMMENT '创建时间 (yyyy-MM-dd HH:mm:ss)',
    create_by   VARCHAR(36)   DEFAULT NULL COMMENT '创建人',
    update_time VARCHAR(19)   NOT NULL COMMENT '修改时间 (yyyy-MM-dd HH:mm:ss)',
    update_by   VARCHAR(36)   DEFAULT NULL COMMENT '修改人',
    deleted     TINYINT(1)    NOT NULL DEFAULT 0 COMMENT '软删除标记',
    INDEX idx_req_project (project_id),
    INDEX idx_req_category (category_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci COMMENT='保存的接口请求';

-- ============================================================
-- 环境分组
-- ============================================================
CREATE TABLE IF NOT EXISTS ch_environment_groups (
    id          VARCHAR(36)  NOT NULL PRIMARY KEY COMMENT '分组唯一标识',
    project_id  VARCHAR(36)  DEFAULT NULL COMMENT '所属项目（团队经项目成员表关联）',
    name        VARCHAR(200) NOT NULL COMMENT '分组名称',
    sort_order  INT          NOT NULL DEFAULT 0 COMMENT '排序序号',
    expanded    TINYINT(1)   NOT NULL DEFAULT 1 COMMENT '是否展开',
    data_mode   VARCHAR(16)  NOT NULL DEFAULT 'online' COMMENT '数据模式 online/offline',
    create_time VARCHAR(19)  NOT NULL COMMENT '创建时间 (yyyy-MM-dd HH:mm:ss)',
    create_by   VARCHAR(36)  DEFAULT NULL COMMENT '创建人',
    update_time VARCHAR(19)  NOT NULL COMMENT '修改时间 (yyyy-MM-dd HH:mm:ss)',
    update_by   VARCHAR(36)  DEFAULT NULL COMMENT '修改人',
    deleted     TINYINT(1)   NOT NULL DEFAULT 0 COMMENT '软删除标记',
    INDEX idx_envg_project (project_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci COMMENT='环境分组';

-- ============================================================
-- 环境
-- ============================================================
CREATE TABLE IF NOT EXISTS ch_environments (
    id          VARCHAR(36)  NOT NULL PRIMARY KEY COMMENT '环境唯一标识',
    project_id  VARCHAR(36)  DEFAULT NULL COMMENT '所属项目（团队经项目成员表关联）',
    name        VARCHAR(200) NOT NULL COMMENT '环境名称',
    group_id    VARCHAR(36)  DEFAULT NULL COMMENT '所属分组',
    is_active   TINYINT(1)   NOT NULL DEFAULT 0 COMMENT '是否激活',
    sort_order  INT          NOT NULL DEFAULT 0 COMMENT '排序序号',
    data_mode   VARCHAR(16)  NOT NULL DEFAULT 'online' COMMENT '数据模式 online/offline',
    create_time VARCHAR(19)  NOT NULL COMMENT '创建时间 (yyyy-MM-dd HH:mm:ss)',
    create_by   VARCHAR(36)  DEFAULT NULL COMMENT '创建人',
    update_time VARCHAR(19)  NOT NULL COMMENT '修改时间 (yyyy-MM-dd HH:mm:ss)',
    update_by   VARCHAR(36)  DEFAULT NULL COMMENT '修改人',
    deleted     TINYINT(1)   NOT NULL DEFAULT 0 COMMENT '软删除标记',
    INDEX idx_env_project (project_id),
    INDEX idx_env_group (group_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci COMMENT='环境';

-- ============================================================
-- 环境变量
--   说明：列名 var_key 而非 key —— KEY 为 MySQL 保留字，
--         避免在手写 SQL / QueryWrapper 中处处加反引号；
--         Java 实体经 @JsonProperty("key") 对外仍输出 key。
-- ============================================================
CREATE TABLE IF NOT EXISTS ch_environment_variables (
    id             VARCHAR(36)  NOT NULL PRIMARY KEY COMMENT '变量唯一标识',
    environment_id VARCHAR(36)  NOT NULL COMMENT '所属环境',
    var_key        VARCHAR(200) NOT NULL COMMENT '变量名',
    `value`        TEXT         NOT NULL COMMENT '变量值',
    enabled        TINYINT(1)   NOT NULL DEFAULT 1 COMMENT '是否启用',
    sort_order     INT          NOT NULL DEFAULT 0 COMMENT '排序序号',
    data_mode      VARCHAR(16)  NOT NULL DEFAULT 'online' COMMENT '数据模式 online/offline',
    create_time    VARCHAR(19)  NOT NULL COMMENT '创建时间 (yyyy-MM-dd HH:mm:ss)',
    create_by      VARCHAR(36)  DEFAULT NULL COMMENT '创建人',
    update_time    VARCHAR(19)  NOT NULL COMMENT '修改时间 (yyyy-MM-dd HH:mm:ss)',
    update_by      VARCHAR(36)  DEFAULT NULL COMMENT '修改人',
    deleted        TINYINT(1)   NOT NULL DEFAULT 0 COMMENT '软删除标记',
    INDEX idx_envv_env (environment_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci COMMENT='环境变量';

-- ============================================================
-- 操作审计日志
-- ============================================================
CREATE TABLE IF NOT EXISTS ch_audit_logs (
    id          VARCHAR(36)  NOT NULL PRIMARY KEY COMMENT '日志唯一标识',
    project_id  VARCHAR(36)  DEFAULT NULL COMMENT '所属项目 ID',
    user_id     VARCHAR(36)  DEFAULT NULL COMMENT '操作人',
    username    VARCHAR(50)  DEFAULT '' COMMENT '操作人用户名（冗余，便于展示）',
    action      VARCHAR(50)  NOT NULL COMMENT '操作类型: create/update/delete/restore/login/...',
    entity_type VARCHAR(50)  NOT NULL COMMENT '实体类型: request/category/environment/project/member',
    entity_id   VARCHAR(36)  DEFAULT NULL COMMENT '实体 ID',
    entity_name VARCHAR(300) DEFAULT '' COMMENT '实体名称（冗余展示）',
    before_json MEDIUMTEXT   COMMENT '变更前快照（JSON）',
    after_json  MEDIUMTEXT   COMMENT '变更后快照（JSON）',
    ip          VARCHAR(64)  DEFAULT '' COMMENT '来源 IP',
    create_time VARCHAR(19)  NOT NULL COMMENT '操作时间 (yyyy-MM-dd HH:mm:ss)',
    deleted     TINYINT(1)   NOT NULL DEFAULT 0 COMMENT '软删除标记',
    INDEX idx_audit_project (project_id),
    INDEX idx_audit_entity (entity_type, entity_id),
    INDEX idx_audit_time (create_time)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci COMMENT='操作审计日志';
