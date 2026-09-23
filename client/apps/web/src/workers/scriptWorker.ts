import CryptoJS from 'crypto-js'
import type { ScriptLogEntry } from '../types'

/**
 * 脚本执行 Worker（真隔离沙箱）。
 *
 * 与主线程相比，Worker 全局作用域**没有** `window`/`document`/`__TAURI__`，
 * 因此脚本无法访问 DOM、也无法调用 Tauri 命令（如读写本地文件），
 * 从根本上消除了 `new AsyncFunction` 就地执行时的逃逸风险。
 *
 * 协议：
 * - 入参 RunRequest：{ id, script, req, res, envVars }
 * - 出参 RunResponse：{ id, logs, req, vars, persistVars, error? }
 *   · req 为脚本修改后的请求上下文（主线程据此回填表单）
 *   · vars：脚本写入的全部变量（请求级 + 环境级），主线程用来解析当前请求的 {{var}}
 *   · persistVars：仅环境级变量，主线程负责持久化到当前激活环境
 *
 * 脚本 API：Postman(pm) 风格 —— pm.variables / pm.environment / pm.globals /
 *   pm.request / pm.response / pm.test / pm.expect；并向下兼容旧全局 req/res/env/test/assert。
 */

/** 敏感环境变量 key：脚本写入时仅提示，不做额外拦截（与既有行为一致） */
const PROTECTED_ENV_KEYS = new Set([
  'token', 'tokens', 'secret', 'secrets', 'password', 'passwords',
  'apikey', 'api_key', 'authorization', 'auth', 'cookie', 'cookies',
  'access_token', 'refresh_token', 'private_key', 'credential', 'credentials',
])

// 会话级全局变量（pm.globals），跨多次脚本运行保留（进程级）
const globalsStore = new Map<string, string>()

interface RunRequest {
  id: number
  script: string
  req: Record<string, unknown>
  res: Record<string, unknown> | null
  envVars: Array<{ key: string; value: string; enabled: boolean }>
}

interface RunResponse {
  id: number
  logs: ScriptLogEntry[]
  req: Record<string, unknown>
  vars: Array<[string, string]>
  persistVars: Array<[string, string]>
  error?: string
}

// 避免依赖 lib.webworker（会与 DOM lib 冲突），用 globalThis 显式转型访问 Worker 全局 API
const ctx = globalThis as unknown as {
  onmessage: ((ev: MessageEvent<RunRequest>) => void) | null
  postMessage: (msg: RunResponse) => void
}

function deepEqual(a: any, b: any): boolean {
  if (a === b) return true
  if (a == null || b == null) return a === b
  if (typeof a !== typeof b) return false
  if (Array.isArray(a) || Array.isArray(b)) {
    if (!Array.isArray(a) || !Array.isArray(b) || a.length !== b.length) return false
    return a.every((x: any, i: number) => deepEqual(x, b[i]))
  }
  if (typeof a === 'object') {
    const ka = Object.keys(a), kb = Object.keys(b)
    if (ka.length !== kb.length) return false
    return ka.every((k) => deepEqual(a[k], b[k]))
  }
  return false
}

/** Postman 风格断言链：pm.expect(actual).to.equal(expected) 等。失败时抛错（由 pm.test 捕获）。 */
function buildExpect(actual: any): any {
  const state = { actual, negated: false }
  const api: any = {}
  const linkWords = ['to', 'be', 'been', 'is', 'are', 'was', 'were', 'that', 'which', 'and', 'has', 'have', 'with', 'a', 'an', 'of', 'it', 'at', 'does', 'did', 'same', 'contains']
  linkWords.forEach((w) => { api[w] = api })
  Object.defineProperty(api, 'not', {
    get() { state.negated = !state.negated; return api },
  })
  function evalCond(cond: boolean, msg: string) {
    if (state.negated) cond = !cond
    if (!cond) throw new Error(msg)
  }
  api.equal = api.equals = api.eq = (expected: any, msg?: string) =>
    evalCond(actual === expected, msg || `expected ${JSON.stringify(actual)} to equal ${JSON.stringify(expected)}`)
  api.eql = (expected: any, msg?: string) =>
    evalCond(deepEqual(actual, expected), msg || `expected ${JSON.stringify(actual)} to deep equal ${JSON.stringify(expected)}`)
  api.true = () => evalCond(actual === true, `expected ${JSON.stringify(actual)} to be true`)
  api.false = () => evalCond(actual === false, `expected ${JSON.stringify(actual)} to be false`)
  api.null = () => evalCond(actual === null, `expected ${JSON.stringify(actual)} to be null`)
  api.undefined = () => evalCond(actual === undefined, `expected ${JSON.stringify(actual)} to be undefined`)
  api.NaN = () => evalCond(Number.isNaN(actual), `expected ${JSON.stringify(actual)} to be NaN`)
  api.ok = () => evalCond(!!actual, `expected ${JSON.stringify(actual)} to be truthy`)
  api.exist = () => evalCond(actual !== null && actual !== undefined, `expected value to exist`)
  api.empty = () => {
    const empty = actual == null ||
      (typeof actual === 'string' && actual.length === 0) ||
      (Array.isArray(actual) && actual.length === 0) ||
      (typeof actual === 'object' && Object.keys(actual).length === 0)
    evalCond(empty, `expected ${JSON.stringify(actual)} to be empty`)
  }
  api.above = api.greaterThan = (n: number, msg?: string) => evalCond(actual > n, msg || `expected ${JSON.stringify(actual)} to be > ${n}`)
  api.least = (n: number, msg?: string) => evalCond(actual >= n, msg || `expected ${JSON.stringify(actual)} to be >= ${n}`)
  api.below = api.lessThan = (n: number, msg?: string) => evalCond(actual < n, msg || `expected ${JSON.stringify(actual)} to be < ${n}`)
  api.most = (n: number, msg?: string) => evalCond(actual <= n, msg || `expected ${JSON.stringify(actual)} to be <= ${n}`)
  api.closeTo = (n: number, delta: number, msg?: string) => evalCond(Math.abs(actual - n) <= delta, msg || `expected ${actual} to be close to ${n}`)
  api.a = api.an = (type: any, msg?: string) => {
    let ok = false
    if (type === 'array') ok = Array.isArray(actual)
    else if (type === 'object') ok = actual !== null && typeof actual === 'object'
    else if (type === 'number' || type === 'string' || type === 'boolean') ok = typeof actual === type
    else if (typeof type === 'function') ok = actual instanceof type
    evalCond(ok, msg || `expected ${JSON.stringify(actual)} to be a ${type}`)
  }
  api.include = api.contain = (sub: any, msg?: string) => {
    let ok = false
    if (typeof actual === 'string') ok = actual.includes(sub)
    else if (Array.isArray(actual)) ok = actual.includes(sub)
    else if (actual && typeof actual === 'object') ok = Object.values(actual).includes(sub)
    evalCond(ok, msg || `expected ${JSON.stringify(actual)} to include ${JSON.stringify(sub)}`)
  }
  api.match = (re: RegExp, msg?: string) => evalCond(re instanceof RegExp && re.test(actual), msg || `expected ${JSON.stringify(actual)} to match ${String(re)}`)
  api.property = function (name: string, val?: any) {
    const has = actual != null && Object.prototype.hasOwnProperty.call(actual, name)
    evalCond(has, `expected object to have property '${name}'`)
    if (arguments.length >= 2) evalCond(deepEqual(actual[name], val), `expected property '${name}' to equal ${JSON.stringify(val)}`)
  }
  api.length = (n: number, msg?: string) => {
    const len = actual == null ? -1
      : (typeof actual === 'string' || Array.isArray(actual)) ? actual.length
      : (typeof actual === 'object' ? Object.keys(actual).length : -1)
    evalCond(len === n, msg || `expected length ${len} to equal ${n}`)
  }
  api.members = (arr: any[], msg?: string) =>
    evalCond(Array.isArray(actual) && arr.every((x) => actual.some((y: any) => deepEqual(x, y))), msg || `expected to have members`)
  api.have = {
    property: (name: string, val?: any) => api.property(name, val),
    length: (n: number, msg?: string) => api.length(n, msg),
    members: (arr: any[], msg?: string) => api.members(arr, msg),
  }
  return api
}

/** 把普通 header 对象包装为「对象 + .get(key) + .toObject()」，兼容 Postman 的 headers.get 写法。 */
function asHeaderAccessor(map: Record<string, string>) {
  return Object.assign({}, map, {
    get: (k: string) => (k != null ? (map[k] ?? null) : null),
    toObject: () => ({ ...map }),
  })
}

ctx.onmessage = async (ev: MessageEvent<RunRequest>) => {
  const { id, script, req, res, envVars } = ev.data
  const logs: ScriptLogEntry[] = []
  const requestLocal = new Map<string, string>() // pm.variables / pm.globals 写入（当前请求 + {{}} 可用）
  const workerEnv = new Map<string, string>()    // pm.environment 写入（需持久化到激活环境）
  const push = (type: ScriptLogEntry['type'], message: string) =>
    logs.push({ type, message, time: Date.now() })
  const fmt = (args: unknown[]) =>
    args.map(a => (typeof a === 'object' ? JSON.stringify(a) : String(a))).join(' ')

  const consoleObj = {
    log: (...a: unknown[]) => push('info', fmt(a)),
    warn: (...a: unknown[]) => push('warn', fmt(a)),
    error: (...a: unknown[]) => push('error', fmt(a)),
  }

  const assertFn = (condition: boolean, msg?: string) => {
    if (!condition) throw new Error(`Assertion failed: ${msg || 'condition is false'}`)
  }
  const testFn = (name: string, fn: () => void) => {
    try {
      fn()
      push('success', `✓ ${name}`)
    } catch (e) {
      push('error', `✗ ${name}: ${(e as Error).message}`)
    }
  }

  // 环境读取（先取脚本内已写入的环境变量，再回退到入参环境列表）
  const envGet = (key: string): string => {
    if (workerEnv.has(key)) return workerEnv.get(key)!
    const v = envVars.find(x => x.enabled && x.key === key)
    return v?.value ?? ''
  }
  const envHas = (key: string): boolean =>
    workerEnv.has(key) || envVars.some(x => x.enabled && x.key === key)

  // 请求上下文的可变映射（脚本对 header 的修改需要回写到 req.headers）。
  // 直接复用 req.headers 的同一引用，兼容旧脚本中 `req.headers['X'] = 'y'` 的写法。
  const headerMap: Record<string, string> = (req.headers && typeof req.headers === 'object')
    ? (req.headers as Record<string, string>)
    : {}
  const bodyObj: any = {
    mode: req.bodyType === 'form' ? 'formdata' : 'raw',
    get raw() { return typeof req.body === 'string' ? req.body : '' },
    set raw(v: string) { req.body = v },
    toString() { return typeof req.body === 'string' ? req.body : '' },
  }
  const headerList = {
    add: ({ key, value }: { key: string; value: string }) => { if (key != null) headerMap[key] = value },
    get: (key: string) => (key != null ? (headerMap[key] ?? null) : null),
    remove: (key: string) => { delete headerMap[key] },
    each: (fn: (h: { key: string; value: string }) => void) =>
      Object.entries(headerMap).forEach(([k, v]) => fn({ key: k, value: v })),
    toObject: () => ({ ...headerMap }),
    all: () => Object.entries(headerMap).map(([k, v]) => ({ key: k, value: v })),
  }

  const pm: any = {
    variables: {
      get: (k: string) => requestLocal.get(k) ?? globalsStore.get(k) ?? (envGet(k) || ''),
      set: (k: string, v: any) => { requestLocal.set(k, String(v)) },
      has: (k: string) => requestLocal.has(k) || globalsStore.has(k) || envHas(k),
      toObject: () => Object.fromEntries([...requestLocal.entries()]),
    },
    environment: {
      get: (k: string) => envGet(k) || '',
      set: (k: string, v: any) => {
        workerEnv.set(k, String(v))
        requestLocal.set(k, String(v))
        if (PROTECTED_ENV_KEYS.has(k.toLowerCase())) {
          push('info', `敏感变量 ${k} 已写入环境（含当前请求临时值与持久化）`)
        }
      },
      has: (k: string) => envHas(k),
      toObject: () => Object.fromEntries(envVars.filter(v => v.enabled).map(v => [v.key, v.value])),
      clear: () => {},
    },
    globals: {
      get: (k: string) => globalsStore.get(k) ?? '',
      set: (k: string, v: any) => { globalsStore.set(k, String(v)); requestLocal.set(k, String(v)) },
      has: (k: string) => globalsStore.has(k),
      toObject: () => Object.fromEntries([...globalsStore.entries()]),
      clear: () => { globalsStore.clear() },
    },
    collectionVariables: {
      get: (k: string) => globalsStore.get(k) ?? '',
      set: (k: string, v: any) => { globalsStore.set(k, String(v)); requestLocal.set(k, String(v)) },
      has: (k: string) => globalsStore.has(k),
    },
    request: {
      get method(): string { return (req.method as string) ?? '' },
      set method(v: string) { req.method = v },
      get url(): string { return (req.url as string) ?? '' },
      set url(v: string) { req.url = v },
      get headers() { return headerList },
      set headers(v: any) { if (v && typeof v === 'object') Object.assign(headerMap, v) },
      get body() { return bodyObj },
      set body(v: any) { if (typeof v === 'string') req.body = v; else if (v && typeof v.raw === 'string') req.body = v.raw },
    },
    response: res ? {
      code: res.status,
      status: res.statusText,
      responseTime: res.timeMs,
      headers: asHeaderAccessor((res.headers && typeof res.headers === 'object') ? (res.headers as Record<string, string>) : {}),
      get body() { return String(res.body ?? '') },
      text: () => String(res.body ?? ''),
      json: () => { try { return JSON.parse(String(res.body ?? '')) } catch { return null } },
    } : undefined,
    test: testFn,
    expect: (actual: any) => buildExpect(actual),
    iterationData: { get: () => ({}), has: () => false },
    cookies: { get: () => null, has: () => false },
  }

  // 兼容旧语法：req / res / env / test / assert
  const envObj = {
    get: (k: string) => pm.environment.get(k),
    set: (k: string, v: any) => pm.environment.set(k, v),
  }
  // 旧 res.json 兼容
  if (res && typeof res === 'object') {
    res.json = () => { try { return JSON.parse(String((res as { body?: unknown }).body ?? '')) } catch { return null } }
  }

  let error: string | undefined
  try {
    // 仍用 AsyncFunction 注入封闭作用域（不作为安全边界，安全边界是 Worker 本身）
    const AsyncFunction = Object.getPrototypeOf(async function () {}).constructor as
      new (...args: unknown[]) => (...args: unknown[]) => Promise<unknown>
    const fn = new AsyncFunction('pm', 'req', 'res', 'console', 'env', 'test', 'assert', 'CryptoJS', script)
    await fn(pm, req, res, consoleObj, envObj, testFn, assertFn, CryptoJS)
  } catch (e) {
    error = (e as Error)?.message ?? String(e)
    push('error', error)
  }

  // 回写被脚本修改的 headers（method/url/body 已直接在 req 上修改）
  req.headers = headerMap

  // 汇总：vars（请求级 + 环境级，供 {{}} 解析）；persistVars（仅环境级，需持久化）
  const allVars: Array<[string, string]> = [
    ...requestLocal.entries(),
    ...workerEnv.entries(),
  ]
  const persist: Array<[string, string]> = [...workerEnv.entries()]

  ctx.postMessage({ id, logs, req, vars: allVars, persistVars: persist, error })
}
