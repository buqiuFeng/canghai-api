import { invoke } from '@tauri-apps/api/core'
import type { ApiResult } from '@/types'
import { unwrapResult, normalizeError } from '@/lib/tauri'
import { authToken } from '@/stores/session'
import { useDataModeStore } from '@/stores/dataMode'

/**
 * 在线传输层（Phase 8.3）。
 *
 * 从 `composables/useApi.ts` 下沉到 `lib/`：它本质是**无状态函数**，
 * 只读取共享态（`authToken` / `dataMode`）。下沉后依赖方向变为
 * `repositories → lib → stores`，彻底消除 `repositories → composables` 反向依赖。
 *
 * 兼容：`composables/useApi.ts` 仍按原名 re-export，既有调用点零改动。
 */
export { normalizeError }

/**
 * 通过 Rust `call_server_api` 代理调用后端接口（在线模式）。
 *
 * 前端零直连后端，避免 WebView 跨域 CORS。解包统一走 `unwrapResult`，
 * 与本地命令 `invokeUnwrap` 共用同一份信封解析与错误归一（Phase 6.5：解包方式收敛为一种）。
 *
 * 鉴权：将前端内存中的 `authToken` 作为 `token` 参数传给 Rust 端：
 * - Rust 端用内置公钥验签（RS256 JWT），拿到 userId/teamId；
 * - 转发 Java 端时携带 `Authorization: Bearer <token>` 与 body.token 双通道。
 */
export async function invokeApi<T>(path: string, payload: unknown): Promise<T> {
  // 离线模式下云端不可达：不再调用 Rust `call_server_api` 转发后端 HTTP 接口，
  // 直接拒绝，避免无意义的网络请求与报错。
  if (useDataModeStore().dataMode === 'offline') {
    throw new Error('离线模式下不可用')
  }
  // Rust `call_server_api` 返回的是 `ApiResult<Value>`（JSON 对象，非字符串），
  // 此处按对象直接接收并解析，与 Rust 端改造对齐。
  const result = (await invoke('call_server_api', {
    path,
    payload: (payload ?? {}) as Record<string, unknown>,
    token: authToken.value || undefined,
  })) as ApiResult<T>
  return unwrapResult<T>(result)
}

/** 服务端导入结果（对应 Java 端 `ImportResult`） */
export interface ImportSummary {
  /** 识别出的来源格式：openapi / postman / curl / apipost / canghai */
  source?: string
  /** 成功导入的接口数 */
  imported: number
  /** 跳过数（服务端为单事务导入，恒为 0，保留以兼容既有提示逻辑） */
  skipped: number
  /** 新建的分类数 */
  categories: number
  /** 新建的环境数 */
  envImported: number
  /** 新建的环境变量数 */
  envVarImported: number
}

/**
 * 在线导入：把文件原文上传给服务端，由服务端解析并落库。
 *
 * 与 {@link invokeApi} 的差异（因此不能复用它）：
 * - 走独立命令 `import_file`，请求体是 `multipart/form-data`（文件上传）；
 * - 鉴权只能靠 `Authorization` 头：后端 `RequestBodyUtil` 是按 JSON 解析 body 取 token 的，
 *   multipart body 取不到 token（`call_server_api` 那条「token 放 body」的通道在此不可用）。
 *
 * 服务端成功后，调用方应触发一次 `sync()` 把数据拉回本地缓存。
 *
 * @param fileName  文件名（仅用于服务端日志，格式由服务端按内容识别）
 * @param content   文件原文（支持的导入格式均为文本）
 * @param projectId 目标项目 id
 */
export async function uploadImportFile(
  fileName: string,
  content: string,
  projectId: string,
): Promise<ImportSummary> {
  if (useDataModeStore().dataMode === 'offline') {
    throw new Error('离线模式下不可用')
  }
  const result = (await invoke('import_file', {
    fileName,
    content,
    projectId,
    token: authToken.value || undefined,
  })) as ApiResult<ImportSummary>
  return unwrapResult<ImportSummary>(result)
}
