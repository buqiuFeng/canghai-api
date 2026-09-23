import { invoke } from '@tauri-apps/api/core'

/**
 * 错误处理辅助：将 `invoke` 抛出的未知错误收敛为字符串。
 *
 * Tauri 命令失败时 `reject` 出来的是 `string | Error | Record<string,unknown>`，
 * 这里统一转成人类可读文案。原先定义在 `composables/useApi.ts`，但 `useSync`/`useTeams`
 * 都需要它，会造成 composable 之间的循环依赖，故下沉到本模块（依赖图中的叶子）。
 */
export function normalizeError(err: unknown): string {
  if (typeof err === 'string') return err
  if (err instanceof Error) return err.message
  try {
    return JSON.stringify(err)
  } catch {
    return String(err)
  }
}

/** Rust 端命令统一响应包装 `{ success, code, msg, data }`。 */
export interface TauriResult<T> {
  success: boolean
  code: number
  msg: string
  data?: T
}

/**
 * 统一错误码常量（与 `docs/API_CONTRACT.md` §3 对齐）。
 *
 * Java `ErrorCode` 枚举为唯一权威定义，前端仅作同义常量，
 * 避免在业务代码里散用魔法数字判断失败类型。
 */
export const ErrorCodes = {
  SUCCESS: 0,
  PARAM_INVALID: 40000,
  PARAM_MISSING: 40001,
  UNAUTHORIZED: 40100,
  BAD_CREDENTIALS: 40101,
  TOKEN_EXPIRED: 40102,
  FORBIDDEN: 40103,
  NOT_FOUND: 40400,
  CONFLICT: 40900,
  BUSINESS_ERROR: 42200,
  INTERNAL_ERROR: 50000,
  DB_ERROR: 50001,
  DOWNSTREAM_ERROR: 50002,
} as const

/**
 * 统一业务错误类型：携带业务码，便于按码分支处理。
 *
 * 原先失败只抛字符串（`new Error(msg)`），业务码在解包时丢失，调用方无法区分
 * 「未登录」与「参数错误」。此处以 `ApiError` 承载 code，`message` 兼容原有提示用法。
 */
export class ApiError extends Error {
  readonly code: number

  constructor(code: number, msg: string) {
    super(msg)
    this.name = 'ApiError'
    this.code = code ?? -1
  }
}

/**
 * 拆包统一响应信封 `{ success, code, msg, data }`：
 * - 若返回值不是信封结构，原样返回（兼容裸值命令）；
 * - `success=false` 抛 {@link ApiError}（带业务码与 msg）；
 * - 否则返回 `data`。
 *
 * `invokeUnwrap` 与 `invokeApi`（在线代理）共用此唯一解包实现。
 */
export function unwrapResult<T>(r: TauriResult<T> | null | undefined): T {
  if (!r || typeof r !== 'object' || !('success' in r)) return r as unknown as T
  if (!r.success) throw new ApiError(r.code, r.msg || '操作失败')
  return r.data as T
}

/**
 * 调用 Tauri 命令并拆包统一响应（本地命令单一入口）。
 */
export async function invokeUnwrap<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  return unwrapResult<T>(await invoke<TauriResult<T>>(cmd, args))
}
