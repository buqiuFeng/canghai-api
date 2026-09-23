import { ref } from 'vue'
import type { ScriptLogEntry } from '@/types'

export const PRE_SCRIPT_TEMPLATE = `// 前置脚本（Pre-request Script）· 使用 Postman(pm) 语法，支持 async/await
//
// 变量读写：
//   pm.variables.get(key) / set(key, val)   请求级变量（仅当前请求可用，不持久化）
//   pm.environment.get(key) / set(key, val) 环境变量（持久化到当前激活环境，可在 {{key}} 引用）
//   pm.globals.get/set(key, val)            全局变量（当前会话内存）
//   pm.collectionVariables                  等同 pm.globals
//
// 读写即将发出的请求：
//   pm.request.method                  请求方法
//   pm.request.url                     请求 URL（字符串，可直接赋值）
//   pm.request.headers.add({key,value}) / get(key) / remove(key)
//   pm.request.body.raw               请求体（字符串）；也可整体赋值 pm.request.body = '...'
//
// 其他：CryptoJS（MD5/SHA256/HMAC/AES/Base64…）、console.log、pm.test(name, fn)
// 变量引用：URL/Header/Body 中写 {{key}}；动态变量如 {{$guid}} {{$timestamp}} {{$randomFirstName}}
//
// 示例：
//   pm.request.headers.add({ key: 'Authorization', value: 'Bearer ' + pm.environment.get('token') })
//   pm.variables.set('ts', String(Date.now()))
//   pm.request.url = pm.request.url + '?ts={{ts}}'
//   const sign = CryptoJS.MD5(pm.request.body.raw + pm.environment.get('secret')).toString()
//   pm.request.headers.add({ key: 'X-Sign', value: sign })
`

export const POST_SCRIPT_TEMPLATE = `// 后置脚本（Post-response Script）· 使用 Postman(pm) 语法，支持 async/await
//
// 响应对象：
//   pm.response.code            状态码（数字）
//   pm.response.status          状态文本
//   pm.response.responseTime    耗时（毫秒）
//   pm.response.headers         Header 对象（{key: value}）
//   pm.response.text()          响应体文本
//   pm.response.json()          响应体 JSON（解析失败返回 null）
//
// 变量 / 断言：
//   pm.environment.set('token', pm.response.json().token)
//   pm.test('状态码 200', () => { pm.expect(pm.response.code).to.equal(200) })
//   pm.test('返回含 token', () => { pm.expect(pm.response.json().token).to.be.a('string') })
//
// 安全限制：脚本运行于独立 Web Worker 沙箱，无法访问浏览器 DOM 或系统级 API。
`

/**
 * 危险标识符黑名单（静态校验，第一道防线）。
 * 以词边界匹配，避免把 `globals` 误判为 `global`、把 `importScripts` 之外的 `import` 误判等。
 * 命中任一即拒绝执行——即便脚本运行在 Worker 中，仍拦截 Worker 环境里存在的
 * fetch/WebSocket/importScripts/postMessage 等能力，降低脚本外联/串扰风险。
 */
const DANGEROUS_PATTERN = /\b(window|globalThis|self|document|location|navigator|fetch|XMLHttpRequest|WebSocket|Worker|importScripts|localStorage|sessionStorage|indexedDB|caches|Function|eval|constructor|require|process|global|postMessage|addEventListener|setInterval|import|__TAURI__|__TAURI_INTERNALS__)\b/

function assertSafeScript(script: string): void {
  const m = script.match(DANGEROUS_PATTERN)
  if (m) {
    throw new Error(`脚本包含被禁止的标识符: "${m[0]}"`)
  }
}

interface ScriptRunRequest {
  id: number
  script: string
  req: Record<string, unknown>
  res: Record<string, unknown> | null
  envVars: Array<{ key: string; value: string; enabled: boolean }>
}

interface ScriptRunResponse {
  id: number
  logs: ScriptLogEntry[]
  req: Record<string, unknown>
  vars: Array<[string, string]>
  persistVars?: Array<[string, string]>
  error?: string
}

const SCRIPT_TIMEOUT_MS = 15_000

/**
 * 脚本引擎 —— 通过 Web Worker 执行用户 JS 脚本，实现真正的作用域隔离。
 *
 * 与旧实现（主线程 `new AsyncFunction`）相比：Worker 无 window/document/__TAURI__，
 * 脚本即便绕过静态黑名单也无法访问 DOM 或调用 Tauri 命令。
 *
 * 对外 API 与旧版保持一致：`runScript(script, reqCtx, resCtx) => Promise<ScriptLogEntry[]>`，
 * 并保留 `scriptVars`（脚本 env.set 的内存变量，供当前请求 {{var}} 解析）。
 */
export function useScriptEngine(
  activeVariables: { value: Array<{ key: string; value: string; enabled: boolean }> },
  activeEnv: { value: { id: string } | null },
  updateVariable: (v: any) => Promise<void>,
  saveVariable: (envId: string, key: string, value: string) => Promise<any>,
) {
  const scriptLog = ref<ScriptLogEntry[]>([])
  // 脚本写入的内存变量，优先级高于持久化环境变量，用于当前请求的 {{var}} 解析
  const scriptVars = new Map<string, string>()

  // ===== Worker 懒创建 + 请求路由 =====
  let worker: Worker | null = null
  let seq = 0
  const pending = new Map<number, { resolve: (r: ScriptRunResponse) => void; reject: (e: Error) => void }>()

  function disposeWorker(w: Worker) {
    try {
      w.terminate()
    } catch { /* ignore */ }
    if (worker === w) worker = null
  }

  function ensureWorker(): Worker {
    if (worker) return worker
    const w = new Worker(new URL('../workers/scriptWorker.ts', import.meta.url), { type: 'module' })
    w.onmessage = (ev: MessageEvent<ScriptRunResponse>) => {
      const p = pending.get(ev.data.id)
      if (p) {
        pending.delete(ev.data.id)
        p.resolve(ev.data)
      }
    }
    w.onerror = (ev: ErrorEvent) => {
      const err = new Error(ev.message || '脚本 Worker 运行错误')
      for (const p of pending.values()) p.reject(err)
      pending.clear()
      disposeWorker(w)
    }
    worker = w
    return w
  }

  /** 仅保留可结构化克隆的普通字段（剔除函数，如 res.json），供 postMessage 传输 */
  function toCloneable(src: Record<string, any> | null): Record<string, unknown> | null {
    if (!src) return null
    const out: Record<string, unknown> = {}
    for (const [k, v] of Object.entries(src)) {
      if (typeof v !== 'function') out[k] = v
    }
    return out
  }

  async function runScript(
    script: string,
    reqCtx: Record<string, any>,
    resCtx: Record<string, any> | null,
  ): Promise<ScriptLogEntry[]> {
    if (!script?.trim()) return []
    // 静态安全校验（第一道防线，快速失败）
    assertSafeScript(script)
    // 每次执行前清空临时变量
    scriptVars.clear()

    const w = ensureWorker()
    const id = ++seq
    const request: ScriptRunRequest = {
      id,
      script,
      req: reqCtx,
      res: toCloneable(resCtx),
      envVars: activeVariables.value.map(v => ({ key: v.key, value: v.value, enabled: v.enabled })),
    }

    const response = await new Promise<ScriptRunResponse>((resolve, reject) => {
      const timer = setTimeout(() => {
        if (pending.has(id)) {
          pending.delete(id)
          // 脚本疑似死循环：终止 Worker，下次调用自动重建
          disposeWorker(w)
          reject(new Error('脚本执行超时（>15s），已终止'))
        }
      }, SCRIPT_TIMEOUT_MS)
      pending.set(id, {
        resolve: (r) => { clearTimeout(timer); resolve(r) },
        reject: (e) => { clearTimeout(timer); reject(e) },
      })
      w.postMessage(request)
    })

    // Worker 结构化克隆：把脚本对 req 的修改回填到原对象，调用方据此同步表单/URL
    if (response.req && typeof response.req === 'object') {
      Object.assign(reqCtx, response.req)
    }

    // 应用脚本写入的变量：
    // - 请求级 + 环境级变量统一填充到 scriptVars，供当前请求的 {{var}} 解析
    // - 仅环境级变量（persistVars）需要持久化到激活环境
    for (const [key, value] of response.vars) {
      scriptVars.set(key, value)
    }
    if (activeEnv.value) {
      for (const [key, value] of response.persistVars ?? []) {
        const existing = activeVariables.value.find(x => x.key === key)
        if (existing) {
          await updateVariable({ ...existing, value })
        } else {
          await saveVariable(activeEnv.value.id, key, value)
        }
      }
    }

    if (response.error) {
      // L9：脚本异常记录后向上抛出，由调用方决定是否阻断请求
      throw new Error(`脚本执行错误: ${response.error}`)
    }
    return response.logs
  }

  return {
    scriptLog,
    scriptVars,
    runScript,
  }
}
