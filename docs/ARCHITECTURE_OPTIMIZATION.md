# 沧海·API 调试工具 — 架构优化落地方案

> 适用范围：Vue 3 前端（`client/apps/web`）、Rust/Tauri 原生后端（`client/apps/desktop/src`）、Spring Boot 后端（`client-server`）。
> 目标：明确模块边界与职责、统一接口规范、提升通信一致性与效率、增强可扩展/可维护/可观测性、识别并重构瓶颈与重复逻辑。
> 状态：评审稿（基于代码探查证据，含文件/行号引用）。

---

## 0. 当前架构拓扑与协作关系

### 0.1 拓扑

```
┌─────────────────────────────────────────────────────────────┐
│  Tauri 桌面应用 (单进程)                                       │
│                                                               │
│  ┌──────────────┐   invoke    ┌──────────────────────────┐   │
│  │ Vue 3 (WebView)│──────────▶│ Rust 原生层 (Tauri commands) │   │
│  │ 全部业务/UI    │◀──────────│  · HTTP 代理(无 CORS)        │   │
│  └──────────────┘ 事件/返回值   │  · 本地 SQLite 离线存储     │   │
│                               │  · 云同步桥接(在线模式)     │   │
│                               └───────────┬──────────────┘   │
└───────────────────────────────────────────┬──────────────────┘
                                            │ HTTPS /api/*
                                            ▼
                              ┌─────────────────────────────┐
                              │ Spring Boot 后端 (MySQL)     │
                              │ 认证 / 团队 / 项目 / 同步 / 审计│
                              └─────────────────────────────┘
```

### 0.2 三层职责（现状）

- **Vue 层**：UI 与全部业务逻辑/状态，通过 Tauri `invoke` 调用 Rust 命令；在线模式经 `call_server_api` 由 Rust 转发 Java 后端（规避 WebView CORS）。无任何 `fetch/axios` 直连（已确认）。
- **Rust 层（集成枢纽）**：① 本地 HTTP 代理（`reqwest`，SSRF 防护完整）；② 本地 SQLite 离线存储（`canghai.db`，WAL）；③ 在线模式访问 Java 后端的代理桥（`call_server_api`）；④ 本地 Mock / WebSocket 服务。
- **Spring Boot 层**：唯一业务与数据权威——认证、团队/项目/成员/环境/分类/审计、全量数据同步合并。

### 0.3 最关键的协作断裂（阻塞性问题）

1. **JWT 算法不兼容（致命集成 bug）**
   Rust `call_server_api` 用编译期内置公钥做 **RS256** 验签（`jwt.rs`），旧 Aifei 服务端用 RS256（私钥 `resources/keys/private.pem` 签发，匹配）；但新 `client-server` 的 `JwtUtil` 改为 **HS256 对称**（`util/JwtUtil.java:26-27`）。若切到新 Spring Boot 后端，Rust 端对 token 验签会全部失败，在线模式不可用。

2. **部署配置未迁移**
   `Dockerfile` 第 5-6 行 `COPY server/pom.xml`、第 11 行打 `canghai-server-0.1.1.jar`（Aifei），与 `../client-spring-boot-server/pom.xml`（`com.canghai.api`）脱节；`docker-compose.yml` 的 `server` 服务、`nginx.conf` 的 `/api/` 反代也只认旧服务。当前仓库存在两套 Java 后端（`server/` 与 `../client-spring-boot-server/`），新的是未合并的待替换实现。

3. **同步契约已统一但未被充分利用**
   Rust `ApiResult<T>{success,code,msg,data}` 与 Java `Result{boolean success,int code,String msg,T data}` 信封已一致；`server_update_time` 列已在 5 张表维护，却未参与冲突判定，同步每次仍 `collect_all` 全量上传（性能瓶颈，见 §5.2）。

### 0.4 协作边界建议（降耦）

- **Rust 层** = 本地执行 + 集成桥：只负责发 HTTP、本地离线库、把受限请求转发给后端，**不做业务规则**（当前 `project.rs` 在线归属推导等应下沉后端）。
- **Spring Boot 层** = 唯一业务与数据权威。
- **Vue 层** = 纯 UI + 状态，通过 `repository` 抽象屏蔽「在线/离线」「invoke/转发」差异。

---

## 1. 统一接口规范（横切，三端共同遵守）

制定单一事实源 `API_CONTRACT.md`。

| 维度 | 现状问题 | 优化方案 |
|---|---|---|
| **响应信封** | 前端同时用 `ApiResponse`(useApi/useHttpRequest) 与 `ApiResult`(其余 composable) 两个别名；Java `Result`/Rust `ApiResult` 同构但命名不同 | 统一命名为 `ApiResult<T>`；前端删除 `ApiResponse` 别名；TS 类型作为 `shared/types` 单一来源 |
| **错误码** | Rust 本地错误统一 `-1`（无法区分类型）；Java 无枚举，散用 400/401/403/404/409/500；前端 `useAudit` 假设 `call_server_api` 返回**字符串**而其他假设**对象** | 定义三端共享错误码表（整数 code + i18n key）；前端 `invokeApi` 单一解包 |
| **命名风格** | JSON camelCase 已统一（好）；但 DB 列 snake_case，且 `EnvironmentVariable` 需特例 `var_key`+`@JsonProperty("key")`；`Team` 在代码称"团队"而 SQL 注释称"工作区" | 保留 camelCase↔snake_case 自动映射；术语表冻结（团队/项目/环境）；`var_key` 特例**评估后保留**（见 §1.3） |
| **版本管理** | 无版本前缀，路径直接 `/api/team` | 引入 `/api/v1/...` 前缀，便于后续不兼容升级 |
| **请求校验** | Java 0 处 `@Valid`；Rust 无类型校验；全靠 service 内手写 `if(null)` | Java 统一 `@Validated` + 全局 `MethodArgumentNotValidException`；Rust DTO 用 `validator` crate |
| **时间格式** | 前端 `now()` 输出 `YYYY-MM-DD HH:mm:ss`（三处复制），`useProjects` 用 ISO8601；Rust 手写 `now_timestamp` 有闰年/时区风险 | 全链路统一 **ISO8601 UTC（RFC3339）**；Rust 用 `chrono`，删除 `now_timestamp` |

### 1.1 推荐统一响应信封

```jsonc
// 三端一致
{ "success": true, "code": 0, "msg": "ok", "data": { } }
// 错误示例
{ "success": false, "code": 4001, "msg": "参数缺失: projectId", "data": null }
```

### 1.2 推荐错误码表（示例，需补全）

| code | 含义 | HTTP |
|---|---|---|
| 0 | 成功 | 200 |
| 4001 | 参数校验失败 | 400 |
| 4010 | 未登录/Token 失效 | 401 |
| 4030 | 无权限（越权） | 403 |
| 4040 | 资源不存在 | 404 |
| 4090 | 资源冲突（如已是成员） | 409 |
| 5000 | 服务器内部错误 | 500 |
| 5001 | 数据库错误 | 500 |
| 5002 | 下游服务/同步失败 | 502 |

### 1.3 命名决策：保留 `var_key` 列名（Phase 3 实施结论）

评审建议「删除 `var_key` 特例、统一列名 `key`」，实施时**推翻**该建议，理由：

- `KEY` 是 MySQL 保留字，用作列名后所有 SQL（含 MyBatis-Plus `QueryWrapper` 生成的列、`SyncService`/`EnvironmentService` 中手写 `set("var_key", ...)`）都必须加反引号，漏一处即语法错误；
- 收益仅为「列名与 JSON 字段名一致」，而该一致性已由 `@JsonProperty("key")` 达成，对前端/客户端完全透明。

统一口径：数据库列 `var_key`（snake_case 列、camelCase 属性 `key`）→ JSON `key`，并在 Flyway `V1__init_schema.sql` 与 `entity/EnvironmentVariable.java` 注释中固化说明。

---

## 2. 前端优化方案（`client/apps/web/src`）

核心问题：无集中状态管理、类型重复、在线/离线双路径散落、巨型组件、脚本沙箱不安全。

### 2.1 引入集中状态层（Pinia）
当前用 composable 顶层 `ref` 手工实现「穷人版 Pinia」，且 `useEnvironments.ts:527`、`useSavedRequests.ts:475` 在 **import 时即触发 `load()` 发 RPC**、`useEnvironments.ts:132,137`/`useSavedRequests.ts:94,99`/`useHistory.ts:85` 多处顶层 `watch`——隐式全局副作用，难测试。
- 引入 Pinia，将 `authToken/currentUser/dataMode/projects/environments/tabs/settings` 收归 store；
- 移除模块加载副作用，改为显式 `store.init()`。

### 2.2 消除重复类型
- `KV` 被定义 3 次（`types/index.ts:22`、`useSavedRequests.ts:25`、`KeyValueEditor.vue:34`）→ 统一 `import`；
- `EnvironmentVariable`(types) vs `EnvVariable`(useEnvironments.ts:46) → 删除后者；
- `ApiResponse`/`ApiResult` → 仅留 `ApiResult`；
- `uid()`（`utils/index.ts:4` 与 `useEnvironments.ts:66`/`useSavedRequests.ts:71`/`useCategories.ts:19`）/`now()`（`useEnvironments.ts:521`/`useHistory.ts:8`/`useSavedRequests.ts:75`）重复 → 收归 `utils/index.ts`。

### 2.3 抽象「数据访问仓库」层消除在线/离线双路径
每个 composable 都在 `isOnline()` 分支里先调 `useServerApi` 再缓存本地（`useEnvironments.ts:155-234` 等）。建议抽 `repository` 层：

```ts
// composables/repositories/projectRepo.ts
async function saveProject(p: Project) {
  return dataMode.isOnline.value
    ? serverApi.post('/api/v1/project/save', p)                  // 在线：Rust 转发 Java
    : invokeApi('save_project', { project: p, dataMode: 'offline' }) // 离线：本地 SQLite
}
```

composable 只调 repo，不再各自写分支——消除「在线 create 仅返回精简对象须用原 req 缓存」的散落补丁（`M9`）。

### 2.4 拆分巨型组件
- `ApiDebuggerView.vue` ~1960 行、`EnvironmentManager.vue` 27KB、`AppLayout.vue` 21KB，按「请求面板/响应面板/脚本面板/环境选择」拆子组件，状态下沉 store。

### 2.5 脚本引擎安全（高危）
`useScriptEngine.ts:136` 用 `new AsyncFunction` + **静态标识符黑名单**（`DANGEROUS_IDENTIFIERS`）。黑名单可被 `this`/`[...]`/编码绕过，**非真正隔离**。
- 短期：至少放到 **Web Worker** 中执行，切断对 DOM/`__TAURI__` 的同步访问；
- 长期：改用 **QuickJS-WASM** 或隔离 VM 上下文做真正沙箱，`env`/CryptoJS 以白名单注入。

### 2.6 可观测性与健壮性
- 统一错误：删除 `invokeSafe`/`invokeApi`/`invokeUnwrap` 三种风格，仅留 `invokeApi` 抛统一 `ApiError`；
- 消除 `await invoke().catch(()=>{})` 静默吞错（`useEnvironments/useSavedRequests/useHistory` 多处）；
- 移除生产 `console.log` 残留（如 `useProjects.ts:126`）；
- 接入轻量前端日志（埋点请求耗时/同步结果）。

---

## 3. Rust 服务优化方案（`client/apps/desktop/src`）

核心问题：三模块复制通信胶水、错误模型形同虚设、并发隐患、迁移无版本、JWT 与后端不匹配。

### 3.1 收敛通信胶水（最高优先级重构）
`api_post`/`load_config`/`get_server_url_and_token` 在 `project.rs`/`project_member.rs`/`team.rs` **完整复制各 50-100 行**，且 `infra.rs` 已提供 `http_client()`/`post_api_result()`/`load_config()` 却未被复用；同时存在「通用代理 `call_server_api`」与「各模块硬编码 path」两套路径。
- 删除三处副本，所有在线调用改走 `infra::post_api_result` / `call_server_api`；
- `save_project` 在线模式（`project.rs:203`）也应走 `call_server_api`，统一路径。

### 3.2 落地错误模型
`DbError` 枚举（`db.rs:46`）被 `.to_string()` 降级为 `-1`——错误码全失。
- 将 `DbError` 映射到整数业务码（用 `thiserror`）；统一返回包裹为 **单层 `ApiResult<T>`**（当前 `Result<ApiResult<T>,String>` 与 `ApiResult<T>` 混用）。

### 3.3 并发与资源修正（正确性 bug）
- `WsManager` 的 `rand_id()`（`ws.rs:188`）仅以 `now_ms()` 为 `DefaultHasher` 种子 → **同毫秒并发 session_id 碰撞**，改用 UUID；
- `MockServerState`（`mock.rs:23`）：`thread::spawn` 在 async 命令内跑阻塞 `tiny_http`，`stop` 仅置 flag 且循环间才检查、不 join → 无请求时无法退出。改为 `tokio::sync::oneshot` 通知退出 + 线程 join，或用异步 `hyper`；
- `AppDb { Mutex<Connection> }` 单连接串行化全部本地 DB（桌面场景可接受），评估 `r2d2`/`deadpool` 连接池以支撑同步高并发。

### 3.4 迁移治理与时间
- 手工 `ALTER TABLE ADD COLUMN` 演进（`db.rs:666`）无版本号、无回滚 → 引入 `schema_version` 表 + 有序迁移；
- 删除手写 `now_timestamp`（`db.rs:380`，闰年/时区风险），改用 `chrono::Utc::now()`。

### 3.5 清理与语义
- 删除空文件 `commands/workspace.rs`（未注册、0 字节）；
- 澄清 `server_update_time` 语义：目前写入但不参与冲突判定（`sync.rs:189` 只看实体 `update_time`）→ 要么用于增量同步游标（推荐，见 §5.2），要么删除。

### 3.6 🔴 JWT 必须与新后端对齐
见 §0.3-①。推荐：**让新 Spring Boot 用 RS256 并复用同一 `private.pem`**（Rust 公钥验签无需改），或统一改为 HS256 并把密钥抽成运行时配置（双端共享）。这是切到新后端的**前置阻塞项**。

---

## 4. Spring Boot 服务优化方案（`client-server`）

核心问题：旧 Aifei Dispatcher 死代码、脚手架配置残留、字符串拼接 SQL、三重异常捕获、硬编码密钥、DDL 漂移。

### 4.1 删除死代码与配置残留
- `BaseService.handle()` 路由分发器（`BaseService.java:34` 及所有 service 的 `switch`）全仓 0 命中——**旧 Aifei 单 Dispatcher 遗留死代码**，删除 `handle/getAction/pathSegment`；
- `application.yaml` 大量 ruoyi/yudao 模板配置（nacos/xxl-job/knife4j/spring-boot-admin/easy-trans/lock4j/`yudao.info.base-package`）均**未在 `pom.xml` 引入** → 全部删除，并补上真正需要的 `springdoc-openapi`（Swagger UI 当前打不开）→ ✅ **已完成（Phase 3）**：引入 `springdoc-openapi-starter-webmvc-ui:3.1.1`（适配 Spring Boot 4），`config/OpenApiConfig.java` 定义 info + HTTP Bearer(JWT) 安全方案，`springdoc.paths-to-match=/api/**` 只暴露业务接口；`/v3/api-docs`、`/swagger-ui.html` 已加入鉴权白名单；同时引入 `spring-boot-starter-actuator` 并仅暴露 `health`/`info`；
- `BaseController` 注入未使用的 `AuthService`（`BaseController.java:18`）。

### 4.2 同步性能（瓶颈）
当前 `SyncController` 走全量 `SyncData`，Rust 端 `collect_all` 每次全量上传。已有 `server_update_time` 列却未用。
- 改为**增量同步**：`pull` 带 `lastSyncTime`，服务端按 `server_update_time > ?` 过滤返回；Rust 端用 `server_update_time` 作游标——把 O(全量) 降为 O(增量)，是最大性能收益点。

### 4.3 安全与校验
- **SQL 注入风格**：`TeamService.java:63-64`、`ProjectMemberService.java` 字符串拼接 `inSql("...user_id='"+userId+"'")`、`AuditService.java:95` `.last("LIMIT "+size)` → 改 MyBatis-Plus `QueryWrapper`/`Page` 参数化；
- **异常信息泄露**：`GlobalExceptionHandler.java:24` 直接回吐 `e.getMessage()`（含 SQL/栈）→ 仅返回错误码+用户友好文案，明细写日志；删除散落 `e.printStackTrace()`；
- **三重异常捕获**：`AuthAspect` + `BaseController.invoke` + `GlobalExceptionHandler` 都 catch → 改为**仅 `@RestControllerAdvice` 一处**；`AuthAspect` 只做鉴权不处理异常；
- **硬编码密钥**：`application-local.yaml` 的 JWT 密钥 `change-me-canghai-2026`、DB 密码 `abc@1234` 提交进仓库 → 改环境变量/配置中心，git 清理历史；
- **CORS 过宽**：`WebConfig.java:16` `allowedOriginPatterns("*")`+`allowCredentials(true)` 生产收紧为具体域名。

### 4.4 其他
- 引入 **Bean Validation**（`@Valid`）+ 统一错误码枚举，替代散落 `Result.fail(400,...)`；
- **循环依赖容忍**（`application.yaml:9 allow-circular-references:true`）→ 重构 `BaseService` 注入 `AuditService/ProjectMemberService` 紧耦合（事件/依赖倒置解耦）；
- **审计 before 快照缺失**：`BaseService.audit(...)`（`BaseService.java:128`）`beforeJson` 恒为 null → 补全变更前快照；
- **DDL 漂移治理**：`schema.sql`(LONGTEXT+`var_key`) 与 `db.sql`(JSON+`` `key` ``) 不一致 → 统一到一份，引入 **Flyway/Liquibase** 做版本化迁移（替代注释掉的 `sql.init`）→ ✅ **已完成（Phase 3）**：两份脚本合并为唯一事实源 `src/main/resources/db/migration/V1__init_schema.sql`（列统一 `var_key`、JSON 类列统一 `LONGTEXT`、术语统一「团队」、密码算法注释统一 PBKDF2），删除 `schema.sql`/`db.sql`；`spring-boot-flyway` + `flyway-core`/`flyway-mysql` 接管迁移，存量库 `baseline-on-migrate`（baseline version 1）不重建表，空库执行 V1 建表（已端到端验证）；
- **`data_mode` 双写**（`online/offline` 各存一份，处处 `eq("data_mode",...)`）→ 评估简化为「离线优先 + 同步合并」单副本，降低复杂度；
- 清理 `server/` 旧模块、更新 README（仍写"Aifei 服务端"）、统一术语。

---

## 5. 三者协作关系与一致性总览

| 关注点 | 规范（建议） | 负责方 |
|---|---|---|
| 响应信封 | `ApiResult{success,code,msg,data}` | 三端复用同结构（Rust 已定义，Java/TS 对齐）|
| 认证 | **统一 JWT 算法**（RS256 推荐，复用现有公私钥对）| Rust(验签)+Spring Boot(签发) |
| 同步契约 | `SyncData`/`SyncResponse` 为唯一同步 API，**增量**按 `server_update_time` | Rust(编排)+Spring Boot(存取) |
| 错误码 | 共享错误码枚举表 | 三端 |
| 术语/命名 | camelCase JSON + snake_case 列 + 冻结术语表 | 三端 |
| 版本 | `/api/v1` 前缀 | Spring Boot(路由)+Rust(转发 path)+前端(path) |
| 部署 | Docker 指向 `client-server`，弃用 `server/` | 运维/CI |

### 5.1 关键链路示例（发送一次请求）
`ApiDebuggerView.sendRequest()` → 前置脚本(`useScriptEngine`) → `{{var}}` 解析(`useEnvironments.resolveVariables`，含 `stripCrlf` 防 Header 注入) → `buildUrl/buildBody`(`useHttpRequest`) → `send_http_request`/`send_http_request_stream`(Rust 实发 HTTP，SSRF 防护) → 后置脚本(`res.json()`) → `save_history`(Rust 写本地) → 写 `TabState.response`。

### 5.2 增量同步建议时序
```
Rust(run_sync) ──▶ POST /api/v1/sync/upload {entities, lastUpdateTime}
Spring Boot     ──▶ 按 server_update_time 合并，返回 {entities, serverTime}
Rust            ──▶ 以 server_update_time 为游标写入本地，记录 lastSyncTime
Rust(run_pull)  ──▶ POST /api/v1/sync/pull {teamId, lastSyncTime}
Spring Boot     ──▶ SELECT ... WHERE server_update_time > lastSyncTime
```
冲突判定：保留现有「最后写入者胜」（`update_time` 字典序比较），并评估将 `version` 字段纳入判定。

---

## 6. 重构路线图（分阶段，降低风险）

- **Phase 0（已完成 ✅）**：统一 JWT 算法为 RS256（复用 `private.pem`，Rust 端公钥验签）；`Dockerfile`/`docker-compose.yml`/`nginx.conf` 已切到 `client-server`；密钥对三处一致（Rust 嵌入 / Spring Boot 签发自验）。端到端登录→同步需重建桌面端（公钥编译期嵌入）后跑通。
- **Phase 1（已完成 ✅）**：三端 `/api/v1` 前缀统一；Java 新增 `ErrorCode` 枚举 + `ApiResult.fail(ErrorCode)` 重载（认证路径已切换为例）；响应类型名统一为 `ApiResult`（Java `Result`→`ApiResult`、前端 `ApiResponse`→`ApiResult`）；`docs/API_CONTRACT.md` 落地。ISO8601 仅定规范，全量迁移见 Phase 4。
- **Phase 2（已完成 ✅）**：
  - ✅ Rust：`team`/`project`/`project_member` 的 `config_path`/`load_config`/`get_server_url_and_token`/`api_post`/`api_get` 全部收敛到 `infra`（新增 `get_server_url_and_token`/`get_api_result`），`cargo check` 通过。
  - ✅ Spring Boot：删除死代码（`BaseService.handle`/`getAction`/`pathSegment`/`inClause` + 9 个 service 的 `handle` 分发）；删除脚手架残留配置（nacos/knife4j/xxl-job/easy-trans/mybatis-plus-join/redis/actuator/lock4j/admin/未定义的 `yudao.info.base-package`、硬编码 `encryptor.password`）；移除 `BaseController` 未使用的 `AuthService` 注入。
  - ✅ 前端类型/工具归一：`KV` 统一到 `types`；`uid`/`now` 统一到 `utils`。
  - ✅ 前端引入 Pinia：新增 `stores/{pinia,dataMode,settings,conflict}`；`useDataMode`/`useSettings`/`useConflict` 改为同名兼容包装器（零调用点改动，`vue-tsc` + `vite build` 通过）。
  - ✅ 前端 repository 层（完整覆盖）：`repositories/` 共 12 个 —— 资源型 category/request/environment/history/project/projectMember/team/audit + 服务型 sync/http/mock/ws；新增共享 `lib/tauri.ts::invokeUnwrap`（消除 useSync/settings 的重复实现）。除底层封装 `useApi.ts` 外，所有 composable 的 `invoke`/`serverApi` 访问均收敛到 repo；composable 只保留编排（乐观更新、刷新、冲突弹窗），`vue-tsc` + `vite build` 通过。
  - ✅ 修复 Phase 1 回归：`/api/`→`/api/v1/` 批量替换曾误将 `@tauri-apps/api/core` 改成 `@tauri-apps/api/v1/core`（`useSync`/`useAudit`），已修正并通过类型检查。
  - ✅ `EnvVariable`↔`EnvironmentVariable` 归一：统一到 `types.EnvironmentVariable`（`sortOrder?`），删除 `useEnvironments` 内的重复定义与 `EnvVariable` 别名，`environmentRepo`/`EnvironmentManager.vue` 改用统一类型。
  - ✅ Spring Boot 循环依赖解耦：`BaseService` 对 `AuditService`/`ProjectMemberService` 改用 `@Lazy` 注入打断循环，移除 `spring.main.allow-circular-references`。
- **Phase 3（已完成 ✅）**：
  - ✅ Rust：WS 会话 id 改用「原子计数器+纳秒」消除同毫秒碰撞；Mock 服务改为主线程同步绑定端口 + `recv_timeout` 轮询可退出 + `stop` join 释放端口（`cargo check` 通过）。
  - ✅ Spring Boot：`TeamService`/`ProjectMemberService` 的子查询 `inSql` 字符串拼接 → `apply("… {0} …", val)` 参数化；`AuditService` 分页页号钳制为非负。
  - ✅ Spring Boot：异常收敛到唯一 `GlobalExceptionHandler`（移除 `BaseController`/`AuthController`/`AuditController`/`AuthAspect` 的重复 try-catch；handler 记日志、不再回吐内部异常信息）。
  - ✅ 密钥外置：`application-local.yaml` 的 DB 密码移除明文默认值（仅留 `${DB_PASSWORD:}`）。
  - ✅ 前端脚本沙箱升级：`useScriptEngine` 由主线程 `new AsyncFunction`+静态黑名单，改为 **Web Worker 真隔离**（新增 `workers/scriptWorker.ts`；Worker 无 `window`/`document`/`__TAURI__`，脚本无法访问 DOM 或调用 Tauri 命令；静态黑名单保留为第一道防线；`req` 修改经结构化克隆回填原对象、`res.json()` 在 Worker 内重建、15s 超时终止并重建 Worker；Tauri CSP 补 `worker-src 'self' blob:`）。`vue-tsc`+`vite build` 通过，worker 独立 chunk 产出。
  - ✅ Spring Boot **Flyway 版本化迁移**：新增 `spring-boot-flyway` + `flyway-core`/`flyway-mysql`（版本由父 POM 管理，实为 Flyway 11.14.1）；`db/migration/V1__init_schema.sql` 成为 DDL 唯一事实源，删除漂移的 `schema.sql`/`db.sql`（统一 `var_key`/`LONGTEXT`/术语/注释）；`application-local.yaml` 配置 `baseline-on-migrate=true`、`baseline-version=1`、`validate-on-migrate` 可经 `FLYWAY_VALIDATE` 覆盖。验证：空库执行 V1 建成 12 张表 + `flyway_schema_history`(v1)；存量库首次启动自动 baseline 到 v1 且提示 "up to date"。
  - ✅ **Swagger / Actuator**：`springdoc-openapi-starter-webmvc-ui:3.1.1`（其 parent 为 Spring Boot 4.0.0，与 SB 4.0.8 兼容）+ `spring-boot-starter-actuator`；`config/OpenApiConfig.java`（info + HTTP Bearer/JWT，声明支持 `X-Token`/`body.token`/`query.token`）；`springdoc.paths-to-match=/api/**` 屏蔽运维端点；`management.endpoints.web.exposure.include=health,info`；`canghai.auth.whitelist` 追加 `/v3/api-docs/**`、`/swagger-ui/**`、`/actuator/health/**`、`/actuator/info`。验证：`/swagger-ui.html` 200、`/v3/api-docs` 200 且仅含 `/api/**` 路径、`/actuator/health` 返回 `UP`。
  - ✅ 顺带修复：`dto/req/LoginRequest.java` 缺失 `import lombok.Data`（依赖变更触发全量重编译后暴露），已补齐，`mvn package` 通过。
- **Phase 4（性能/扩展，已完成 ✅）**：
  - 迁移规范：所有 DDL 变更一律新增 `V{n}__{描述}.sql`，**禁止修改已提交的迁移脚本**（生产建议 `FLYWAY_VALIDATE=true`）。
  - ✅ **增量同步（下发）**：`V2__sync_incremental.sql` 为 5 张同步表加 `server_update_time DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3)`（**由 MySQL 自动维护**，业务 CRUD / 同步合并零代码覆盖）+ `sync_version INT`（BEFORE UPDATE 触发器自增）+ 时间列索引；存量数据按 `update_time` 回填。
  - ✅ **增量同步（上传）**：Rust 侧引入 `dirty` 标记（本地写入置 1、服务端数据合并置 0、上传成功后整表清零），`collect_all(..., only_dirty=true)` 只上报变更行；存量行默认 dirty=1，故首次仍是全量、之后纯增量。
  - ✅ **同步契约扩展**：`SyncData.lastSyncTime`（客户端游标）、`SyncResponse.serverTime/incremental/deletedIds`（游标、是否增量、墓碑）。服务端按 `server_update_time >= lastSyncTime` 过滤（用 `>=` 保证不漏，客户端合并幂等）；墓碑解决"增量感知不到删除"。
  - ✅ **游标时区一致**（关键坑）：`server_update_time` 由 MySQL 按其会话时区（Docker 镜像常为 UTC）写入，而 JVM 默认 UTC+8；游标必须取 `SELECT UTC_TIMESTAMP(3)`，否则游标超前 8 小时、增量恒为空。另加"游标超前则退化为全量"的保护，兼容升级前用本地时钟生成的旧游标。
  - ✅ **冲突合并引入版本号**：字段命名为 `sync_version`（**不叫 `version`**，因 `ch_saved_requests.version` 已被本地"请求快照版本"占用）。服务端合并判定：先比 `update_time`，同一秒再比 `sync_version`；前端 `isServerNewer(updateTime, localUpdateTime, syncVersion, localSyncVersion)` 同样做版本兜底。
  - ✅ **SQLite 并发评估（结论：暂不引入连接池）**：`DbConn = Mutex<Connection>` 单连接串行在桌面单用户场景下不是瓶颈，真正痛点是一次合并 N 行 × 隐式事务；已改为 `db::in_transaction()`（SAVEPOINT）把整批合并放进单事务（已在 WAL 模式下）。引入 r2d2 需改写 60+ 处 `lock_db` 调用点，收益不抵风险；若未来出现"同步阻塞 UI"的实测问题再评估。
  - ✅ 验证：`mvn package` 通过；本地库 V2 迁移成功（5 表均含 `server_update_time`+`sync_version`，5 个触发器就位）；端到端验证——全量拉取 `incremental=false` 并下发游标 → 创建分类 → 按游标拉取 `incremental=true` 且只带回该分类（`syncVersion=1`）→ 无变更时返回 0 条 → 删除后返回 `deletedIds` 墓碑；Rust `cargo check` 通过；前端 `vue-tsc --noEmit` 通过。
  - ⏳ 遗留（不在本期范围）：`now_timestamp()` 仍为手写时间换算（闰年/时区风险，见 §3.4）；全链路 ISO8601 UTC 统一（§1）需在时间格式整体迁移时一并处理。

---

## 附：探查证据索引

- 前端：`types/index.ts`、`useApi.ts`、`useServerApi.ts`、`useEnvironments.ts`、`useSavedRequests.ts`、`useScriptEngine.ts`、`ApiDebuggerView.vue`、`useWorkspaces.ts`(空桩)。
- Rust：`lib.rs`(65 命令注册)、`commands/{project,project_member,team,sync,ws,mock}.rs`、`infra.rs`、`db.rs`(schema+迁移+dirty/version+事务)、`jwt.rs`(RS256)、`sync.rs`(增量收集/合并/游标/墓碑)。
- Spring Boot：`CanghaiApiApplication.java`、`common/ApiResult.java`、`common/ErrorCode.java`、`common/ApiException.java`、`aspect/AuthAspect.java`、`controller/*`、`service/BaseService.java`、`service/*Service.java`、`util/JwtUtil.java`(RS256)、`config/WebConfig.java`、`config/OpenApiConfig.java`(OpenAPI)、`db/migration/V1__init_schema.sql`(DDL 唯一事实源)、`db/migration/V2__sync_incremental.sql`(增量同步游标/版本号)、`model/SyncData|SyncResponse`(增量契约)、`service/SyncService`(合并/游标/墓碑)、`application.yaml`(springdoc/actuator)。
- 部署：`Dockerfile`(指向 `client-server`)、`docker-compose.yml`、`nginx.conf`、`README.md`(已更新为 Spring Boot)。
