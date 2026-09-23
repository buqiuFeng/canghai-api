import { invokeUnwrap } from '@/lib/tauri'
import type { SyncConfig, UserInfo, LoginResponse } from '@/types'

/**
 * 认证 / 同步数据仓库 —— 收敛登录注册、配置读写、连通性探测、数据拉取等命令。
 * （在线/离线的业务判断仍在 composable，repo 只负责命令调用与拆包。）
 */
export function useSyncRepo() {
  return {
    getConfig: () => invokeUnwrap<SyncConfig>('get_sync_config'),
    setConfig: (config: SyncConfig) => invokeUnwrap<void>('set_sync_config', { config }),
    login: (serverUrl: string, username: string, password: string) =>
      invokeUnwrap<LoginResponse>('login', { serverUrl, username, password }),
    register: (serverUrl: string, username: string, password: string, nickname: string, email: string) =>
      invokeUnwrap<LoginResponse>('register', { serverUrl, username, password, nickname, email }),
    logout: () => invokeUnwrap<void>('logout'),
    checkConnection: (serverUrl: string) => invokeUnwrap<boolean>('check_server_connection', { serverUrl }),
    pullData: (serverUrl: string, teamId: string) =>
      invokeUnwrap<string>('pull_data', { serverUrl, teamId }),
    /** 经 Rust 转发后端 /api/v1/auth/me 校验 token 并取当前用户。 */
    fetchMe: () => invokeUnwrap<UserInfo>('call_server_api', { path: '/api/v1/auth/me', payload: {} }),
  }
}
