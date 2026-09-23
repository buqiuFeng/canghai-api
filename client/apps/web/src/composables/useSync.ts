import CryptoJS from 'crypto-js'
import { teams, loadTeams, loadCurrentTeamMembers } from './useTeams'
import { DEFAULT_SERVER_URL } from '@/types'
import type { LoginResponse, SyncConfig, UserInfo } from '@/types'
import { ApiError, ErrorCodes, normalizeError } from '@/lib/tauri'
import { useDataMode } from './useDataMode'
import { useSyncRepo } from '@/repositories/syncRepo'
// 会话共享态下沉到叶模块 useSession（Phase 8.3：打断 useSync ↔ useApi / useTeams 循环），
// 此处按原名 re-export，既有调用点零改动。
import {
  syncStatus,
  syncMessage,
  syncOnline,
  currentTeamId,
  authToken,
  syncServerUrl,
  syncConfigLoaded,
  currentUser,
  isLoggedIn,
} from './useSession'

export {
  syncStatus,
  syncMessage,
  syncOnline,
  currentTeamId,
  authToken,
  syncServerUrl,
  syncConfigLoaded,
  currentUser,
  isLoggedIn,
}
// 跨端共享类型统一由 @/types 提供（Phase 6.2 #4 消除双份定义），此处 re-export 兼容旧路径。
export type { LoginResponse, SyncConfig, UserInfo }

const { dataMode } = useDataMode()
const repo = useSyncRepo()

let autoSyncTimer: ReturnType<typeof setInterval> | null = null

const AUTH_KEY = 'canghai-auth'

/**
 * 当前用户的本地快照键（仅非敏感的展示字段：id/username/nickname/email）。
 *
 * 背景：Tauri 的 sync_config 只保存 token / serverUrl / teamId，**不含用户信息**，
 * 刷新后必须靠 `/api/v1/auth/me` 回捞才能填上 `currentUser`。而 `isLoggedIn`
 * 要求 token 与 user 同时非空，因此只要该请求「未发起」（离线模式）或「失败」
 * （服务端未启动 / 网络抖动 / 验签异常），刷新后就表现为「登录状态丢失」。
 * 这里缓存一份快照，作为登录态的本地兜底依据。
 */
const USER_KEY = 'canghai-current-user'

/** 写入 / 清除当前用户快照。 */
function persistUser(user: UserInfo | null): void {
  try {
    if (user) localStorage.setItem(USER_KEY, JSON.stringify(user))
    else localStorage.removeItem(USER_KEY)
  } catch { /* localStorage 不可用（如隐私模式）时忽略 */ }
}

/** 读取当前用户快照；无快照或结构非法时返回 null。 */
function restoreUser(): UserInfo | null {
  try {
    const raw = localStorage.getItem(USER_KEY)
    if (!raw) return null
    const u = JSON.parse(raw)
    return u && u.id ? (u as UserInfo) : null
  } catch {
    return null
  }
}

/**
 * 是否为「凭证失效」类错误（应清除登录态）。
 *
 * 只有明确的鉴权失败（401 / 40100 未授权 / 40101 凭证错误 / 40102 令牌过期 /
 * 40103 无权限）才登出；服务端未启动、超时、5xx 等瞬时故障不应把用户踢下线。
 */
function isAuthError(e: unknown): boolean {
  if (e instanceof ApiError) {
    return (
      e.code === 401 ||
      e.code === ErrorCodes.UNAUTHORIZED ||
      e.code === ErrorCodes.BAD_CREDENTIALS ||
      e.code === ErrorCodes.TOKEN_EXPIRED ||
      e.code === ErrorCodes.FORBIDDEN
    )
  }
  // 非信封错误（裸字符串）：按文案兜底判断
  return /验签失败|请先登录|令牌过期|token.*过期/i.test(normalizeError(e))
}

/// 运行环境是否为 Tauri（具备原生持久化能力）。
/// 是 → 以 Tauri 同步配置为认证权威源（跨 WebView 共享，且不与 localStorage 双写）；
/// 否（纯 WebView/浏览器调试） → 降级到 localStorage 作为唯一持久化。
const isTauri =
  typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

/**
 * Web 降级模式下本地加密密钥（派生自固定盐 + 运行环境指纹）。
 * 说明：Web 存储无法达到 Tauri secureStorage 级别的隔离，此加密仅用于避免 token
 * 以明文暴露在 localStorage（对应清单 M5），降低同源 XSS 直接窃取凭证的风险。
 * 生产环境应始终运行于 Tauri 模式。
 */
const WEB_ENC_KEY = (() => {
  const salt = 'canghai-web-enc-v1'
  const fingerprint = (navigator.userAgent || 'unknown') + (location.hostname || 'localhost')
  return CryptoJS.SHA256(salt + fingerprint).toString()
})()

function encryptWeb(value: string): string {
  return CryptoJS.AES.encrypt(value, WEB_ENC_KEY).toString()
}
function decryptWeb(value: string): string {
  try {
    return CryptoJS.AES.decrypt(value, WEB_ENC_KEY).toString(CryptoJS.enc.Utf8)
  } catch {
    return ''
  }
}

/**
 * 持久化认证状态。
 * - Tauri 环境：以原生 sync_config 为权威持久化（跨 WebView 共享）。登录成功后必须落盘，
 *   否则刷新页面后 initAuth 从空配置恢复，会丢失登录态（表现为刷新即未登录）。
 * - 纯 WebView/浏览器调试：降级写 localStorage（对 token 做 AES 加密，M5）。
 */
function persistAuth(serverUrl: string = DEFAULT_SERVER_URL): void {
  if (isTauri) {
    // Tauri 配置不保存用户信息，仅保存 token + serverUrl + teamId，
    // 用户信息在 initAuth 时通过 /api/v1/auth/me 重新校验获取。
    const config: SyncConfig = {
      serverUrl: serverUrl || DEFAULT_SERVER_URL,
      authToken: authToken.value,
      teamId: currentTeamId.value || 'default',
    }
    setSyncConfig(config).catch(() => { /* 忽略持久化失败，内存态仍然有效 */ })
    return
  }
  if (authToken.value && currentUser.value) {
    const payload = encryptWeb(JSON.stringify({
      token: authToken.value,
      user: currentUser.value,
    }))
    localStorage.setItem(AUTH_KEY, payload)
  } else {
    localStorage.removeItem(AUTH_KEY)
  }
}

/** 从 localStorage 恢复认证状态（仅无 Tauri 的降级环境调用）。Web 模式先解密（M5）。 */
function restoreAuth(): boolean {
  if (isTauri) return false
  try {
    const raw = localStorage.getItem(AUTH_KEY)
    if (raw) {
      const decrypted = decryptWeb(raw)
      if (!decrypted) return false
      const saved = JSON.parse(decrypted)
      if (saved.token && saved.user) {
        authToken.value = saved.token
        currentUser.value = saved.user
        return true
      }
    }
  } catch { /* ignore */ }
  return false
}

/** 获取同步配置 */
export async function getSyncConfig(): Promise<SyncConfig> {
  try {
    const config = await repo.getConfig()
    // 同步 token 到内存
    if (config.authToken) {
      authToken.value = config.authToken
    }
    // 发布持久化的 serverUrl 快照：settings store 与 AppLayout 顶栏可直接复用，
    // 避免刷新页面时对同一条 `get_sync_config` 命令重复发起 IPC。
    if (config.serverUrl) {
      syncServerUrl.value = config.serverUrl
    }
    syncConfigLoaded.value = true
    return config
  } catch {
    return { serverUrl: DEFAULT_SERVER_URL, teamId: 'default' }
  }
}

/** 保存同步配置 */
export async function setSyncConfig(config: SyncConfig): Promise<void> {
  await repo.setConfig(config)
  // 配置已改写：同步刷新快照，避免后续读取到旧值
  if (config.serverUrl) {
    syncServerUrl.value = config.serverUrl
  }
}

/** 初始化认证状态（应用启动时调用） */
export async function initAuth(): Promise<void> {
  // Tauri 环境：以原生持久化配置为唯一权威源，不碰 localStorage（避免双写竞态）。
  // 纯 WebView/浏览器调试且无 Tauri：降级从 localStorage 恢复。
  if (!isTauri) {
    restoreAuth()
    return
  }

  // 先恢复本地用户快照：即便随后 /auth/me 未发起（离线模式）或失败（服务端未启动），
  // 也能保持「已登录」，不再被误判为未登录。
  const snapshot = restoreUser()

  try {
    const config = await getSyncConfig()
    if (!config.authToken) {
      // getSyncConfig 读取失败时会回退默认配置（无 token），此时不能误判为「未登录」，
      // 否则会把用户快照一并清掉，导致下次即使读到配置也无法恢复登录态。
      if (!syncConfigLoaded.value) return
      // 无 token：清内存态。注意不删除用户快照——令牌缺失可能只是读取失败/被清理，
      // 若此时顺手删掉快照，即便后续令牌恢复也无法复原用户信息；显式登出（logout）
      // 才是唯一需要抹掉快照的路径。
      authToken.value = ''
      currentUser.value = null
      currentTeamId.value = 'default'
      return
    }
    authToken.value = config.authToken
    // 恢复登录用户所属团队（不可切换，仅作后端 team_id 参数）。
    // 之前漏恢复，导致刷新后 currentTeamId 停在默认值 'default'，
    // 在线模式下真实团队权限无法加载，canWrite 降级为只读，
    // 接口页头部的环境管理/环境选择器等被禁用（不可点击）。
    if (config.teamId) {
      currentTeamId.value = config.teamId
    }
    // 先用本地快照恢复用户信息：既避免 me 返回前 isLoggedIn 短暂为 false 引发
    // 页面按未登录加载，也保证离线模式下刷新后登录态不丢。
    if (snapshot) {
      currentUser.value = snapshot
    }
    // 离线模式下云端不可达：不再调用 /api/v1/auth/me 校验 token，
    // 直接信任本地已落盘的 token + 用户快照（离线优先设计）。
    if (dataMode.value === 'offline') {
      return
    }
    // Tauri 配置不含用户信息，需通过 /api/v1/auth/me 校验 token 并获取当前用户。
    // 经 Rust call_server_api 转发，避免 WebView 跨域 CORS。
    // 注意：call_server_api 返回的是 `ApiResult<Value>`（对象，非字符串），
    // 须用 invokeUnwrap 拆包取业务 data（即 user 对象）。
    // 之前误用 invoke<string> 再 JSON.parse 会因类型不匹配解析失败，
    // 导致 currentUser 始终为 null、刷新后显示未登录。
    try {
      // 注意：不传 token，由 Rust 回退到 sync_config 中已落盘的 auth_token 直接透传后端，
      // 由后端独立验签。这是与 login 一致的行为；避免在 Rust/Java 公钥不匹配时
      // Rust 验签失败而错误地清掉登录态（表现为刷新即未登录）。
      const user = await repo.fetchMe()
      if (user) {
        currentUser.value = user
        persistUser(user)
      }
    } catch (e) {
      // 仅「凭证确实失效」才清除登录态；服务端未启动 / 网络超时等瞬时故障保留快照，
      // 否则应用重启（后端尚未就绪）时刷新会直接掉登录。
      if (isAuthError(e)) {
        authToken.value = ''
        currentUser.value = null
        persistUser(null)
        currentTeamId.value = 'default'
      }
    }
  } catch {
    // getSyncConfig 失败（极端情况），保持内存态不变。
  }
}

/** 用户登录 */
export async function login(serverUrl: string, username: string, password: string): Promise<LoginResponse> {
  const result = await repo.login(serverUrl, username, password)
  authToken.value = result.token
  currentUser.value = result.user
  // 记录登录用户所属团队（不可切换，仅作后端 team_id 参数）
  if (result.teamId) {
    currentTeamId.value = result.teamId
  }
  // 持久化须放在 currentTeamId 之后，确保 Tauri 模式下团队 ID 一并落盘。
  persistAuth(serverUrl)
  persistUser(result.user)
  // 登录成功后立即同步一次（项目 / 团队 / 项目成员 / 请求等）
  triggerSyncAfterLogin()
  return result
}

/** 用户注册 */
export async function register(serverUrl: string, username: string, password: string, nickname: string, email: string): Promise<LoginResponse> {
  const result = await repo.register(serverUrl, username, password, nickname, email)
  authToken.value = result.token
  currentUser.value = result.user
  // 记录登录用户所属团队（不可切换，仅作后端 team_id 参数）
  if (result.teamId) {
    currentTeamId.value = result.teamId
  }
  // 持久化须放在 currentTeamId 之后，确保 Tauri 模式下团队 ID 一并落盘。
  persistAuth(serverUrl)
  persistUser(result.user)
  // 注册成功后立即同步一次
  triggerSyncAfterLogin()
  return result
}

/** 登录/注册成功后触发一次同步（拉取项目、团队、项目成员、请求等），失败不影响登录结果 */
function triggerSyncAfterLogin(): void {
  sync()
    .then(() => {
      // 同步完成后刷新团队列表（项目成员按项目按需加载，无需全局刷新）
      loadTeams().catch(() => {})
      // 加载当前团队成员，恢复 canWrite 等权限状态，避免刷新后环境选择器等被禁用
      loadCurrentTeamMembers().catch(() => {})
    })
    .catch((err) => {
      console.warn('登录后同步失败（可稍后手动同步）:', err)
    })
}

/** 登出 */
export async function logout(): Promise<void> {
  try {
    await repo.logout()
  } catch {
    // 忽略
  }
  authToken.value = ''
  currentUser.value = null
  // 仅在无 Tauri 的降级环境下清理 localStorage；Tauri 配置已由 invoke('logout') 清除
  if (!isTauri) localStorage.removeItem(AUTH_KEY)
  // 用户快照一并清除，避免下次启动用旧快照恢复出「已登录」的假象
  persistUser(null)
  // 重置为离线团队
  teams.value = [{ id: 'default', name: '离线团队', description: '' }]
  currentTeamId.value = 'default'
}

/** 检测服务端是否可连通 */
export async function checkConnection(serverUrl: string): Promise<boolean> {
  // 离线模式下云端不可达：不再调用 check_server_connection 探测，直接判定离线。
  if (dataMode.value === 'offline') {
    return false
  }
  try {
    return await repo.checkConnection(serverUrl)
  } catch {
    return false
  }
}

/**
 * 执行一次同步（只从服务器拉取，需要登录）
 * 在线模式下的新增/编辑/删除均已直接落库到后端，因此同步仅把服务器最新数据
 * 合并到本地缓存（pull-only），不再向服务器上传本地数据。
 * 返回服务端数据摘要，失败则 throw
 */
export async function sync(config?: SyncConfig): Promise<string> {
  if (!isLoggedIn.value) {
    throw new Error('请先登录后再同步')
  }

  if (syncStatus.value === 'syncing') {
    return '正在同步中...'
  }

  // 如果没传配置则从持久化文件读取
  const serverUrl = config?.serverUrl || DEFAULT_SERVER_URL

  // 离线状态下不调用 pull_data，直接返回离线提示（避免无意义的网络请求与报错）
  const online = await checkConnection(serverUrl)
  syncOnline.value = online
  if (!online) {
    syncStatus.value = 'idle'
    syncMessage.value = '离线状态，已跳过数据同步'
    return syncMessage.value
  }

  syncStatus.value = 'syncing'
  syncMessage.value = '正在从服务端拉取数据...'

  const teamId = config?.teamId || 'default'

  try {
    const result = await repo.pullData(serverUrl, teamId)
    syncStatus.value = 'success'
    syncMessage.value = result
    // 3 秒后恢复 idle
    setTimeout(() => { syncStatus.value = 'idle' }, 3000)
    return result
  } catch (e) {
    syncStatus.value = 'error'
    syncMessage.value = normalizeError(e)
    setTimeout(() => { syncStatus.value = 'idle' }, 5000)
    throw e
  }
}

/**
 * 启动自动定时同步 + 启动时连接检测
 * 注意：未登录时不会自动同步
 */
export async function startAutoSync(intervalMs: number = 3600_000): Promise<void> {
  // sync() 内部已做离线检测：离线时不调用 pull_data，并自动维护 syncOnline。
  // 因此这里仅依赖 sync() 自身的网关判断，无需重复探测连通性。
  if (isLoggedIn.value) {
    // 启动后先同步一次（离线则自动跳过）
    try {
      await sync()
    } catch {
      // 失败不中断循环
    }
  }

  // 定时检测并同步
  if (autoSyncTimer) clearInterval(autoSyncTimer)
  autoSyncTimer = setInterval(async () => {
    if (!isLoggedIn.value) return
    try {
      await sync()
    } catch {
      // 静默失败
    }
  }, intervalMs)
}

/** 停止自动同步 */
export function stopAutoSync(): void {
  if (autoSyncTimer) {
    clearInterval(autoSyncTimer)
    autoSyncTimer = null
  }
}

export function useSync() {
  return {
    syncStatus,
    syncMessage,
    syncOnline,
    authToken,
    currentUser,
    isLoggedIn,
    getSyncConfig,
    setSyncConfig,
    initAuth,
    login,
    register,
    logout,
    checkConnection,
    sync,
    startAutoSync,
    stopAutoSync,
  }
}
