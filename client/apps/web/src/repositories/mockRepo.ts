import { invoke } from '@tauri-apps/api/core'
import type { ApiResult } from '@/types'
import type { MockRoute } from '@/types'

/**
 * 本地 Mock 服务数据仓库 —— 收敛 Mock 服务生命周期命令。
 */
export function useMockRepo() {
  return {
    start(routes: MockRoute[], port: number) {
      return invoke<ApiResult<null>>('start_mock_server', { routes, port })
    },
    stop() {
      return invoke<ApiResult<null>>('stop_mock_server')
    },
  }
}
