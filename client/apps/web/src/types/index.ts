// ====== HTTP 方法 & Body 类型 ======
export type Method = 'GET' | 'POST' | 'PUT' | 'DELETE' | 'PATCH' | 'HEAD' | 'OPTIONS'
export type BodyType = 'none' | 'json' | 'form' | 'text'

export const METHODS: Method[] = ['GET', 'POST', 'PUT', 'DELETE', 'PATCH', 'HEAD', 'OPTIONS']

// ====== 全局常量 ======
/** 默认同步服务端地址。多处引用的魔法字符串统一收敛到此处，避免散落硬编码。 */
export const DEFAULT_SERVER_URL = 'http://127.0.0.1:8092'
/** 离线模式下的默认团队 */
export const DEFAULT_TEAM_ID = 'default'

// ====== 服务端统一响应结构（与后端 ApiResult<T> / Rust ApiResult<T> 对齐：{success, code, msg, data}）======
export interface ApiResult<T> {
  success: boolean
  code: number
  msg: string
  data: T
}

// ====== 通用键值对 ======
export interface KV {
  key: string
  value: string
  enabled: boolean
}

export const emptyKV = (): KV => ({ key: '', value: '', enabled: true })

// ====== 后端发送 HTTP 请求的原始响应字段（位于统一响应的 data 内）======
export interface BackendResp {
  status: number
  statusText: string
  headers: [string, string][]
  body: string
  timeMs: number
  size: number
}

// ====== 前端响应信息 ======
export interface ResponseInfo {
  status: number
  statusText: string
  timeMs: number
  size: number
  headers: { key: string; value: string }[]
  headerCount: number
  body: string
  contentType: string
}

// ====== 历史记录条目 ======
export interface HistoryItem {
  id: string
  method: Method
  url: string
  params: KV[]
  headers: KV[]
  bodyType: BodyType
  body: string
  formBody: KV[]
  preScript: string
  postScript: string
  createTime: string
  status: number | null
  categoryId: string | null
  /** 所属项目（历史按项目隔离用，与 Rust `HistoryItem.projectId` 对齐） */
  projectId?: string | null
}

// ====== 工作模式 ======
export type WorkMode = 'debug' | 'preview' | 'design' | 'websocket'

// ====== Mock 响应（设计模式） ======
export interface MockResponse {
  id: string
  name: string
  statusCode: number
  statusText: string
  headers: string
  body: string
  delay: number
  isActive: boolean
}

export const MOCK_STATUS_OPTIONS = [
  { label: '200 OK', value: 200 },
  { label: '201 Created', value: 201 },
  { label: '204 No Content', value: 204 },
  { label: '301 Moved', value: 301 },
  { label: '304 Not Modified', value: 304 },
  { label: '400 Bad Request', value: 400 },
  { label: '401 Unauthorized', value: 401 },
  { label: '403 Forbidden', value: 403 },
  { label: '404 Not Found', value: 404 },
  { label: '405 Method Not Allowed', value: 405 },
  { label: '500 Internal Server Error', value: 500 },
  { label: '502 Bad Gateway', value: 502 },
  { label: '503 Service Unavailable', value: 503 },
]

// ====== 脚本日志条目 ======
export interface ScriptLogEntry {
  type: 'info' | 'warn' | 'error' | 'success'
  message: string
  time: number
}

// ====== 页签表单状态 ======
export interface TabFormState {
  method: Method
  url: string
  params: KV[]
  headers: KV[]
  bodyType: BodyType
  body: string
  formBody: KV[]
  categoryId: string | undefined
  preScript: string
  postScript: string
}

// ====== 页签完整状态 ======
export interface TabState {
  id: string
  title: string
  form: TabFormState
  reqTab: 'params' | 'headers' | 'body' | 'preScript' | 'postScript'
  respTab: 'body' | 'headers' | 'requestHeaders' | 'requestDetail' | 'scriptLog'
  respView: 'pretty' | 'raw'
  response: ResponseInfo | null
  requestHeaders: { key: string; value: string }[]
  error: string
  loading: boolean
  currentRequestId: string
  scriptLog: ScriptLogEntry[]
}

// ====== 项目（扁平，无多级嵌套）======
export interface Project {
  id: string
  name: string
  parentId: string | null
  /** 对应 Rust `sort_order`（Phase 6.2 #5：统一为 sortOrder，此前为 order，需手工映射） */
  sortOrder: number
  expanded: boolean
  /** 归属用户 ID（在线模式下为当前登录用户，用于用户级归属绑定；离线为空） */
  userId?: string
  /** 创建人（在线模式下为当前登录用户，离线为 'local'） */
  createBy?: string
  /** 最近更新人 */
  updateBy?: string
  /** 创建时间 */
  createTime?: string
  /** 最近更新时间 */
  updateTime?: string
  /** 当前登录用户在该项目中的角色：owner/admin/readwrite/readonly/inherit（成员可见时由后端返回） */
  currentUserRole?: 'owner' | 'admin' | 'readwrite' | 'readonly' | 'inherit'
}

export interface ProjectMemberInfo {
  id: string
  projectId: string
  memberType: 'user' | 'team'
  memberId: string
  memberName: string
  role: 'owner' | 'admin' | 'readwrite' | 'readonly' | 'inherit'
  createTime?: string
}

// ====== 分类（项目下可多级嵌套）======
export interface Category {
  id: string
  projectId: string | null
  name: string
  parentId: string | null
  /** 对应 Rust `sort_order`（Phase 6.2 #5：统一为 sortOrder） */
  sortOrder: number
  expanded: boolean
  /** 服务端对该数据的修改时间快照（pull 自 server 的 updateTime），用于编辑冲突检测 */
  serverUpdateTime?: string
  /** 服务端版本号（syncVersion）：服务端每次变更自增，用于 updateTime 相同时的并发判定 */
  syncVersion?: number
}

// ====== 接口（saved request）======
export interface SavedRequest {
  id: string
  projectId: string | null
  name: string
  method: Method
  url: string
  categoryId: string | null
  params: KV[]
  headers: KV[]
  bodyType: BodyType
  body: string
  formBody: KV[]
  preScript: string
  postScript: string
  sortOrder?: number
  createTime?: string
  createBy?: string
  updateTime?: string
  updateBy?: string
  /** 服务端对该数据的修改时间快照（pull 自 server 的 updateTime），用于编辑冲突检测 */
  serverUpdateTime?: string
  /** 服务端版本号（syncVersion）：服务端每次变更自增，用于 updateTime 相同时的并发判定 */
  syncVersion?: number
}

// ====== 环境变量相关 ======
export interface Environment {
  id: string
  projectId: string | null
  name: string
  groupId: string | null
  isActive: boolean
  createTime?: string
  createBy?: string
  updateTime?: string
  updateBy?: string
  /** 服务端对该数据的修改时间快照（pull 自 server 的 updateTime），用于编辑冲突检测 */
  serverUpdateTime?: string
  /** 服务端版本号（syncVersion）：服务端每次变更自增，用于 updateTime 相同时的并发判定 */
  syncVersion?: number
}

export interface EnvironmentGroup {
  id: string
  projectId: string | null
  name: string
  sortOrder?: number
  expanded?: boolean
  createTime?: string
  createBy?: string
  updateTime?: string
  updateBy?: string
  /** 服务端对该数据的修改时间快照（pull 自 server 的 updateTime），用于编辑冲突检测 */
  serverUpdateTime?: string
  /** 服务端版本号（syncVersion）：服务端每次变更自增，用于 updateTime 相同时的并发判定 */
  syncVersion?: number
}

export interface EnvironmentVariable {
  id: string
  environmentId: string
  key: string
  value: string
  enabled: boolean
  sortOrder?: number
  createTime?: string
  createBy?: string
  updateTime?: string
  updateBy?: string
  /** 服务端对该数据的修改时间快照（pull 自 server 的 updateTime），用于编辑冲突检测 */
  serverUpdateTime?: string
  /** 服务端版本号（syncVersion）：服务端每次变更自增，用于 updateTime 相同时的并发判定 */
  syncVersion?: number
}

// ====== 团队 / 成员 ======
// 说明（Phase 8.3）：以下「跨模块共享」的类型统一收敛到本文件，作为**单一事实来源**。
// 原先 `Team/MemberInfo/MemberRole` 定义在 `composables/useTeams.ts`、
// `UserInfo/LoginResponse/SyncConfig` 定义在 `composables/useSync.ts`，
// 而 `repositories/*` 又要 import 这些类型 → 形成 repository ↔ composable 循环依赖。
// 现由 composable 反向 re-export，调用点无需改动。

export type MemberRole = 'owner' | 'admin' | 'readwrite' | 'readonly'

export interface Team {
  id: string
  name: string
  description?: string
  ownerId?: string
  createTime?: string
  createBy?: string
  updateTime?: string
  updateBy?: string
}

export interface MemberInfo {
  id: string
  userId: string
  username: string
  role: MemberRole
  createTime?: string
}

// ====== 认证 / 会话 ======
export interface UserInfo {
  id: string
  username: string
  nickname: string
  email: string
}

export interface LoginResponse {
  token: string
  user: UserInfo
  expiresAt: number
  teamId?: string
}

/** 同步配置（camelCase，与 Rust `commands::sync::SyncConfig` 对齐） */
export interface SyncConfig {
  serverUrl: string
  teamId: string
  authToken?: string
}

// ====== 审计日志 ======
export interface AuditEntry {
  id: string
  projectId: string
  userId: string
  username: string
  action: string
  entityType: string
  entityId: string
  entityName: string
  beforeJson: string
  afterJson: string
  createTime: string
}

export interface AuditQuery {
  projectId?: string
  entityType?: string
  entityId?: string
  keyword?: string
  page?: number
  size?: number
}

// ====== Mock 服务 ======
/** Mock 路由规则。字段名为 snake_case，直接对应 Rust `commands::mock::MockRoute` 结构体。 */
export interface MockRoute {
  path: string
  method: string
  status: number
  body: string
  content_type: string
  headers?: [string, string][]
  delay_ms?: number
  sse?: boolean
}
