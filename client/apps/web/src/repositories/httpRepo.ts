import { invoke } from '@tauri-apps/api/core'
import type { ApiResult, BackendResp } from '@/types'

/** HTTP 代理请求参数（allowPrivate 由全局设置决定）。 */
export interface HttpSendReq {
  method: string
  url: string
  headers: Record<string, string>
  body: string | null
  stream?: boolean
  allowPrivate: boolean
}

/**
 * HTTP 代理数据仓库 —— 收敛本地 Rust HTTP 代理命令（普通 / 流式）。
 * 该工具的核心能力：经 Rust 发送请求以规避 WebView 的 CORS 限制。
 */
export function useHttpRepo() {
  return {
    send(req: HttpSendReq) {
      return invoke<ApiResult<BackendResp>>('send_http_request', { req })
    },
    sendStream(req: HttpSendReq) {
      return invoke<ApiResult<BackendResp>>('send_http_request_stream', { req })
    },
  }
}
