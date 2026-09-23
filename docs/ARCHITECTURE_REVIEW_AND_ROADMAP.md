# 沧海·API 调试工具 — 三端架构评审与分阶段优化路线

> 评审对象：Vue 3 前端（`client/apps/web`）· Rust/Tauri 桌面端（`client/apps/desktop`）· Spring Boot 服务端（`client-server`）
> 评审日期：2026-09-16
> 前置文档：`docs/ARCHITECTURE_OPTIMIZATION.md`（已记录 Phase 0–4 的落地结果）
> 本文定位：在 Phase 0–4 基础上做**整体架构再评估**，并给出 **Phase 5–8** 的可落地、可度量优化路线。
> 证据约定：文中 `文件:行号` 均来自本次代码探查，可直接跳转核对。

---

## 0. 执行摘要

### 0.1 总体判断

系统是一个**「桌面优先 + 云端协作」的三端一体化**产品：Vue 负责 UI 与编排，Rust 作为本地执行与集成枢纽（HTTP 代理 / SQLite 离线库 / 云端桥），Spring Boot 作为数据与权限权威。这个分层方向是**正确且合理**的——它同时满足了"无 CORS 限制"和"团队协作"两个看似冲突的诉求。

但从架构师视角看，当前处于**"功能已跑通、工程化未收敛"**的阶段：Phase 0–4 解决了集成阻塞（JWT 算法、DDL 漂移、增量同步、脚本沙箱），遗留的是**正确性债务、契约漂移与性能未度量**三类问题。其中存在**1 个可直接导致越权的并发缺陷**和**7 个缺失权限校验的读接口**，优先级高于任何性能优化。

### 0.2 六维评分

| 维度 | 评分（5 分制） | 判断依据 |
|---|---|---|
| 架构合理性 | 3.5 | 三端职责边界清晰（Rust=执行枢纽、Java=权威），但分层有越界：Rust 命令内做 JWT/业务推导，Java Service 用实例字段持有用户 |
| 模块耦合度 | 2.5 | `db.rs` 1877 行承担 5 类职责；前端 `useSync↔useTeams`、`useSync↔useApi` 循环依赖；repository 反向依赖 composable |
| 通信协议一致性 | 3.0 | 信封 `ApiResult` 三端已同构、camelCase 已统一；但字段级漂移 6+ 处（含导致数据丢失的 `HistoryItem`） |
| 数据流向清晰度 | 3.5 | 在线/离线双路径已收敛到 repository；但 `data_mode` 字符串透传、导入即 `load()` 的隐式副作用仍在 |
| 性能工程 | 2.0 | **零埋点、零基线**；已知热点（reqwest 每次建连、N+1 收集、逐条写入、全量重拉）均未量化 |
| 安全与稳定性 | 2.5 | 越权缺口 7 处、事务 0 处、同步无防重入、密钥明文默认值；SSRF/沙箱防护已较完善（加分项） |

### 0.3 必须立即处理的 5 个 P0 项

| # | 问题 | 位置 | 后果 |
|---|---|---|---|
| 1 | **单例 Service 用实例字段持有当前用户** | `BaseService.java:35` + 9 个 Service 的 `this.currentUser = user` | 并发请求互相覆盖 → **A 用户可能越权操作 B 的项目** |
| 2 | **读接口缺失项目越权校验** | `EnvironmentService:47/57/142/152/218`、`CategoryService:29`、`SavedRequestService:26` | 传任意 `projectId` 即可读取他人数据 |
| 3 | **全项目 0 处 `@Transactional`** | 已核验（grep 命中 0） | 注册（3 次 insert）、建项目（项目+成员）、移交所有权（3 次 update）、同步合并（N 次写）中断即产生脏数据 |
| 4 | **同步无防重入 / 无重试** | `sync.rs:344`（upload）、`:431`（pull） | 用户连点或自动同步并发 → 双向覆盖、数据回退 |
| 5 | **令牌明文落盘 + 暴露给 WebView** | `infra.rs:39`（写 `sync_config.json`）、`commands/sync.rs:310`（`get_token`） | 本地提权/脚本侧信道可窃取云端凭证 |

---

## 1. 架构全景

### 1.1 运行时拓扑

```
┌─────────────────────── 单台桌面机（一个 Tauri 进程）───────────────────────┐
│                                                                            │
│  Vue 3 WebView                     Rust 原生层                             │
│  ┌──────────────┐   invoke(64)   ┌───────────────────────────────────┐     │
│  │ UI + 编排     │──────────────▶│ commands/*  (64 个命令)            │     │
│  │ composables   │◀──────────────│ ├─ http.rs    HTTP 代理(绕 CORS)   │     │
│  │ repositories  │   返回值/事件  │ ├─ db.rs       SQLite(11 表,WAL)   │     │
│  │ stores(Pinia) │               │ ├─ sync.rs    同步编排             │     │
│  └──────────────┘               │ ├─ ws.rs/mock.rs 本地服务          │     │
│         │                        │ └─ infra.rs   reqwest + 配置       │     │
│         │ Web Worker(脚本沙箱)    └───────┬───────────────┬───────────┘     │
└─────────┼──────────────────────────────┼───────────────┼─────────────────┘
          │                              │ SQLite       │ HTTPS POST /api/v1/*
          │                              ▼              ▼
          │                    ┌──────────────┐  ┌──────────────────────────┐
          └───────────────────▶│ 本地 canghai.db│  │ Spring Boot 4 (8092)     │
                               └──────────────┘  │ AuthAspect(JWT)→Service   │
                                                 │ MyBatis-Plus → MySQL 12 表 │
                                                 └──────────────────────────┘
                                        Docker：nginx(8099) → server(8092) → mysql
```

### 1.2 三端职责边界：现状 vs 应有

| 端 | 现状 | 应有（目标态） |
|---|---|---|
| Vue | UI + **业务编排 + 冲突判定 + 在线/离线分支** | UI + 编排；分支与判定下沉 repository / 后端 |
| Rust | HTTP 代理 + SQLite + 云桥 + **部分业务逻辑**（`project.rs:27-58` 内做 JWT 解析、归属推导、HTTP 调用） | 本地执行 + 集成桥；**零业务规则** |
| Spring Boot | 权限与数据权威（基本达成） | 权威 + 事务边界 + 批量能力 |

### 1.3 核心功能域链路

| 功能域 | 主链路 |
|---|---|
| 发请求 | `ApiDebuggerView.sendRequest(:1626)` → 前置脚本(Worker) → `{{var}}` 解析 → `httpRepo.send_http_request` → Rust `http.rs:87` → 后置脚本 → `save_history` |
| 保存接口 | `saveCurrentRequest(:1870)` → `useSavedRequests.updateRequest(:156)` → `useServerApi(:109)` → `invokeApi('call_server_api')` → Rust 转发 `/api/v1/request/update` → 冲突判定 → 本地缓存 |
| 云同步 | `useSync.sync(:285)` → `syncRepo` → Rust `run_sync/pull_data` → `/api/v1/sync/upload|pull` → 服务端 `SyncService.mergeByProjectIds:196` + 墓碑回传 |
| 鉴权 | `AuthAspect:38` → `RequestBodyUtil.extractToken` → `AuthService.validateToken:142`（**每次查库**） |

---

## 2. 架构合理性评估

### 2.1 值得肯定的设计

1. **Rust 做集成枢纽**：把"绕 CORS""本地持久化""云端桥接"收敛到原生层，前端零 `fetch`，是桌面端最优解。
2. **SSRF 防护扎实**：`http.rs:12` 协议白名单、`:63-83` IP 判定、`:46-56` DNS 解析后逐 IP 校验、169.254.169.254 硬拦截。
3. **增量同步已落地**：`server_update_time` + `sync_version` + 墓碑 + 游标时区保护（`SyncService:95-110` 显式取 `UTC_TIMESTAMP(3)`），设计正确。
4. **脚本沙箱已 Worker 化**（Phase 3）：脚本无法触达 DOM 与 `__TAURI__`。
5. **DDL 已版本化**：Flyway 单事实源，`var_key` 保留决策有充分依据。

### 2.2 分层越界与职责错配

| 问题 | 证据 | 建议 |
|---|---|---|
| Service 用实例字段持有请求态用户 | `BaseService.java:35`，各 Service 首行 `this.currentUser = user` | 改为方法参数传递；`isProjectAccessible(projectId, user)` |
| Rust 命令内做业务决策 | `commands/project.rs:27-58`（JWT 解析 + 归属推导 + HTTP） | 下沉 `infra` 或后端 |
| Controller 与切面重复清理上下文 | 各 Controller `finally { clear(); }` + `AuthAspect:71` | 统一在切面 finally 清理，Controller 不再清理 |
| `db.rs` 1877 行承担模型/DDL/迁移/CRUD/合并 5 类职责 | `db.rs` 全文 | 拆为 `db/{schema,migrate,crud,sync}.rs` |

### 2.3 事务与一致性（结构性缺失）

已核验：全仓 `@Transactional` 命中 **0**。受影响的多写操作：

- `AuthService.register:73-100`（用户 + 团队 + 成员 3 次 insert）
- `ProjectService.save:59-71`（项目 + 成员 2 次 insert）
- `TeamService.transferOwnership:262-289`（成员 update + 旧 owner 降级 + 团队 owner 变更）
- `SyncService.upload:134-182`（5 类实体批量合并，行数可达数千）
- `CategoryService.batchSave:107-139`（逐条 upsert + 逐条软删）

### 2.4 部署与配置一致性

- `docker-compose.yml:34` 的 `DB_URL` 指向库名 `canghai`，`docker-compose.yml:11` 创建 `MYSQL_DATABASE: canghai`，但 README 与迁移脚本约定库名为 `canghai_api` → **首次部署存在库名不一致风险**。
- `docker-compose.yml:18` healthcheck 硬编码 `-pcanghai_pwd`，与 `:36` 的 `DB_PASSWORD: canghai_pwd` 耦合。

---

## 3. 模块耦合度分析

### 3.1 跨端耦合矩阵

| 耦合方向 | 强度 | 说明 |
|---|---|---|
| Vue → Rust | 高（64 invoke 点） | 已收敛到 12 个 repository，但仍有泄漏（`useApi.ts:50`、`useSync`） |
| Rust → Java | 中（8 个命令） | 两套路径并存：通用代理 `call_server_api` 与模块硬编码 path |
| Vue → Java | **无直连** | 正确（规避 CORS） |
| Rust 内部 | 高 | `db.rs` ↔ `sync.rs` 通过 18 个 `*_for_merge` 函数硬耦合 |
| Vue 内部 | 中高 | 循环依赖：`useSync.ts:3 ↔ useTeams.ts:2`、`useSync.ts:5 ↔ useApi.ts:3`；`categoryRepo:5/requestRepo:5` 反向依赖 composable |

### 3.2 巨型模块与死代码

| 端 | 巨型模块 | 行数 | 死代码 |
|---|---|---|---|
| Rust | `db.rs` | 1877（占该端 38%） | `commands/workspace.rs`（0 字节，未注册） |
| Vue | `ApiDebuggerView.vue` | 2216 | `useWorkspaces.ts`、`WorkspaceManager.vue`（均 0 字节） |
| Java | `SyncService.java` | 621 | 无（Phase 2 已清理 `handle` 分发器） |

### 3.3 隐式副作用（可测试性障碍）

`useSync.ts:9-10`、`useCategories.ts:10-11` 在**模块导入时**执行副作用；`useSavedRequests.ts:432`、`useEnvironments.ts:446` 在模块顶层直接 `load()` → 引入即发 RPC，单元测试无法隔离。

---

## 4. 通信协议与数据流向

### 4.1 三条链路

| 链路 | 协议 | 序列化 | 现状问题 |
|---|---|---|---|
| Vue ↔ Rust | Tauri IPC（`invoke`） | JSON（camelCase） | 错误解包三套并存（`invokeUnwrap` / `invokeApi` / `normalizeError`）；`categoryRepo:83`、`requestRepo:61` 手写 `resp?.success && resp.code===0` |
| Rust ↔ Java | HTTPS POST `/api/v1/*` | JSON | `infra::post_api_result:75-83` **不校验 HTTP 状态码**，5xx 会被当 JSON 解析；`project.rs:243/258`、`team.rs:81/94` 用 `let _ =` 丢弃后端失败仍返回成功 |
| Rust ↔ SQLite | rusqlite | — | `Mutex<Connection>` 单连接串行；已用 `in_transaction` 包批 |

### 4.2 信封与错误码

- 信封 `ApiResult{success,code,msg,data}` 三端已同构 ✅
- **Rust 返回类型三套并存**：`Result<ApiResult<T>,String>`（commands/sync.rs、project.rs、environment.rs）vs 裸 `ApiResult<T>`（http.rs:87、team.rs:41、ws.rs:54、mock.rs:55）
- **错误码信息全失**：`DbError` 枚举（`db.rs:46`）被 `.to_string()` 降级为 `-1`
- Java 侧已有 `ErrorCode` 枚举，但 Service 仍散用 `ApiResult.fail(400, "...")` 字面量

### 4.3 在线/离线数据流

`data_mode` 由前端字符串透传（`stores/dataMode.ts:43 modeArg()`）→ Rust `resolve_conn`（`db.rs:21-26`）：**非 `"offline"` 一律按 online 处理**，拼错静默走在线分支，无告警。

### 4.4 同步数据流

```
upload: collect_all(only_dirty) → POST /api/v1/sync/upload
        → 服务端 mergeXxx（逐条 compareToServer 判定）
        → buildResponse（5 表增量 + 项目/团队/成员全量 + 墓碑）
        → 客户端 apply_server_response（单事务） → 推进游标 → clear_dirty_flags
pull  : POST /api/v1/sync/pull → 只合并、不清 dirty
```

**问题**：
1. 客户端冲突判定 `sync.rs:222` **仅比较 `update_time` 字符串**，本地更新一律保留，`sync_version` 已定义但未参与 → 静默覆盖服务端 newer 数据。
2. 无防重入锁、无重试退避。
3. `apply_deleted_ids`（`db.rs:1845-1853`）按 ids × 5 表逐条执行。

---

## 5. 跨语言接口设计评估

### 5.1 契约漂移清单（按严重度排序）

| # | 契约缺口 | 三端证据 | 影响 | 修复 |
|---|---|---|---|---|
| 1 | 同步上传键名不一致 | Rust `sync.rs:14` `requests` vs Java `SyncData:31` `savedRequests` | **接口数据上传后一直被丢弃**（已用 `@JsonAlias("requests")` 临时修复） | 契约文档冻结 + 生成式类型 |
| 2 | 历史项字段缺失 | 前端 `types/index.ts:53-67` 有 `preScript/postScript`，Rust `db.rs:266-286` 无 | **脚本落库即丢失** | 补列 + 迁移脚本（新增 `V3__*.sql`） |
| 3 | `ch_history.project_id` 恒 NULL | 表有列（`db.rs:497`），结构体无字段（`db.rs:266`） | 历史无法按项目隔离 | 补字段或删列 |
| 4 | 同一实体两套类型定义 | `types/index.ts:185` vs `useSavedRequests.ts:28`（method 类型、`sortOrder` 必填性不同） | 类型检查失效 | 单一来源 |
| 5 | 排序字段命名不统一 | `Category.order` vs `SavedRequest.sortOrder` vs Rust `sort_order` | 需手工映射（`useCategories.ts:32`） | 统一 `sortOrder` |
| 6 | 前端臆造字段 | `types/index.ts:155` `ownerName`，Rust 无 | `useProjects.ts:96` 用 `createBy` 伪造 | 删除或后端补齐 |
| 7 | 硬编码常量双写 | `history.rs:14` 20 条 vs `useHistory.ts:7` `MAX_HISTORY=20` | 易漂移 | 后端下发配置 |

### 5.2 时间基准（双时钟问题）

- Java `BaseService.now():63` 用 `LocalDateTime.now()`（JVM 时区，通常 UTC+8）
- MySQL `server_update_time` 由 `CURRENT_TIMESTAMP(3)`（容器常为 UTC）维护
- Rust `db.rs:380` 手写 `now_timestamp`（闰年/时区风险）

→ 两条时间线并存。`SyncService` 已用 `UTC_TIMESTAMP(3)` 做游标规避，但**业务 `update_time` 仍是本地时区**，一旦跨时区协作或容器时区变更即错乱。

### 5.3 缺失的横切能力

| 能力 | 现状 |
|---|---|
| 参数校验 | 三端均为 0：Java 无 `@Valid`、Rust 无 validator、前端靠 TS（运行时无保障） |
| 接口版本 | 已有 `/api/v1` ✅，但无废弃/兼容策略 |
| 幂等 | 同步合并为 upsert，其余写接口无幂等键 |

---

## 6. 性能瓶颈

> ⚠️ 当前**零埋点、零基线**。以下为静态分析得出的热点，Phase 6 必须先补埋点再验证收益。

### 6.1 端到端热点

| 优先级 | 瓶颈 | 位置 | 影响机制 | 预估量级 |
|---|---|---|---|---|
| P1 | reqwest Client 每次新建 | `infra.rs:48`、`http.rs:100/200` | 无连接池 → 每次请求重新 TLS 握手 | 单次请求 +100~300ms（取决于 RTT） |
| P1 | 同步收集 N+1 | `sync.rs:103-113` | 每项目查环境/分组，每环境查变量 | 100 项目 × N → 数百次查询 |
| P1 | 服务端逐条写入 | `SyncService:320-446` 逐条 insert/update | 无批量 upsert、无事务 | 1000 行 ≈ 2000+ 次 round-trip |
| P1 | 前端全量重拉 | `useSavedRequests:148/192/215`、`useEnvironments:108/140/347/439` | 每次增删改都 `load()` 全量 | 数据量大时 UI 卡顿 |
| P2 | 无虚拟滚动 | `CategoryTree.vue:173`、`ApiDebuggerView:994` | 全量 `v-for` | 千级节点渲染 >1s |
| P2 | deep watch + 全量持久化 | `useTabs.ts:97`、`ApiDebuggerView:1346-1349` | 每次按键 `JSON.stringify` 写 localStorage | 输入延迟 |
| P2 | JWT 每次查库 | `AuthAspect:57` → `AuthService:147` | 每个请求 1 次 `selectOne` | 高频请求下放大 DB 负载 |
| P2 | 列表接口无分页 | `CategoryService.list:32`、`SavedRequestService.list:30` 等 6 处 | 全表加载 | 大项目内存/延迟 |
| P2 | 存历史触发全表 DELETE 子查询 | `db.rs:1148` | 每次写入额外全表扫描 | 历史增长后线性劣化 |
| P3 | SSE 每 chunk 一次 IPC | `http.rs:242-299` | 事件洪泛 + `full` 全量累积 | 大响应打爆 IPC |
| P3 | JSON 多次解析 | Filter → `AuthAspect:55` → `RequestBodyUtil:39` → Jackson 绑定 | 同一 body 被解析 3~4 次 | CPU 浪费 |
| P3 | `ch_projects` 无二级索引 | `V1__init_schema.sql:89-98` | 按 `deleted`/`create_by` 过滤走全表 | 数据量增长后劣化 |

### 6.2 各端 Top 3

- **Rust**：Client 不复用 > 同步 N+1 > 每 chunk IPC
- **Java**：逐条写入 > 无事务（含失败重试成本） > JWT 查库
- **Vue**：全量重拉 > 无虚拟滚动 > deep watch 持久化

---

## 7. 安全与稳定性

| 类别 | 问题 | 证据 | 建议 |
|---|---|---|---|
| 越权 | 7 个读接口无 `isProjectAccessible` | `EnvironmentService:47/57/142/152/218`、`CategoryService:29`、`SavedRequestService:26` | 统一读路径校验（写路径已有） |
| 并发 | `currentUser` 实例字段 | `BaseService.java:35` | 改方法参数（P0-1） |
| 凭证 | token 明文落盘 + `get_token` 暴露 | `infra.rs:39`、`commands/sync.rs:310` | 系统钥匙串（keyring）/ 移除暴露命令 |
| 配置 | DB 密码明文默认值 | `application-local.yaml:10` `password: ${DB_PASSWORD:abc@1234}` | 移除默认值 |
| SSRF | DNS rebinding（校验后二次解析） | `http.rs:46` vs `:117` | 自定义 `Resolver` 固定解析结果 |
| 代理 | `call_server_api` path 仅黑名单校验 | `commands/sync.rs:388-393` | 白名单枚举 + 规范化 |
| 日志 | mapper SQL debug 打印参数 | `application-local.yaml:42` | 生产关闭 |
| 审计 | `afterJson` 落库完整请求体（含脚本） | `AuditService:33` | 脱敏 + 体积上限 |
| 沙箱 | 黑名单可被字符串拼接绕过 | `useScriptEngine.ts:151` | 已靠 Worker 兜底，长期改 QuickJS-WASM |

---

## 8. 分阶段优化计划（Phase 5–8）

### 8.1 编排原则

1. **先止血、后优化**：正确性与安全（Phase 5）优先于任何性能项。
2. **先可观测、再优化**：Phase 6 先建埋点与基线，Phase 7 的收益才可被度量与证伪。
3. **契约先行**：跨端改动以 `API_CONTRACT.md` 冻结为前提，禁止口头约定。
4. **每阶段可独立发布**，失败可回滚（DB 变更一律新增 `V{n}__*.sql`，禁止改写已提交脚本）。
5. 工作量单位为**粗估人日（1 人）**，含自测与回归。

### 8.2 路线总览

| 阶段 | 主题 | 目标 | 优先级 | 粗估 | 前置 |
|---|---|---|---|---|---|
| **Phase 5** | 正确性与安全止血 | 消除越权与数据错乱风险 | P0 | 8–12 人日 | — |
| **Phase 6** | 契约统一与可观测 | 契约可校验、性能可度量 | P1 | 10–14 人日 | Phase 5 |
| **Phase 7** | 性能与体验 | 量化达标（见 8.6） | P1 | 15–20 人日 | Phase 6 基线 |
| **Phase 8** | 架构收敛 | 降低耦合、提升可维护性 | P2 | 20–25 人日 | Phase 7 |

---

### 8.3 Phase 5：正确性与安全止血（P0）

**目标**：消除可导致越权、数据丢失、凭证泄露的缺陷；建立事务边界。

| # | 任务 | 主要改动点 | 验收标准（可度量） | 人日 |
|---|---|---|---|---|
| 5.1 | `currentUser` 改为方法参数 | `BaseService.java:35` + 9 个 Service 全部 `this.currentUser = user` | 并发测试：10 线程 × 不同用户并发调用 `/project/save` 与 `/category/list`，归属正确率 100%；`grep "currentUser =" ` 命中 0 | 2 |
| 5.2 | 读接口补越权校验 | `EnvironmentService` 5 处 + `CategoryService.list` + `SavedRequestService.list` | 新增 7 个越权用例（A 用户传 B 的 `projectId` 全部 403）；用例通过率 100% | 1.5 |
| 5.3 | 事务边界 | `register` / `ProjectService.save` / `transferOwnership` / `CategoryService.batchSave` / `SyncService.upload` 加 `@Transactional` | 故障注入（中途抛异常）后 DB 无残留脏行；回归用例通过 | 2 |
| 5.4 | 同步防重入 + 重试 | Rust `sync.rs:344/431` 加 `Mutex<()>`/`AtomicBool` + 指数退避重试（3 次） | 并发触发 2 次 `sync_data` 只执行 1 次（日志计数校验）；弱网注入 3 次失败后最终成功 | 2 |
| 5.5 | 凭证治理 | `infra.rs:39` 改用系统钥匙串；移除或收紧 `get_token` | `sync_config.json` 中无明文 token；`grep auth_token` 落盘命中 0 | 2 |
| 5.6 | 配置与部署一致性 | 统一库名（`docker-compose` / README / 脚本）；移除 `application-local.yaml:10` 明文默认密码 | 全新 `docker compose up` 一次成功；`grep abc@1234` 命中 0 | 1 |
| 5.7 | 冲突判定纳入 `sync_version` | Rust `sync.rs:222` 对齐服务端 `SyncService.compareToServer:127` | 构造同秒并发修改用例，结果与服务端一致 | 1 |

**阶段出口指标**：
- P0 缺陷清零（5 项验收全通过）
- 越权回归用例集 ≥ 10 条，CI 集成
- 生产无 P0 级数据错乱反馈

#### Phase 5 执行状态（复核结论）

| # | 任务 | 状态 | 落地内容 |
|---|---|---|---|
| 5.1 | `currentUser` 改为方法参数 | ✅ | `grep currentUser` 全仓 **0 命中**；用户经 `RequestContext`（ThreadLocal，由 `AuthAspect` 注入、`finally` 清理）+ 方法参数传递，`BaseService.isProjectAccessible(projectId, user)` |
| 5.2 | 读接口补越权校验 | ✅ | `isProjectAccessible` 已覆盖原 7 个读接口（`EnvironmentService` 5 处 + `CategoryService.list` + `SavedRequestService.list`），并顺带覆盖 `AuditService.query`、`ProjectMemberService` 等 |
| 5.3 | 事务边界 | ✅ | 6 处 `@Transactional(rollbackFor = Exception.class)`：`AuthService.register`、`ProjectService.save/delete`、`TeamService.transferOwnership`、`CategoryService.batchSave`、`SyncService.upload(timeout=120)` |
| 5.4 | 同步防重入 + 重试 | ✅ | `sync.rs`：`SYNC_RUNNING: AtomicBool` + RAII `SyncGuard`（upload/pull 共用同一把锁）；`post_json_with_retry` 仅对传输层错误指数退避重试 3 次（业务响应不重试，避免重复写入） |
| 5.5 | 凭证治理 | ✅ | `infra.rs`：令牌写入系统钥匙串（`keyring`），落盘 `sync_config.json` 中 `auth_token` 置空、登出删除条目；钥匙串不可用的平台回退明文落盘以保证可用性 |
| 5.6 | 配置与部署一致性 | ✅ | `docker-compose.yml` 库名统一 `canghai_api`、密码与 healthcheck 均走 `${MYSQL_*}`（不再硬编码）；`application-local.yaml` DB 密码仅 `${DB_PASSWORD:}` 无明文默认值 |
| 5.7 | 冲突判定纳入 `sync_version` | ✅ | Rust `server_is_newer` 先比 `update_time`、同一秒再比 `sync_version`，与服务端 `SyncService.compareToServer` 判定规则一致 |

> 本轮对 Phase 5 全部 7 项做了逐条复核（grep + 代码走查 + 编译），**无需改动即可确认达标**。

---

### 8.4 Phase 6：契约统一与可观测（P1）

**目标**：让三端契约**可被机器校验**，让性能**可被度量**。

| # | 任务 | 主要改动点 | 验收标准 | 人日 |
|---|---|---|---|---|
| 6.1 | 契约单一事实源 | `docs/API_CONTRACT.md` 补全全部接口；新增 `scripts/contract-check` 比对 Java DTO / Rust struct / TS type 字段 | CI 执行契约检查，差异数 = 0 | 3 |
| 6.2 | 修复 7 项契约漂移 | 见 §5.1（含 `HistoryItem` 补列 → 新增 `V3__history_scripts.sql`） | 历史中的脚本可完整还原；字段 diff = 0 | 3 |
| 6.3 | 时间基准统一 | 业务 `update_time` 迁移 UTC；Rust 删除 `now_timestamp` 改用 `chrono::Utc` | 三端时间互解析误差 < 1s；跨时区容器回归通过 | 2 |
| 6.4 | 埋点与基线 | Rust 侧同步/HTTP 耗时打点；Java 侧 Filter 记录 `uri,ms,rows`；前端 `performance.now` 关键链路 | 产出基线报告：P50/P95 同步耗时、请求代理延迟、首屏 TTI | 4 |
| 6.5 | 错误码统一 | Rust `DbError` 映射整数码（`thiserror`）；Java 全量替换字面量为 `ErrorCode`；前端只留 `invokeUnwrap` | 错误码表覆盖 100% 业务错误；三套解包收敛为 1 套 | 3 |
| 6.6 | 参数校验 | Java `@Validated` + 全局 `MethodArgumentNotValidException`；Rust 引入 `validator` | 核心写接口校验覆盖率 100% | 2 |

**阶段出口指标**：
- 契约检查进 CI，字段差异 = 0
- 基线报告产出（后续所有性能收益以此为参照）
- 错误码表落地，前端解包方式收敛为 1 种

#### Phase 6 执行状态（截至本轮）

| # | 任务 | 状态 | 落地内容 / 遗留 |
|---|---|---|---|
| 6.1 | 契约单一事实源 | ✅ | 新增 `scripts/contract-check.mjs`（实体字段集 TS⊆Rust、响应信封、`requests` 别名、错误码表）；`docs/API_CONTRACT.md` 补 §8 机器校验；**白名单已清空（字段差异 = 0）**；新增 `.github/workflows/contract-check.yml` **接入 CI**（同时执行 `scripts/dep-graph-check.mjs`）。按 8.7.1 建议补两类静态检查：① `@TableName` 有 typeHandler 字段时强制 `autoResultMap`；② JSON 列的 `UpdateWrapper.set` 必须显式序列化（已用临时探针反向验证能拦住） |
| 6.2 | 修复契约漂移 | ✅ | Rust `HistoryItem` 补 `pre_script/post_script/project_id`（含 SQLite 轻量迁移，修复「脚本落库即丢」）；历史上限改由后端 `get_history_limit` 单一来源下发；移除臆造字段 `ownerName`；**顺带修复真实 bug**：`RawProject.order` 误读导致项目排序恒为 0（改为读 `sortOrder`）。**两项遗留已清零**：#4 TS 类型双份（`types/index.ts` vs `useSavedRequests.ts`）已收敛到 `@/types`；#5 命名统一 —— `Project.order`/`Category.order` → `sortOrder`（与 Rust 同形，`useCategories`/`useProjects`/`useImporters`/`projectRepo` 及冲突对比字段一并更新），`Environment.sortOrder` 经核查为「Rust 未落库、UI 从未读取」的死字段已删除，`vue-tsc` 通过 |
| 6.3 | 时间基准统一 | ✅ | Java `now()`→UTC（`ZoneOffset.UTC`）、Rust `now_timestamp()`→`chrono::Utc`（移除手写公历换算）、前端 `now()`→UTC 并新增 `parseServerTime()` 展示转换；同步修正 `useProjects` 的 ISO 格式与 `useImporters.nowStr()` 本地时区 |
| 6.4 | 埋点与基线 | ✅ 代码就绪（采集已工具化） | Java `TimingConfig` 过滤 `/api/*` 记录 `uri/method/status/cost(ms)`（≥500ms 记 WARN）；Rust `post_api_result` 与 `run_sync`/`run_pull` 打点耗时。新增 `scripts/perf-baseline.mjs`：统一解析上述三种日志行，按「server METHOD uri(status)」/「desktop http POST path(status)」/`run_sync`/`run_pull` 分组输出 count/min/P50/P95/P99/max/mean 的 Markdown 表格（nearest-rank 口径，`--json`/`--out=` 可选），已用样例日志验证。**遗留（需真实负载）**：运行一轮采集，把表格写入基线报告 |
| 6.5 | 错误码统一 | ✅ | ① Java：服务层 **全部** `ApiResult.fail(4xx"/5xx", ...)` 字面量（9 个 Service、100+ 处）替换为 `ErrorCode.*`；`fail(ErrorCode, detail)` 语义改为「detail 直接作为 msg」（不再拼 `标准文案：detail`）；`BaseService.fail(int,…)` 改为 `fail(ErrorCode,…)` 以阻断后续字面量。② Rust：新增 `sync::codes` 常量模块（对齐契约 §3）+ `DbError::code()` 映射（`Init→50000`，`LockPoisoned/Sql/Migration→50001`）；新增 `ApiResult::from_db_err`，把原先 **38 处** `.to_string()` 压成 `-1` 的降级写法改为带业务码；`infra.rs`/`commands/*` 的传输类错误归 `DOWNSTREAM_ERROR(50002)`，SSRF/非法 URL 归 `PARAM_INVALID`。③ 前端：新增 `lib/tauri.ts::unwrapResult + ApiError(code)` 作为**唯一**信封解析实现，`invokeUnwrap` 与在线代理 `invokeApi` 共用；删除无用 `invokeSafe`；`categoryRepo/requestRepo/teamRepo/projectRepo/projectMemberRepo/historyRepo/environmentRepo/auditRepo/wsRepo` 中 9 处手写 `resp?.success && resp.code === 0` 全部收敛。④ 契约表补 `50001 DB_ERROR`/`50002 DOWNSTREAM_ERROR`（Java 枚举 + 文档 + Rust 常量三处同步）。 |
| 6.6 | 参数校验 | ✅ 三端齐备 | 后端：`spring-boot-starter-validation` + 核心写 DTO `@NotBlank` + 控制器 `@Valid` + `GlobalExceptionHandler` 归一 `PARAM_INVALID`。**Rust 侧已补齐**：引入 `validator 0.21`（derive），规则写在 `db/models.rs` 的结构体上（`Category/SavedRequest/Environment/EnvironmentGroup/EnvironmentVariable/Project/Team` 的必填与长度上限，文案与 Java `@NotBlank` 对齐），命令层统一走 `commands/valid.rs`（`check` 结构体校验 + `required` 标量必填 + `as_fail` 适配裸 `ApiResult<T>` 的同步命令）；覆盖 `category/saved_request/environment/project/team` 全部写命令，`cargo check` 零告警 |

> **Phase 7 启动条件**：6.4 的埋点代码已就绪，需跑一轮真实负载产出基线报告后方可开始性能对比。

---

### 8.5 Phase 7：性能与体验（P1）

**目标**：以 Phase 6 基线为准，达成下表量化指标。

| # | 任务 | 改动点 | 目标（相对基线） | 人日 |
|---|---|---|---|---|
| 7.1 | reqwest Client 复用 | `infra.rs` 全局懒静态 `OnceLock<Client>`，http.rs 复用 | 单次代理请求 P95 **-30%** | 1.5 |
| 7.2 | 同步收集去 N+1 | `sync.rs:103-113` 改批量 `IN` 查询 | 100 项目场景收集耗时 **-60%** | 2 |
| 7.3 | 服务端批量 upsert | `SyncService` 用 `saveBatch` / 批量 `INSERT ... ON DUPLICATE KEY UPDATE` | 1000 行合并 DB round-trip **-80%** | 3 |
| 7.4 | JWT 校验缓存 | `AuthService.validateToken` 加短 TTL Caffeine 缓存（60s） | 高频请求 DB QPS **-50%** | 1 |
| 7.5 | 前端去重与增量刷新 | 用乐观更新替代 `load()` 全量重拉（`useSavedRequests` / `useEnvironments`） | 增删改后接口数 **-70%** | 3 |
| 7.6 | 大列表虚拟滚动 | 分类树 / 接口列表 / 历史引入虚拟列表 | 1000 节点首渲染 **< 300ms** | 3 |
| 7.7 | 持久化去抖 | `useTabs` deep watch 改防抖 500ms + 增量 patch | 输入 P95 延迟 **-50%** | 1.5 |
| 7.8 | SSE/流式节流 | `http.rs:242-299` 事件合批（50ms） + 上限保护 | 大响应 IPC 事件数 **-90%** | 2 |
| 7.9 | 补索引 | `ch_projects(deleted, create_by)` 等（新增 `V4__perf_index.sql`） | 相关查询 `EXPLAIN` 走索引（`type=ref`） | 1 |

**阶段出口指标**（全部以 Phase 6 基线对比，CI 中固化性能冒烟）：
- 同步 P95 耗时下降 ≥ 50%
- 单次代理请求 P95 下降 ≥ 30%
- 大项目（1000 接口）首屏 TTI < 1.5s

#### Phase 7 执行状态（截至本轮）

| # | 任务 | 状态 | 落地内容 / 遗留 |
|---|---|---|---|
| 7.1 | reqwest Client 复用 | ✅ | `infra.rs` 新增 `PROXY_CLIENT`/`proxy_client()`（懒初始化 + UA/超时/禁重定向），`http.rs` 普通与流式两条路径均改为复用；原先每次请求 `Client::builder().build()` 的重复建连与 TLS 握手被消除 |
| 7.2 | 同步收集去 N+1 | ✅ | `collect_all` 原按项目逐个查环境/分组、按环境逐个查变量；改为 `db::get_all_environment_groups_all` / `get_all_environments_all` / `get_all_env_variables_all` 三条**全量**查询（同步本就是团队级全量语义），并抽出 `map_env_var` 复用映射。100 项目场景由 200+ 次查询降为 3 次 |
| 7.3 | 服务端批量 upsert | ✅（已端到端验证） | 5 张同步表的 Mapper 新增 `batchUpsert`（MyBatis `<foreach>` 多值 `INSERT ... ON DUPLICATE KEY UPDATE`，冲突时只更新业务列、不覆盖 `create_time/create_by`，`sync_version` 仍由触发器自增）；`SyncService` 合并逻辑改为「先收集待写入集合、再按 200 行分片批量执行」，逐行 insert/update 全部删除。**验证**：插入 `syncVersion=1` → 更新 `syncVersion=2` 且 `create_time` 保持 → 重复上报幂等不写入 |
| 7.4 | JWT 校验缓存 | ✅ | `AuthService.validateToken` 加 60s TTL + 1000 上限的 `ConcurrentHashMap` 缓存（含过期清理）。`AuthAspect` 每请求一次的 `selectOne` 在高频场景下降为每 token 每 60s 一次 |
| 7.5 | 前端去重与增量刷新 | ✅ | 状态收归 Pinia 后由 store 提供最小变更原语（`upsert`/`removeById`/`upsertVariable` 等），写路径不再全量重拉：`useSavedRequests` 的 create/update/delete（在线含冲突确认分支）与 `useEnvironments` 的环境/分组/变量增删改全部改为就地更新。**效果**：一次「保存接口」由「1 写 + 1 全量读」降为「1 写」；保存/更新环境变量不再触发 `getVariables(envId, true)` 整表重拉。**遗留**：`load()` 仍在「切换项目/数据模式」与首屏显式初始化时调用（全量重拉只在这两类场景保留） |
| 7.6 | 大列表虚拟滚动 | ✅ 部分 | 新增零依赖的 `composables/useVirtualWindow.ts`（定长窗口 + 上下 padding 占位，单一处行标记，避免模板重复），并应用于 **分类下的接口列表**（`CategoryNode.vue`，真正的千级场景）：超过 60 条才启用窗口化，普通分类保持原有展开式布局（无内层滚动条）不变，虚拟模式下行高固定 30px。**遗留**：分类树本身（`CategoryTree` 递归节点）未窗口化 —— 需先把树压平并接管侧栏滚动容器，属需交互回归的前端专项；`ApiDebuggerView` 的历史列表上限仅 20 条，无虚拟化必要 |
| 7.7 | 持久化去抖 | ✅ | `useTabs` 原 `watch([tabs, activeTabId], …, { deep: true })` 每次按键都深遍历 + `JSON.stringify` + 同步写 localStorage；拆为「结构性变更（`tabs.length` / 活跃页签）立即落盘」+「内容变更 500ms 防抖落盘」，避免丢页签的同时压掉高频写入 |
| 7.8 | SSE/流式节流 | ✅ | `http.rs`：普通分块文本按 **50ms 间隔 / 64KB 体积** 合批后再 emit（原先每收到一个网络分块即发一次 IPC）；SSE 把**同一次网络读取内**解析出的多个事件合并为一次 emit（直接首尾拼接，前端累积文本完全一致、不引入实时性延迟）；流结束前冲刷合批缓冲 |
| 7.9 | 补索引 | ✅ | 新增 `V4__perf_index.sql`：`ch_projects(deleted,create_by)`、`ch_categories(project_id,deleted)`、`ch_saved_requests(project_id,deleted)`、`ch_environments(project_id,deleted)`、`ch_environment_groups(project_id,deleted)`、`ch_environment_variables(environment_id,deleted)`、`ch_team_members(team_id,deleted)`、`ch_audit_logs(project_id,create_time)`；只新增不删除既有索引，已在本地库执行成功 |

> **待补**：7.1~7.9 的「优化前后 P95 对照数据」（先按 6.4 用 `scripts/perf-baseline.mjs` 采集基线，代码侧已无阻塞）。7.5 已落地、7.6 已覆盖千级列表场景，仅剩「分类树整体窗口化」这一需交互回归的子项。

---

### 8.6 Phase 8：架构收敛（P2）

**目标**：把"能跑"变成"好维护"。

| # | 任务 | 改动点 | 验收标准 | 人日 |
|---|---|---|---|---|
| 8.1 | 拆分 `db.rs`（1877 行） | 拆为 `db/{schema,migrate,crud,sync}.rs` | 单文件 < 600 行；`cargo check` + 回归通过 | 4 |
| 8.2 | 拆分 `ApiDebuggerView.vue`（2216 行） | 按请求面板/响应面板/脚本面板/环境选择拆子组件，状态下沉 store | 单组件 < 500 行；`vue-tsc` + build 通过 | 6 |
| 8.3 | 消除循环依赖 | `useSync ↔ useTeams`、`useSync ↔ useApi`；repository 不再反向依赖 composable | 依赖图无环（`madge --circular` 输出为空） | 3 |
| 8.4 | 状态收归 Pinia | `useSavedRequests`/`useCategories`/`useProjects`/`useEnvironments` 模块级单例 → store；移除导入即 `load()` | 无模块级副作用（`grep "^load()"` = 0）；可单测 | 5 |
| 8.5 | Rust 返回类型统一 | 全部命令统一为 `ApiResult<T>`（去掉 `Result<ApiResult<T>,String>` 双层） | 类型签名一致；`cargo check` 通过 | 3 |
| 8.6 | 清理死代码与空文件 | `commands/workspace.rs`、`useWorkspaces.ts`、`WorkspaceManager.vue` | 删除完成，构建通过 | 0.5 |
| 8.7 | 审计补全 before 快照 + 脱敏 | `BaseService.audit` | 审计记录含变更前后值；敏感字段脱敏 | 2 |

**阶段出口指标**：
- 最大文件 < 600 行（Rust）/ < 500 行（Vue）
- 静态依赖图无环
- 核心 composable 具备单元测试

#### Phase 8 执行状态（截至本轮）

| # | 任务 | 状态 | 落地内容 / 遗留 |
|---|---|---|---|
| 8.1 | 拆分 `db.rs`（1877 行） | ✅ | 已拆为 `db/{mod,models,schema,sync}.rs` + `db/crud/{projects,categories,history,requests,environments}.rs`；`db/mod.rs` 用 `pub use` 重新导出，**`crate::db::*` 路径零改动**（调用方无一处修改）。行数：1948 → 最大单文件 352 行（models.rs），全部 < 600 行。切分由一次性脚本按顶层条目 + 文档注释边界完成（纯搬迁，语义不变），`cargo check` 零错误零告警。附带修好一个隐患：`SYNC_TABLES` 由私有改为 `pub(crate)`，使 `schema.rs` 与 `sync.rs` 共用同一份表清单 |
| 8.2 | 拆分 `ApiDebuggerView.vue`（2216 行） | ⏳ 未开始（已给出可执行切分方案） | 实测结构：template 1065 行（1–1065）+ script 1145 行（1067–2212）+ style 3 行。达到「单组件 < 500 行」需按 **请求面板 / 响应面板 / 脚本与日志面板 / 环境与历史面板** 拆 4+ 子组件，并把 `form`/`response`/`scriptLog`/`history` 等状态下沉到 store（与 8.4 同批）。**为何本轮不动**：文件承载调试主流程，`props/emits` 契约复杂，必须配合真实交互回归；在无运行环境的情况下拆分，风险高于收益（`vue-tsc` 只能保证类型与模板编译，无法验证交互） |
| 8.3 | 消除循环依赖 | ✅ | ① 共享态下沉到叶子模块：新增 `stores/session.ts`（Pinia），`composables/useSession.ts` 改为 `storeToRefs` 兼容导出，`useTeams`/`useSync` 引用随之断链。② 传输层下沉到 `lib/`：`composables/useApi.ts` → `lib/server.ts`、`composables/useServerApi.ts` → `lib/serverApi.ts`（均保留同名兼容 re-export），依赖方向统一为 `repositories → lib → stores`，**4 个反向依赖 composable 的 repository 全部改完**。③ 新增零依赖校验脚本 `scripts/dep-graph-check.mjs`（DFS 三色环检测 + 「repository 不得依赖 composable」分层规则，替代 madge；脚本自身已通过「注释里的示例 import 被误判成环」的假阳性用例验证），当前输出：69 模块 / 197 边 / **0 环 / 0 分层违规** |
| 8.4 | 状态收归 Pinia | ✅ 部分 | ① **已收归**：会话态 → `stores/session.ts`；接口列表 → `stores/savedRequests.ts`；环境/分组/变量/变量缓存 → `stores/environments.ts`（三者均保留 `storeToRefs` 兼容导出，视图零改动），store 只持状态与最小变更原语、不含业务编排，因而**可单测**。② **已移除模块级副作用**：`useSavedRequests.ts` / `useEnvironments.ts` 顶层的 `load()`（「import 即发 RPC」）删除，改为「照既有约定由视图在 `onMounted` 显式加载」（`ProjectSelectView` 原本依赖该副作用，已补显式调用），`grep "^load()"` = 0。**遗留**：`useCategories.ts` / `useProjects.ts` 仍有模块级 `useDataMode()/useRepo()/useConflict()` 实例化（无 RPC，但仍是导入副作用），需与 8.2 的组件拆分同批改造；前端仍无测试框架（无 vitest 配置），「可单测」尚停留在结构层面 |
| 8.5 | Rust 返回类型统一 | ❌ **不可行（结论：保留双层）** | 实测尝试后发现 **Tauri 硬约束**：async 命令若参数含引用（本项目大量命令带 `tauri::State<'_, AppDb>`）**必须返回 `Result`**，否则宏直接报 `async commands that contain references as inputs must return a Result`。因此 `Result<ApiResult<T>, String>` 并非冗余噪声，而是框架要求；仅纯同步命令可去掉外层。已完整回滚该项，不建议再排期 |
| 8.6 | 清理死代码与空文件 | ✅ | 删除 `commands/workspace.rs`、`composables/useWorkspaces.ts`、`components/WorkspaceManager.vue`（均 0 字节），并同步清理自动生成的 `components.d.ts` 中的残留声明；`cargo check` / `vue-tsc` 通过 |
| 8.7 | 审计补全 before 快照 + 脱敏 | ✅（已端到端验证） | ① `BaseService.audit` 增加带 `before` 的重载 + `snapshot(mapper,id)` 助手；`SavedRequestService`/`CategoryService` 的 update/delete 共 4 处在写库**之前**取快照（create 的 before 天然为空）。② `AuditService.log` 统一走 `toMaskedJson`：字段名敏感（token/password/authorization/…）直接打码；**KV 形态**（`{"key":"Authorization","value":"…"}`）按 `key` 的值判定并打码同级 `value`；**内嵌 JSON 字符串**（如 `body`）解析后递归打码；单条快照截断到 20KB。**验证**：`beforeJson` 已含变更前全量字段，`Authorization` 头与请求体内 `pwd` 均显示为 `***`，非敏感字段（`X-Trace`）保持原值 |

---

### 8.7.1 本轮验证时发现并修复的存量缺陷（原评审未列出）

以下 4 项均是在执行 Phase 6.5 / 7 的**端到端验证**中被真实请求打出来的「功能不可用」级缺陷，原评审清单未覆盖，已一并修复：

| # | 缺陷 | 现象 | 根因 | 修复 |
|---|---|---|---|---|
| B1 | 环境变量链路整体不可用（DDL 漂移） | 环境变量任何读写/同步均报 `Unknown column 'var_key' in 'field list'` | Phase 3 前的旧 `db.sql` 列名为 `key`；存量库走 `baseline-on-migrate` 到 v1 后**不会重跑 V1**，旧列名被永久保留 | 新增幂等迁移 `V5__fix_env_var_key_drift.sql`（仅当旧列存在且新列不存在时 `CHANGE COLUMN`） |
| B2 | 环境变量查询 SQL 语法错误 | `var_key AS key` → `You have an error in your SQL syntax … near 'key,value,enabled…'` | 实体 `@TableField("var_key")` 而属性名叫 `key`，属性名与列名转驼峰不一致时 MyBatis-Plus 自动补别名 `AS key`，而 `KEY` 是保留字且别名未加反引号 | 实体属性改名 `varKey`（与列名同形 → 不再生成别名），对外 JSON 由 `@JsonProperty("key")` 保持不变；同时修好 `selectActiveVariables` 的 `v.*` 结果映射（原先 `var_key` 无法映射到 `key`，key 恒为 null） |
| B3 | 请求参数/请求头/表单体**回读恒为空** | 写入 DB 内容正确，但查询返回 `params/headers/form_body` 全为 null → 同步下发会清空客户端请求头与参数 | 实体声明了 `@TableField(typeHandler = JacksonTypeHandler.class)`，但 `@TableName` 未开 `autoResultMap` → typeHandler **只在写入生效**，查询不走 typeHandler | `@TableName(value = "ch_saved_requests", autoResultMap = true)` |
| B4 | 接口更新接口直接失败 | `POST /api/v1/request/update` 返回「服务器内部错误」 | `UpdateWrapper.set("headers", List<KV>)` 不会应用 typeHandler，MyBatis 无法绑定 `ArrayList` → `Cannot convert class java.util.ArrayList to SQL type` / `NotSerializableException: KV` | 显式 JSON 序列化后再 set（新增 `jsonOrNull` 助手，`null` 保持 null，避免向 JSON 列写入字面量 `"null"`） |

> 结论：B1/B2 互为因果（DDL 漂移 × ORM 别名规则），B3/B4 同源于 MyBatis-Plus 的 typeHandler 约束。建议后续在 `scripts/contract-check.mjs` 中补两类静态检查：① `@TableName` 是否开启 `autoResultMap`（存在 typeHandler 字段时强制）；② `UpdateWrapper.set` 是否误传集合/POJO 到 JSON 列。这样这类缺陷能被 CI 拦住而不是靠端到端手测发现。

---

### 8.7 优先级矩阵（价值 / 成本）

| 高价值·低成本（立即做） | 高价值·高成本（排期做） |
|---|---|
| 5.2 读接口越权校验 · 5.6 配置一致性 · 6.5 错误码 · 7.1 Client 复用 · 7.4 JWT 缓存 · 7.9 索引 · 8.6 删死代码 | 5.1 currentUser 重构 · 5.3 事务 · 7.3 批量 upsert · 7.6 虚拟滚动 · 8.1/8.2 拆巨型文件 · 8.4 Pinia 收编 |
| 低价值·低成本（顺手做） | 低价值·高成本（暂缓） |
| 8.6 空文件清理 · 6.1 契约文档补全 | 引入 Rust 连接池（r2d2，Phase 4 已评估收益不抵风险）· 全面重写同步协议 |

---

### 8.8 本轮（2026-09-17）落地小结

| 类别 | 项目 |
|---|---|
| **本轮完成** | 6.1 契约检查补齐+接入 CI（白名单清空、差异=0）· 6.2 契约漂移清零（含命名统一）· 6.4 基线采集工具化 · 6.6 Rust `validator` 校验 · 7.5 乐观更新替代全量重拉 · 7.6 千级接口列表窗口化 · 8.1 `db.rs` 拆分 · 8.3 循环依赖清零 · 8.4 状态收归 Pinia + 移除 import 副作用 |
| **部分完成** | 7.6 分类树整体窗口化（需交互回归）· 8.4 `useCategories/useProjects` 模块级实例化与前端测试框架 |
| **明确未做** | 8.2 `ApiDebuggerView.vue` 拆分（已给出切分方案，需真实交互回归）· 6.4 真实负载基线数值（需运行一轮采集） |
| **判定不可行** | 8.5 Rust 返回类型统一（Tauri 框架硬约束，保留双层，不建议再排期） |

**本轮验证证据**（全部为可复现命令）：

| 命令 | 结果 |
|---|---|
| `cargo check`（client/apps/desktop） | 0 error / 0 warning |
| `npx vue-tsc --noEmit`（client/apps/web） | 0 error |
| `node scripts/contract-check.mjs` | 7 个实体差异 = 0，退出码 0（含 2 项新增 ORM 静态检查，已用探针反向验证拦截能力） |
| `node scripts/dep-graph-check.mjs` | 69 模块 / 197 边，0 环 / 0 分层违规 |
| `node scripts/perf-baseline.mjs <log>` | 三端埋点解析正确，输出 P50/P95/P99 表格（样例日志验证） |

---

## 9. 度量体系（让优化可被证伪）

| 类别 | 指标 | 采集点 | 用途 |
|---|---|---|---|
| 链路耗时 | 同步 upload/pull P50/P95 | Rust `sync.rs` 打点 | Phase 7 主指标 |
| 链路耗时 | HTTP 代理 P50/P95 | Rust `http.rs` 打点 | 7.1 验证 |
| 后端 | `uri, ms, rows, err` | 新增 `MetricsFilter` | 7.3/7.4/7.9 验证 |
| 前端 | 首屏 TTI、大列表首渲染 | `performance.now` | 7.5/7.6 验证 |
| 质量 | 越权用例通过率、契约字段差异数 | CI | Phase 5/6 出口 |
| 稳定 | 同步冲突率、失败重试成功率 | 服务端日志 | 5.4/5.7 验证 |

**执行纪律**：Phase 7 每项优化必须给出「基线 → 优化后」对照数据，未达标不关闭。

---

## 10. 风险与回滚

| 风险 | 影响 | 应对 |
|---|---|---|
| `currentUser` 重构面广（9 个 Service） | 编译/逻辑回归 | 分批替换 + 并发用例；保留 `isProjectAccessible` 行为不变 |
| 事务引入后长事务锁表（批量同步） | 同步阻塞 | 批量分片（每 200 行一批）+ 事务超时配置 |
| 历史表补列需迁移 | 升级失败 | 新增 `V3__*.sql`（幂等 `IF NOT EXISTS`），不改写 V1/V2 |
| 前端 Pinia 收编改动面大 | 功能回归 | 保留同名兼容包装器（沿用 Phase 2 经验，零调用点改动） |
| 性能基线缺失导致收益无法证明 | 无法验收 | Phase 6 必须先完成 6.4，否则 Phase 7 不启动 |

---

## 11. 附：证据索引

**Java**：`BaseService.java:35,38,63,68,102` · `AuthAspect.java:38-73` · `AuthService.java:142-152` · `EnvironmentService.java:47,57,142,152,218` · `CategoryService.java:29,107-139` · `SavedRequestService.java:26` · `SyncService.java:127,196,320-446,498` · `ProjectService.java:59-71` · `TeamService.java:262-289` · `db/migration/V1__init_schema.sql:89-98` · `db/migration/V2__sync_incremental.sql:28-48`

**Rust**：`db.rs:21-26,46,76,266-286,288,380,472-664,1148,1845-1876` · `sync.rs:14,103-113,143,222,344,431` · `infra.rs:13,39-48,75-83` · `commands/http.rs:12,46-56,63-83,87,100,167,200,242-299` · `commands/sync.rs:310,324,388-393` · `commands/project.rs:27-58,243,258` · `commands/team.rs:41,81,94` · `lib.rs:34-111`

**Vue**：`ApiDebuggerView.vue:994,1346-1349,1626,1870,2194` · `useSync.ts:3,5,9-10,285` · `useSavedRequests.ts:28,60,74,148,192,215,432` · `useEnvironments.ts:51-54,108,140,347,439,446` · `useTabs.ts:80-82,97` · `useScriptEngine.ts:116,151,171` · `types/index.ts:53-67,142,155,176,185,199,217` · `stores/dataMode.ts:43` · `repositories/*`（12 个） · `lib/tauri.ts:19`
