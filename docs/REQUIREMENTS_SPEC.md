# 苍海 API 文档系统 — 需求规格说明书 (AI-Readable)

> 本文档由代码逆向生成（server: Java/Undertow, client: Rust/Tauri + Vue3），供 AI 理解与二次开发。
> 编码约定：UTF-8；时间字段统一为字符串（服务端与客户端均用 `now()` 生成可读时间戳）；
> 鉴权令牌置于 JSON 请求体 `token` 字段（非 HTTP Header）。

---

## 0. 系统概览 (System Overview)

- **产品名称**: 苍海 (canghai) — 团队协作式 API 接口文档/调试工具
- **架构**:
  - 服务端: `com.wenhui.canghai` — 基于 aifei 框架 + Undertow 内嵌服务器，Mysql 持久化（`Db`/`Row` 轻量 ORM）。
  - 桌面端: `client/apps/desktop` (Rust + Tauri) — 本地 SQLite 缓存 + 与服务端同步。
  - Web 端: `client/apps/web` (Vue3 + Vite + TypeScript) — 通过 HTTP JSON 调用服务端 API。
- **核心领域对象**: User / Team / TeamMember / Category(Project) / SavedRequest / EnvironmentGroup / Environment / EnvironmentVariable
- **协作模型**: 以 `team`（团队/工作区）为数据隔离边界，成员按角色分级授权，通过 `/api/sync` 实现多端数据同步（last-write-wins）。

---

## 1. 统一接口约定 (API Conventions)

### 1.1 请求/响应格式
- Content-Type: `application/json`
- 请求体 (POST): JSON，鉴权接口需在顶层包含 `token` 字段。
- 响应统一结构 (`Result<T>`):
  ```json
  { "success": true, "msg": "ok", "data": <T> }
  { "success": false, "msg": "错误描述" }   // 无 data 字段
  ```
- HTTP 状态码: 成功 200；业务失败（含 401/403/404/400）统一返回 400，错误详情在 `msg`。
  （注：UndertowDispatcher 在异常时返回 500。）

### 1.2 鉴权 (Auth)
- `token` 由登录/注册时服务端生成（32 字节随机 → Base64URL，无填充），有效期 **7 天**。
- 除 `/api/auth/*` 外，所有接口需在 body 携带 `token`，服务端用 `AuthService.validateToken` 校验
  （过期自动清理并返回 401）。
- 密码哈希: `SHA-256(salt + password)`，存储格式 `salt:hash`（salt 16 字节 Base64）。

### 1.3 路由分发 (ApiHandler)
| 路径前缀 | Service | 鉴权 |
|---|---|---|
| `/api/auth/*` | AuthService | 否 |
| `/api/sync*` | SyncService | 是 |
| `/api/team*` | TeamService | 是 |
| `/api/audit*` | AuditService | 是 |
| `/api/category*` | CategoryService | 是 |
| `/api/request*` | SavedRequestService | 是 |
| `/api/environment*` | EnvironmentService | 是 |

---

## 2. 领域模型 (Data Model)

### 2.1 用户 (User) — 表 `ch_users`
| 字段 | 类型 | 说明 |
|---|---|---|
| id | string(uuid) | PK |
| username | string | 唯一（软删除过滤），登录名 |
| nickname | string | 显示名（默认同 username） |
| email | string | 可空 |
| password_hash | string | `salt:hash` |
| create_time/update_time/create_by/update_by | string | 审计字段 |
| deleted | int(0/1) | 软删除 |

### 2.2 团队 (Team) — 表 `ch_teams`
| 字段 | 类型 | 说明 |
|---|---|---|
| id | string(uuid) | PK |
| name | string | 团队名 |
| description | string | 描述 |
| owner_id | string | 所有者 user id |
| user_id | string | 冗余创建者 |
| deleted | int | 软删除 |
> 注册时自动为用户创建「{nickname}的默认工作区」团队，并加为 owner。

### 2.3 团队成员 (TeamMember) — 表 `ch_team_members`
| 字段 | 类型 | 说明 |
|---|---|---|
| id | string(uuid) | PK |
| team_id | string | FK → ch_teams |
| user_id | string | FK → ch_users |
| role | enum | 见下 |
| deleted | int | 软删除 |

**角色权限分级 (roleLevel)**:
- `owner` (4): 完全控制，含删除团队、移交所有权。
- `admin` (3): 管理成员、编辑团队、删除实体。
- `readwrite` (2): 同步上下行、编辑。
- `readonly` (1): 仅查看与 pull。
- 写入判定 `canWrite` = 角色 ≠ readonly；`isAdminOrAbove` = owner/admin。

### 2.4 分类 (Category，代码中亦称 Project) — 表 `ch_categories`
- **注意**: 服务端表名为 `ch_categories`，但 Rust 端结构体命名为 `Project`，二者为同一实体（同步时字段映射一致）。
| 字段 | 类型 | 说明 |
|---|---|---|
| id | string(uuid) | PK |
| team_id | string | 数据隔离边界 |
| name | string | 分类名 |
| parent_id | string\|null | 树形父节点 |
| sort_order | int | 排序 |
| expanded | bool | 前端展开态 |
| deleted | int | 软删除 |

### 2.5 保存的请求 (SavedRequest) — 表 `ch_saved_requests`
| 字段 | 类型 | 说明 |
|---|---|---|
| id | string(uuid) | PK |
| team_id | string | 隔离边界 |
| name | string | 请求名 |
| method | string | GET/POST/... 默认 GET |
| url | string | 请求 URL |
| params | JSON(KV[]) | 查询参数（存为 KV JSON 字符串） |
| headers | JSON(KV[]) | 请求头 |
| body_type | string | none/form-data/raw 等，默认 none |
| body | string | 请求体 |
| form_body | JSON(KV[]) | 表单数据 |
| pre_script / post_script | string | 前后置脚本 |
| category_id | string\|null | 所属分类 |
| sort_order | int | 排序 |
| deleted | int | 软删除 |

### 2.6 环境分组 (EnvironmentGroup) — 表 `ch_environment_groups`
| 字段 | 类型 | 说明 |
|---|---|---|
| id | string(uuid) | PK |
| team_id | string | 隔离边界 |
| project_id | string\|null | 关联项目（可选） |
| name | string | 分组名 |
| sort_order | int | 排序 |
| expanded | bool | 展开态 |
| deleted | int | 软删除 |

### 2.7 环境 (Environment) — 表 `ch_environments`
| 字段 | 类型 | 说明 |
|---|---|---|
| id | string(uuid) | PK |
| team_id | string | 隔离边界 |
| project_id | string\|null | 关联项目（可选） |
| name | string | 环境名 |
| group_id | string\|null | 所属分组 |
| is_active | bool | 是否激活（同 team+project 下唯一激活） |
| deleted | int | 软删除 |

### 2.8 环境变量 (EnvironmentVariable) — 表 `ch_environment_variables`
| 字段 | 类型 | 说明 |
|---|---|---|
| id | string(uuid) | PK（**全局唯一**，跨团队仅按 environment_id 关联） |
| environment_id | string | FK → ch_environments |
| key | string | 变量名 |
| value | string | 变量值 |
| enabled | bool | 是否启用 |
| sort_order | int | 排序 |
| deleted | int | 软删除 |

### 2.9 审计日志 (AuditLog) — 表 `ch_audit_logs`
| 字段 | 类型 | 说明 |
|---|---|---|
| id | string | PK |
| team_id | string | 隔离边界 |
| user_id / username | string | 操作人 |
| action | string | create/update/delete |
| entity_type | string | request/category/environment/... |
| entity_id / entity_name | string | 目标 |
| before_json / after_json | string\|null | 变更快照 |
| ip | string | 预留（当前空） |
| create_time | string | 时间 |

### 2.10 令牌 (UserToken) — 表 `ch_user_tokens`
| 字段 | 类型 | 说明 |
|---|---|---|
| id | string | PK |
| user_id | string | FK |
| token | string | 随机令牌 |
| expires_at | long(ms) | 过期时间戳 |
> 审计字段省略。

---

## 3. 功能需求 (Functional Requirements)

### 3.1 认证 (AuthService) — `/api/auth`
| action | method | 入参 | 说明 | 返回 |
|---|---|---|---|---|
| register | POST | {username, password(≥6), nickname?, email?} | 注册 + 自动建默认团队 + 自动登录 | LoginResponse |
| login | POST | {username, password} | 校验密码，发令牌 | LoginResponse |
| logout | POST | {token} | 删除令牌 | ok |
| me | POST | {token} | 取当前用户 | User |

`LoginResponse` = { token, user: User, expiresAt: long, teamId: string(首个团队) }

### 3.2 团队管理 (TeamService) — `/api/team`
| action | 说明 | 权限 | 关键校验 |
|---|---|---|---|
| mine | 列出我所属团队 | 任何成员 | — |
| create | 建团队，创建者=owner | 登录 | 名称非空 |
| update | 改团队名/描述 | owner/admin | — |
| delete | 软删团队+成员 | owner | — |
| members | 成员列表 | 任何成员 | — |
| invite | 邀请用户(按 username) | owner/admin | 不可分配高于自己角色；目标须已注册 |
| changeRole | 改成员角色 | owner/admin | 不可改 owner；不可高于自己 |
| removeMember | 移除成员 | owner/admin | 不可移除 owner；admin 不可移除 admin/自己 |
| transfer | 移交所有权 | owner | 目标自动加为 owner，原 owner 降级 admin |

### 3.3 分类 (CategoryService) — `/api/category`
- CRUD: `list`(按 teamId) / `create` / `update` / `delete`(软删)
- 校验用户为团队成员（`TeamService.isMember`）。
- 变更写审计日志。

### 3.4 请求 (SavedRequestService) — `/api/request`
- CRUD: `list` / `create` / `update` / `delete`(软删)
- 字段含 params/headers/form_body 以 KV JSON 存储（`RowMapper.toKVJson`）。
- 校验成员权限；变更写审计日志。

### 3.5 环境 (EnvironmentService) — `/api/environment`
- **分组** `group/{list,create,update,delete}` — 支持按 teamId / projectId 过滤。
- **环境** `{list,create,update,delete,activate}` — `activate` 取消同 team(+project) 下其他激活态后激活目标。
- **变量** `variable/{list,save,delete}` — save 按 id 存在与否决定插入/更新。
- 写操作（create/update/delete 分组与环境）写审计日志。

### 3.6 同步 (SyncService) — `/api/sync`
- `upload` (POST): 客户端上报 `SyncData`，服务端按 **update_time 最新者胜 (last-write-wins)** 合并五类实体（categories/requests/environmentGroups/environments/environmentVariables），软删除标记 `deleted=true` 同步处理。仅 `canWrite` 可上传。返回全量 `SyncResponse`。
  - 若 teamId 不存在则自动建团队并将上传者加为 owner。
- `pull` (GET/POST): 拉取服务端全量（含按 team 过滤的环境变量），任何成员可访问。
- `SyncData` 字段: teamId, categories[], categoryTrees[], requests[], environmentGroups[], environments[], environmentVariables[]
- `SyncResponse` 字段: teamId, 同上五类 + syncAt(ms)
- **合并算法** (`mergeEntities`): 服务端加载 (id→update_time) 映射 → 客户端无记录则插入 / 客户端更新时间更新则覆盖 / 否则保留服务端；最后处理软删除标记。

### 3.7 审计 (AuditService) — `/api/audit/query`
- 按 projectId + 可选 entityType/entityId/keyword 分页查询（page/size，size 20~100）；关系已改为以项目维度隔离（原 teamId 已废弃）。
- 写入失败不阻断主流程（try-catch）。

---

## 4. 非功能需求 (Non-Functional)

- **数据隔离**: 所有业务表以 `team_id` 过滤（环境变量经 environment_id 间接关联）。
- **软删除**: 全表统一 `deleted=1` 标记，查询默认 `deleted=0`。
- **审计**: 实体增删改写 `ch_audit_logs`（environment 类、category、request 已接入）。
- **并发同步**: last-write-wins 策略，无分布式锁，依赖 `update_time` 字符串比较（ISO 可读时间戳需保证字典序=时间序）。
- **本地缓存**: 桌面端 Rust 层用 SQLite 缓存并通过 `merge_server_data` 同策略合回本地。
- **安全**: 密码 SHA-256+salt；令牌随机 32 字节；角色分级鉴权。

---

## 5. 客户端约定 (Client Notes)

### 5.1 桌面端 (Rust/Tauri)
- `sync.rs`: `SyncData`/`SyncResponse` 用 `serde(rename_all="camelCase")`，与服务端字段对齐。
- `collect_all` 聚合本地数据 → `merge_server_data` 用 `update_time` 字符串比较合入服务端数据。
- `db.rs` 提供各实体 `save_*_for_merge` / `update_*_for_merge`。

### 5.2 Web 端 (Vue3)
- `useApi.ts`: 封装 fetch，自动附加 `token`（来自 `useAuth`）。
- `useSync.ts`: 调用 `/api/sync/upload` 与 `pull`，本地状态合并。
- `useTeams`/`useWorkspaces`: 团队与项目（workspace）状态管理。
- 类型集中在 `types/index.ts`。

---

## 6. 待办 / 已知缺口 (Gaps & TODO)

1. **Workspace vs Team 命名混用**: Web 端称 workspace/project，服务端后端称 team；`project_id` 字段已在 environment 实体引入，但 Server 模型中 `Workspace`/`WorkspaceMember` 模型存在却未见对应 Service 路由（需确认是否已接入）。
2. **Category 双重命名**: 服务端 `ch_categories` ↔ Rust `Project` 结构体，文档与代码需统一。
3. **审计覆盖不全**: 同步上传路径（SyncService）未写审计日志，仅 CRUD 接口写。
4. **ip 字段**: `AuditService.log` 写死空串，未取真实 IP。
5. **project_id 过滤**: EnvironmentService 支持 projectId 过滤，但 Category/Request 同步未显式按 project 隔离（仅 team 级）。

---

> 生成依据：已读取 `server/.../service/*`、`handler/*`、`model/*`、`db/DbInitializer.java`、
> `client/apps/desktop/src/sync.rs`、`client/apps/web/src/composables/*` 及 `README.md`。
> 编码：UTF-8。
