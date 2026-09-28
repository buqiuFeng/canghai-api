import type { Completion, CompletionContext, CompletionResult } from '@codemirror/autocomplete'

/**
 * 前后置脚本可用的 API 目录。
 *
 * 同一份数据同时驱动两处：
 * - 「API 速查」面板（按 `group` 分组展示，点击插入 `insert`）；
 * - 脚本编辑器的自动补全（在 `owner.` 之后列出该 owner 下的成员）。
 *
 * `insert` 中的 `'`（空引号）会被补全逻辑用作光标落点提示：插入后光标停在首个引号内。
 */
export interface ScriptApiEntry {
  /** 速查面板分组 */
  group: string
  /** 所属链；顶层可调用项用 '' */
  owner: string
  /** 补全列表显示的成员名 */
  name: string
  /** 插入文本 */
  insert: string
  /** 简短说明 */
  detail: string
  /** 详细文档（补全下拉的 info 面板） */
  doc?: string
}

export const SCRIPT_API: ScriptApiEntry[] = [
  // ===== 变量 =====
  { group: '变量', owner: 'pm.variables', name: 'get', insert: "pm.variables.get('key')", detail: '读取请求级变量', doc: '请求级变量仅当前请求可用，不持久化。' },
  { group: '变量', owner: 'pm.variables', name: 'set', insert: "pm.variables.set('key', value)", detail: '写入请求级变量', doc: '当前请求内可用（含 {{key}} 解析），不写入环境、不持久化。' },
  { group: '变量', owner: 'pm.variables', name: 'has', insert: "pm.variables.has('key')", detail: '请求级变量是否存在' },
  { group: '变量', owner: 'pm.variables', name: 'toObject', insert: 'pm.variables.toObject()', detail: '导出请求级变量对象' },
  { group: '变量', owner: 'pm.environment', name: 'get', insert: "pm.environment.get('key')", detail: '读取环境变量' },
  { group: '变量', owner: 'pm.environment', name: 'set', insert: "pm.environment.set('key', value)", detail: '写入环境变量（持久化）', doc: '写入当前激活环境并持久化，同名变量会被更新而不是新增。之后可在 URL/Header/Body 中用 {{key}} 引用。' },
  { group: '变量', owner: 'pm.environment', name: 'has', insert: "pm.environment.has('key')", detail: '环境变量是否存在' },
  { group: '变量', owner: 'pm.environment', name: 'toObject', insert: 'pm.environment.toObject()', detail: '导出环境变量对象' },
  { group: '变量', owner: 'pm.globals', name: 'get', insert: "pm.globals.get('key')", detail: '读取全局变量（会话内存）' },
  { group: '变量', owner: 'pm.globals', name: 'set', insert: "pm.globals.set('key', value)", detail: '写入全局变量（会话内存）' },
  { group: '变量', owner: 'pm.collectionVariables', name: 'get', insert: "pm.collectionVariables.get('key')", detail: '读取集合变量（等同 globals）' },
  { group: '变量', owner: 'pm.collectionVariables', name: 'set', insert: "pm.collectionVariables.set('key', value)", detail: '写入集合变量（等同 globals）' },
  { group: '变量', owner: 'env', name: 'get', insert: "env.get('key')", detail: '读取环境变量（旧语法）' },
  { group: '变量', owner: 'env', name: 'set', insert: "env.set('key', value)", detail: '写入环境变量（旧语法）' },

  // ===== 请求（前置脚本）=====
  { group: '请求', owner: 'pm.request', name: 'method', insert: 'pm.request.method', detail: '请求方法（字符串）' },
  { group: '请求', owner: 'pm.request', name: 'url', insert: 'pm.request.url', detail: '请求 URL（字符串，可赋值）' },
  { group: '请求', owner: 'pm.request', name: 'body', insert: 'pm.request.body.raw', detail: '请求体（字符串；也可整体赋值）' },
  { group: '请求', owner: 'pm.request.headers', name: 'add', insert: "pm.request.headers.add({ key: 'Authorization', value: 'Bearer ' + pm.environment.get('token') })", detail: '添加请求头' },
  { group: '请求', owner: 'pm.request.headers', name: 'get', insert: "pm.request.headers.get('Content-Type')", detail: '读取请求头' },
  { group: '请求', owner: 'pm.request.headers', name: 'remove', insert: "pm.request.headers.remove('X-Debug')", detail: '删除请求头' },
  { group: '请求', owner: 'pm.request.headers', name: 'toObject', insert: 'pm.request.headers.toObject()', detail: '导出请求头对象' },
  { group: '请求', owner: 'req', name: 'url', insert: 'req.url', detail: '请求 URL（旧语法，可赋值）' },
  { group: '请求', owner: 'req', name: 'headers', insert: "req.headers['X-Token'] = value", detail: '请求头映射（旧语法）' },
  { group: '请求', owner: 'req', name: 'body', insert: 'req.body', detail: '请求体（旧语法）' },

  // ===== 响应（后置脚本）=====
  { group: '响应', owner: 'pm.response', name: 'code', insert: 'pm.response.code', detail: 'HTTP 状态码（数字）' },
  { group: '响应', owner: 'pm.response', name: 'status', insert: 'pm.response.status', detail: '状态文本' },
  { group: '响应', owner: 'pm.response', name: 'responseTime', insert: 'pm.response.responseTime', detail: '耗时（毫秒）' },
  { group: '响应', owner: 'pm.response', name: 'headers', insert: "pm.response.headers.get('Content-Type')", detail: '响应头' },
  { group: '响应', owner: 'pm.response', name: 'text', insert: 'pm.response.text()', detail: '响应体文本' },
  { group: '响应', owner: 'pm.response', name: 'json', insert: 'pm.response.json()', detail: '响应体 JSON（解析失败返回 null）' },
  { group: '响应', owner: 'res', name: 'json', insert: 'res.json()', detail: '响应体 JSON（旧语法）' },
  { group: '响应', owner: 'res', name: 'body', insert: 'res.body', detail: '响应体文本（旧语法）' },

  // ===== 断言 / 测试 =====
  { group: '断言', owner: 'pm', name: 'test', insert: "pm.test('状态码 200', () => { pm.expect(pm.response.code).to.equal(200) })", detail: '注册一个测试用例' },
  { group: '断言', owner: 'pm', name: 'expect', insert: 'pm.expect(actual).to.equal(expected)', detail: '断言表达式' },
  { group: '断言', owner: '', name: 'test', insert: "test('用例名', () => { assert(true, '失败信息') })", detail: '测试用例（旧语法）' },
  { group: '断言', owner: '', name: 'assert', insert: "assert(condition, '失败信息')", detail: '断言条件为真' },

  // ===== 工具 =====
  { group: '工具', owner: 'console', name: 'log', insert: "console.log('value', value)", detail: '打印日志（显示在脚本日志）' },
  { group: '工具', owner: 'console', name: 'warn', insert: "console.warn('warning')", detail: '打印警告' },
  { group: '工具', owner: 'console', name: 'error', insert: "console.error('error')", detail: '打印错误' },

  // ===== 加密 =====
  { group: '加密', owner: 'CryptoJS', name: 'MD5', insert: "CryptoJS.MD5('text').toString()", detail: 'MD5 摘要' },
  { group: '加密', owner: 'CryptoJS', name: 'SHA256', insert: "CryptoJS.SHA256('text').toString()", detail: 'SHA256 摘要' },
  { group: '加密', owner: 'CryptoJS', name: 'HmacSHA256', insert: "CryptoJS.HmacSHA256('text', 'secret').toString()", detail: 'HMAC-SHA256 签名' },
  { group: '加密', owner: 'CryptoJS', name: 'AES', insert: "CryptoJS.AES.encrypt('text', 'secret').toString()", detail: 'AES 加密' },
  { group: '加密', owner: 'CryptoJS.enc', name: 'Base64', insert: "CryptoJS.enc.Base64.stringify(CryptoJS.enc.Utf8.parse('text'))", detail: 'Base64 编码' },
  { group: '加密', owner: 'CryptoJS.enc', name: 'Hex', insert: "CryptoJS.enc.Hex.stringify(CryptoJS.enc.Utf8.parse('text'))", detail: 'Hex 编码' },
]

/** 速查面板分组顺序 */
export const SCRIPT_API_GROUPS = ['变量', '请求', '响应', '断言', '工具', '加密']

/**
 * 顶层命名空间（输入普通标识符时提示）。
 * `test` / `assert` 是可调用项，由 `SCRIPT_API` 中 owner 为空的条目提供，此处不重复列出。
 */
const ROOTS: Record<string, string> = {
  pm: 'Postman 兼容 API 入口',
  env: '环境变量（旧语法）',
  req: '即将发出的请求（旧语法）',
  res: '响应对象（旧语法）',
  console: '脚本日志',
  CryptoJS: '加密算法库',
}

const byOwner = new Map<string, ScriptApiEntry[]>()
for (const e of SCRIPT_API) {
  const list = byOwner.get(e.owner) ?? []
  list.push(e)
  byOwner.set(e.owner, list)
}

/** 直接子命名空间：`pm` → `pm.environment`、`pm.request` → `pm.request.headers` 等 */
const childNamespaces = new Map<string, string[]>()
for (const owner of byOwner.keys()) {
  const i = owner.lastIndexOf('.')
  if (i < 0) continue // 顶层命名空间由 ROOTS 覆盖，避免重复
  const parent = owner.slice(0, i)
  const list = childNamespaces.get(parent) ?? []
  list.push(owner)
  childNamespaces.set(parent, list)
}

/** 插入文本中的光标落点：首个单引号之后（落在空引号内） */
function cursorOffset(insert: string): number {
  const i = insert.indexOf("'")
  return i >= 0 ? i + 1 : insert.length
}

/**
 * 补全时插入的文本：去掉 `owner.` 前缀，只保留成员本身。
 * 补全的替换区间从「最后一个点之后」开始，若插入完整路径会导致路径重复
 * （如 `pm.environment.get` → `pm.environment.pm.environment.get(…)`）。
 */
function memberInsert(e: ScriptApiEntry): string {
  if (!e.owner) return e.insert
  const prefix = `${e.owner}.`
  return e.insert.startsWith(prefix) ? e.insert.slice(prefix.length) : e.insert
}

function applyText(text: string): Completion['apply'] {
  return (view, _completion, from, to) => {
    view.dispatch({
      changes: { from, to, insert: text },
      selection: { anchor: from + cursorOffset(text) },
    })
  }
}

/** 命名空间补全项：插入 `name.` 并把光标移到点后，便于继续补全成员 */
function namespaceCompletion(name: string, detail = '命名空间'): Completion {
  return {
    label: name,
    type: 'namespace',
    detail,
    info: name,
    apply: (view, _completion, from, to) => {
      view.dispatch({
        changes: { from, to, insert: `${name}.` },
        selection: { anchor: from + name.length + 1 },
      })
    },
  }
}

function toCompletion(e: ScriptApiEntry): Completion {
  const insert = memberInsert(e)
  return {
    label: e.name,
    detail: e.detail,
    info: e.doc ? `${insert}\n\n${e.doc}` : insert,
    apply: applyText(insert),
  }
}

/** 顶层候选：命名空间 + 顶层可调用项（test/assert） */
function topLevelOptions(): Completion[] {
  return [
    ...Object.entries(ROOTS).map(([name, detail]) => namespaceCompletion(name, detail)),
    ...(byOwner.get('') ?? []).map(toCompletion),
  ]
}

/**
 * 脚本补全源：支持链式成员补全（`pm.` → 子命名空间、`pm.environment.` → get/set/…）
 * 与顶层标识符补全；空位置手动唤起（Ctrl+Space）时给出顶层候选。
 */
export function scriptCompletions(context: CompletionContext): CompletionResult | null {
  // 匹配光标前的链式 token（含点），如 `pm.request.headers.`
  const token = context.matchBefore(/[\w$]+(?:\.[\w$]*)*/)

  if (!token) {
    // 光标前没有标识符（如空文档）：仅在手动唤起时给顶层候选
    if (!context.explicit) return null
    return { from: context.pos, options: topLevelOptions(), validFor: /^[\w$]*$/ }
  }

  const text = token.text
  const dot = text.lastIndexOf('.')

  if (dot >= 0) {
    const owner = text.slice(0, dot)
    const entries = byOwner.get(owner) ?? []
    const children = (childNamespaces.get(owner) ?? []).map(ns => ns.slice(ns.lastIndexOf('.') + 1))
    if (!entries.length && !children.length) return null
    return {
      from: token.from + dot + 1,
      options: [...children.map(name => namespaceCompletion(name)), ...entries.map(toCompletion)],
      validFor: /^[\w$]*$/,
    }
  }

  // 顶层：仅在手动唤起或已在输入标识符时提示，避免干扰其它输入
  if (!context.explicit && !/^[\w$]*$/.test(text)) return null
  return {
    from: token.from,
    options: topLevelOptions(),
    validFor: /^[\w$]*$/,
  }
}
