import { invokeUnwrap } from '@/lib/tauri'

/**
 * WebSocket 数据仓库 —— 收敛 ws 会话命令（连接/发送/断开）。
 *
 * Rust 命令统一返回 `ApiResult<T>` 信封，故必须经 `invokeUnwrap` 拆包；
 * 此前 `invoke<string>('ws_connect')` 直接把信封当作 sessionId 返回，导致后续
 * `ws_send` 收到非法会话 id（契约漂移缺陷）。
 */
export function useWebSocketRepo() {
  return {
    connect(args: { url: string; headers: [string, string][] }) {
      return invokeUnwrap<string>('ws_connect', { args })
    },
    send(sessionId: string, message: string) {
      return invokeUnwrap<void>('ws_send', { sessionId, message })
    },
    disconnect(sessionId: string) {
      return invokeUnwrap<void>('ws_disconnect', { sessionId })
    },
  }
}
