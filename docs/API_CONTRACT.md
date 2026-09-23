# 接口契约（API Contract）

> 单一事实来源。前端（Vue）、Rust（Tauri 命令）、Spring Boot（后端）三端必须共同遵守。
> 本文档对应架构优化路线图 **Phase 1（接口规范）**，与 `ARCHITECTURE_OPTIMIZATION.md` 配套。

---

## 1. 响应信封（统一）

三端统一使用以下结构（字段名、类型、顺序一致）：

```jsonc
{
  "success": true,        // boolean：业务是否成功
  "code": 0,              // int：错误码，0=成功，非 0 见 §3 错误码表
  "msg": "",              // string：可读提示，成功时为空
  "data": { }             // T：业务数据，失败时为 null
}
```

- Java：`com.canghai.api.common.ApiResult<T>`（`ok` / `fail` / `fail(ErrorCode)`）
- Rust：`crate::...::ApiResult<T>`（`ok` / `err`）
- 前端：`ApiResult<T>`（`client/apps/web/src/types/index.ts`）

> 历史别名 `ApiResponse` 已废除，全仓统一为 `ApiResult`。

---

## 2. 路径与版本管理

- 所有后端接口挂在 **`/api/v1`** 前缀下（v1 为当前版本，后续不兼容升级递增为 v2…）。
- 路径风格：`/api/v1/{资源}/{动作}`，动作为小写驼峰或 kebab-case，避免动词冗余。
- 请求方法统一 `POST`（现有契约：body 携带参数，由 `AuthAspect` 从 body/header 提取 token）。
- 前端经 Rust `call_server_api` 转发，仅传**相对路径**（如 `/api/v1/auth/login`），由 Rust 拼接到 `serverUrl`；Rust 侧强制 `path` 以 `/api/v1/` 开头。

### 当前端点清单（v1）

| 资源 | 路径 | 说明 |
|---|---|---|
| auth | `/api/v1/auth/login`、`/register`、`/logout`、`/me` | 认证 |
| team | `/api/v1/team/mine`、`/create`、`/update`、`/delete`、`/members`、`/invite`、`/changeRole`、`/removeMember`、`/transfer` | 团队 |
| project | `/api/v1/project/save`、`/delete` | 项目 |
| project-member | `/api/v1/project-member/{add,remove,list,by-team,visible}` | 项目成员 |
| category | `/api/v1/category/{list,create,update,delete}` | 分类 |
| request | `/api/v1/request/{list,detail,create,update,delete}` | 请求 |
| environment | `/api/v1/environment/{list,groups,save,update,delete,set-active,active,active-vars,...}` | 环境/变量 |
| sync | `/api/v1/sync/upload`、`/pull` | 云端同步 |
| audit | `/api/v1/audit/query` | 审计 |
| ping | `/api/v1/ping` | 探活（无需登录）|

---

## 3. 错误码表（统一枚举）

后端 `ErrorCode` 枚举（`com.canghai.api.common.ErrorCode`）为唯一权威定义，前端/Rust 维护同义常量。
约定：`0=成功`；`4xxxx=客户端/业务错误`；`5xxxx=服务端错误`。

| code | 名称 | 默认 message | 触发场景 |
|---|---|---|---|
| 0 | SUCCESS | 成功 | 一切正常 |
| 40000 | PARAM_INVALID | 参数无效 | 参数格式/取值非法 |
| 40001 | PARAM_MISSING | 缺少必填参数 | 必填字段为空 |
| 40100 | UNAUTHORIZED | 未登录或登录已过期 | 未携带/无法解析 token |
| 40101 | BAD_CREDENTIALS | 用户名或密码错误 | 登录凭证错误 |
| 40102 | TOKEN_EXPIRED | 登录已过期，请重新登录 | token 超过 exp |
| 40103 | FORBIDDEN | 无访问权限 | 角色/成员权限不足 |
| 40400 | NOT_FOUND | 请求的资源不存在 | 资源 ID 不存在 |
| 40900 | CONFLICT | 资源冲突（可能已存在） | 唯一键冲突（如用户名已注册）|
| 42200 | BUSINESS_ERROR | 业务处理失败 | 业务规则校验不通过 |
| 50000 | INTERNAL_ERROR | 服务器内部错误 | 未预期异常 |
| 50001 | DB_ERROR | 数据库操作失败 | 本地 SQLite / 远端 MySQL 执行失败（Rust `DbError` 映射） |
| 50002 | DOWNSTREAM_ERROR | 下游服务异常 | 转发后端 / 网络传输失败（Rust 侧网络错误映射） |

**使用约定**：
- 业务代码优先调用 `ApiResult.fail(ErrorCode)` 或 `ApiResult.fail(ErrorCode, detail)`；
  历史 `ApiResult.fail(int code, String msg)` 仍保留用于透传底层异常码，新代码应避免散用魔法数字。
- 前端收到 `success=false` 时，以 `msg` 直接提示用户；如需细分处理可读取 `code` 映射到 `ErrorCode` 同名常量。

---

## 4. 命名规范

- **JSON 字段**：camelCase（如 `teamId`、`serverUpdateTime`）。
- **数据库列**：snake_case（如 `team_id`、`server_update_time`），由 ORM 自动映射。
- **术语冻结**：团队 / 项目 / 环境 / 分类 / 请求 为固定术语，SQL 注释与代码注释保持一致（"团队"即 team，"工作区"旧称统一收敛为"团队"）。
- **时间字段**：见 §5。

---

## 5. 时间格式

- **存储基准：统一 Asia/Shanghai（北京时间，UTC+8，中国不使用夏令时、固定偏移）**。
  三端生成的时间字符串格式为 `YYYY-MM-DD HH:mm:ss`，例如北京时间 16:30 存为 `2026-09-16 16:30:00`：
  - Java：`BaseService.now()` → `LocalDateTime.now(ZoneId.of("Asia/Shanghai"))`；
    JVM 默认时区在 `CanghaiApiApplication` 启动时即钉死为 `Asia/Shanghai`；
  - Rust：`db::now_timestamp()` → `Utc::now().with_timezone(&FixedOffset::east(8h))`，
    **固定偏移而非 `chrono::Local`**，因此不随宿主机系统时区漂移；
  - 前端：`utils.now()` → 按 `APP_TZ_OFFSET_MS`(+8h) 偏移后取 UTC 字段，同样与本机时区无关。
- **MySQL 侧**：连接串用 `connectionTimeZone=Asia/Shanghai&forceConnectionTimeZoneToSession=true`
  把会话时区钉在 UTC+8，`server_update_time`（`CURRENT_TIMESTAMP(3)`）与增量同步游标
  `DATE_FORMAT(NOW(3), ...)` 因此出自同一时钟。**两者必须同源**：只要偏离一个时区，
  `server_update_time >= lastSyncTime` 就会恒不成立（增量恒为空）或恒成立（每次全量）。
- **展示约定**：前端禁止直接 `new Date('YYYY-MM-DD HH:mm:ss')`（会按本机时区解析导致偏移），
  必须走 `parseServerTime()`（按 `+08:00` 解析）与 `formatTime()`（按 UTC+8 渲染）。
- **切换历史**：早期基准为 UTC（Phase 6.3）；后按产品要求统一为北京时间。
  存量数据需执行一次平移脚本 `../client-spring-boot-server/src/main/resources/db/repair/shift_times_to_cst.sql`，
  做法与注意事项见该脚本头部说明。
- **后续（可选）**：如需对外集成，再评估迁移到带偏移量的 ISO 8601（`2026-09-16T16:30:00+08:00`）；
  当前格式与 DB 列类型、字符串排序完全兼容，暂不强制。

---

## 6. 认证

- 协议：无状态 **JWT（RS256 非对称）**。
- 私钥仅 Java 端持有（classpath `keys/private.pem`，gitignore，禁止提交）；
  公钥编译期嵌入 Rust（`client/apps/desktop/resources/public.pem`）与 Java 端自验（`keys/public.pem`）。
- Claims：`sub`（用户ID）、`username`、`team_id`、`iat`、`exp`。
- 前端登录后保存 token，调用 `call_server_api` 时通过 `token` 参数传入；
  Rust 验签后用 `userId/teamId` 注入请求，并转发 `Authorization: Bearer <token>` 与 body.token 双通道给 Java 端。
- 免登录白名单：`/api/v1/auth/**`、`/api/v1/ping`（见 `AuthProperties.whitelist`）。

---

## 7. 一致性校验清单（每次改接口后自查）

- [ ] 响应结构是否为 `ApiResult<T>`（无额外嵌套/字符串化 JSON）？
- [ ] 新接口是否落在 `/api/v1/...`？
- [ ] 失败是否使用 `ErrorCode` 枚举（而非散用魔法数字）？
- [ ] JSON 是否 camelCase、与 DB snake_case 可自动映射？
- [ ] 时间字段是否为 Asia/Shanghai 基准（`YYYY-MM-DD HH:mm:ss`），展示是否走 `parseServerTime()`？
- [ ] 参数校验是否加 `@Valid` + 约束注解（写接口）？
- [ ] Rust 转发路径与白名单是否同步更新？
- [ ] 是否执行 `node scripts/contract-check.mjs` 且无「未登记差异」？

---

## 8. 契约机器校验（Phase 6.1）

`scripts/contract-check.mjs` 对 **Java 枚举 / Rust struct / TS 接口**做自动比对，应纳入 CI：

```bash
node scripts/contract-check.mjs           # 严格模式：存在未登记差异即退出码 1
node scripts/contract-check.mjs --report  # 仅报告，不改退出码
```

校验项：
1. **实体字段集（TS ⊆ Rust）**——TS 不得声明 Rust 不存在的字段（否则运行时恒为 `undefined`）；
   Rust 独有的服务端内部列（`deleted`/`currentUserRole` 等）允许 TS 不声明。
2. **响应信封**：TS `ApiResult<T>` 必须含 `{ success, code, msg, data }`。
3. **同步上传键名**：Java `SyncData` 必须保留 `@JsonAlias("requests")`。
4. **错误码表**：Java `ErrorCode` 枚举与本文档 §3 表格一一对应。

> 脚本中 `KNOWN_TS_ONLY` 登记的为 Phase 6 遗留待修复项（`Project.order` / `Category.order` / `Environment.sortOrder`），
> 修复后须从白名单移除，使 CI 恢复严格拦截。
