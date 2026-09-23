import type { Category } from '@/composables/useCategories'
import type { SavedRequest } from '@/types'
import type { KV, Method, BodyType } from '@/types'
import { uid, now } from '@/utils'

/** 导入结果统计 */
export interface ImportResult {
  imported: number
  skipped: number
  source: 'openapi' | 'postman' | 'curl' | 'canghai'
}

/** 导入时解析出的环境（含变量），用于在导入接口的同时恢复环境变量 */
export interface ParsedEnvironment {
  id: string
  name: string
  variables: { key: string; value: string; enabled: boolean; sortOrder: number }[]
}

/** 解析后的中间结构：分类 + 请求（均与现有 SavedRequest/Category 兼容） */
export interface ParsedCollection {
  categories: Category[]
  requests: SavedRequest[]
  /** 仅 APIPost 等含环境定义的格式会产生 */
  environments?: ParsedEnvironment[]
}

function nowStr(): string {
  // 时间基准统一为 Asia/Shanghai（与 utils.now / Rust db::now_timestamp / Java BaseService.now 一致）
  return now()
}

// ============================================================
// 1. OpenAPI 3.x 解析
// ============================================================

/** 将 OpenAPI paths 展开为 SavedRequest 列表，顶层 tag 作为分类 */
export function parseOpenApi(text: string): ParsedCollection {
  const doc = JSON.parse(text)
  const categories: Category[] = []
  const requests: SavedRequest[] = []
  const paths: Record<string, any> = doc.paths || {}
  const tags: string[] = (doc.tags || []).map((t: any) => t.name)

  // 为每个 tag 预建分类
  const tagCatMap = new Map<string, string>()
  for (const tag of tags) {
    const catId = uid()
    categories.push({
      id: catId,
      name: tag,
      parentId: null,
      projectId: null,
      sortOrder: categories.length + 1,
      expanded: true,
    })
    tagCatMap.set(tag, catId)
  }

  const methods = ['get', 'post', 'put', 'delete', 'patch', 'head', 'options']
  for (const [path, item] of Object.entries(paths)) {
    for (const m of methods) {
      const op = (item as any)[m]
      if (!op) continue
      const tag = op.tags?.[0]
      const catId = tag ? (tagCatMap.get(tag) ?? null) : null
      const name = op.summary || op.operationId || `${m.toUpperCase()} ${path}`
      requests.push(openApiOpToRequest(m.toUpperCase(), path, op, catId, name))
    }
  }
  return { categories, requests }
}

function openApiOpToRequest(
  method: string,
  path: string,
  op: any,
  categoryId: string | null,
  name: string,
): SavedRequest {
  const params: KV[] = []
  const headers: KV[] = []
  for (const p of op.parameters || []) {
    const inWhere = p.in
    const kv: KV = { key: p.name, value: p.example ?? p.schema?.default ?? '', enabled: true }
    if (inWhere === 'query') params.push(kv)
    else if (inWhere === 'header') headers.push(kv)
  }
  let body = ''
  let bodyType: SavedRequest['bodyType'] = 'none'
  const reqBody = op.requestBody?.content
  if (reqBody) {
    const jsonSchema = reqBody['application/json']
    const formSchema = reqBody['application/x-www-form-urlencoded'] || reqBody['multipart/form-data']
    if (jsonSchema) {
      bodyType = 'json'
      body = schemaToExample(jsonSchema.schema)
    } else if (formSchema) {
      bodyType = 'form'
    }
  }
  const ts = nowStr()
  return {
    id: uid(),
    name,
    method: method as Method,
    url: path,
    params,
    headers,
    bodyType,
    body,
    formBody: [],
    categoryId,
    projectId: null,
    preScript: '',
    postScript: '',
    sortOrder: 0,
    createTime: ts,
    updateTime: ts,
  }
}

function schemaToExample(schema: any): string {
  if (!schema) return ''
  try {
    if (schema.example !== undefined) return JSON.stringify(schema.example, null, 2)
    if (schema.type === 'object' && schema.properties) {
      const obj: Record<string, any> = {}
      for (const [k, v] of Object.entries<any>(schema.properties)) {
        obj[k] = v.example ?? defaultForType(v.type)
      }
      return JSON.stringify(obj, null, 2)
    }
  } catch { /* ignore */ }
  return JSON.stringify(schema, null, 2)
}

function defaultForType(t?: string): any {
  switch (t) {
    case 'string': return 'string'
    case 'integer': return 0
    case 'number': return 0
    case 'boolean': return false
    case 'array': return []
    default: return null
  }
}

/** 导出为 OpenAPI 3.0 文档 */
export function exportOpenApi(categories: Category[], requests: SavedRequest[]): string {
  const paths: Record<string, any> = {}
  const tags: { name: string }[] = []
  const catNameMap = new Map(categories.map(c => [c.id, c.name]))

  for (const req of requests) {
    const url = req.url || '/'
    const method = (req.method || 'GET').toLowerCase() as Method
    if (!paths[url]) paths[url] = {}
    const op: any = {
      summary: req.name,
      responses: { '200': { description: 'Successful response' } },
    }
    if (req.categoryId && catNameMap.has(req.categoryId)) {
      const catName = catNameMap.get(req.categoryId)!
      op.tags = [catName]
      if (!tags.find(t => t.name === catName)) {
        tags.push({ name: catName })
      }
    }
    const params: any[] = []
    for (const p of req.params || []) {
      if (p.enabled && p.key) params.push({ name: p.key, in: 'query', schema: { type: 'string' }, example: p.value })
    }
    for (const h of req.headers || []) {
      if (h.enabled && h.key) params.push({ name: h.key, in: 'header', schema: { type: 'string' }, example: h.value })
    }
    if (params.length) op.parameters = params
    if (req.bodyType === 'json' && req.body) {
      op.requestBody = {
        content: { 'application/json': { schema: { type: 'object' } } },
      }
    } else if (req.bodyType === 'form' && req.formBody?.length) {
      op.requestBody = {
        content: { 'application/x-www-form-urlencoded': { schema: { type: 'object' } } },
      }
    }
    paths[url][method] = op
  }

  const doc = {
    openapi: '3.0.0',
    info: { title: 'CanghaiApi Export', version: '1.0.0' },
    tags,
    paths,
  }
  return JSON.stringify(doc, null, 2)
}

// ============================================================
// 2. Postman Collection v2.1 解析
// ============================================================

export function parsePostman(text: string): ParsedCollection {
  const doc = JSON.parse(text)
  const categories: Category[] = []
  const requests: SavedRequest[] = []
  const itemRoot: any[] = doc.item || doc.items || []

  function walk(items: any[], parentCatId: string | null) {
    let sortOrderSeed = 1
    for (const node of items) {
      if (node.item && Array.isArray(node.item)) {
        // 文件夹 → 分类
        const catId = uid()
        categories.push({
          id: catId,
          name: node.name || '未命名分类',
          parentId: parentCatId,
          projectId: null,
          sortOrder: sortOrderSeed++,
          expanded: true,
        })
        walk(node.item, catId)
      } else if (node.request) {
        requests.push(postmanItemToRequest(node, parentCatId))
      }
    }
  }
  walk(itemRoot, null)
  return { categories, requests }
}

/**
 * 读取 Postman 节点 `event` 数组里的脚本：
 * - `listen: "prerequest"` → 前置脚本
 * - `listen: "test"`       → 后置脚本（测试脚本）
 *
 * Postman 把每个事件写成 `{ listen, script: { type, exec, src } }`，其中 `exec` 通常是
 * 按行拆分的字符串数组（需重新用 `\n` 拼回），也兼容单个字符串；`disabled` 的事件跳过；
 * `script.src` 为外部文件引用，无法随集合导出携带，忽略。
 */
function postmanExtractScript(node: any, listen: 'prerequest' | 'test'): string {
  const events = node?.event
  if (!Array.isArray(events)) return ''
  const blocks: string[] = []
  for (const ev of events) {
    if (!ev || ev.disabled) continue
    const evListen = typeof ev.listen === 'string' ? ev.listen : ''
    const matched =
      listen === 'prerequest' ? evListen === 'prerequest' || evListen === 'pre-request' : evListen === listen
    if (!matched) continue
    const exec = ev.script?.exec
    const text = Array.isArray(exec) ? exec.join('\n') : typeof exec === 'string' ? exec : ''
    if (text.trim()) blocks.push(text)
  }
  return blocks.join('\n\n')
}

function postmanItemToRequest(node: any, categoryId: string | null): SavedRequest {
  const req = node.request
  const method: Method = (req.method || 'GET').toUpperCase() as Method
  const urlObj = req.url
  let url = ''
  let params: KV[] = []
  if (typeof urlObj === 'string') {
    url = urlObj
  } else if (urlObj) {
    url = typeof urlObj.raw === 'string' ? urlObj.raw : ''
    if (Array.isArray(urlObj.query)) {
      params = urlObj.query.map((q: any) => ({ key: q.key, value: q.value ?? '', enabled: true }))
    }
  }
  const headers: KV[] = []
  if (Array.isArray(req.header)) {
    for (const h of req.header) {
      if (h.disabled) continue
      headers.push({ key: h.key, value: h.value ?? '', enabled: true })
    }
  }
  let body = ''
  let bodyType: SavedRequest['bodyType'] = 'none'
  const bodyObj = req.body
  if (bodyObj?.mode === 'raw') {
    bodyType = 'json'
    body = bodyObj.raw ?? ''
  } else if (bodyObj?.mode === 'urlencoded' && Array.isArray(bodyObj.urlencoded)) {
    bodyType = 'form'
  }
  const ts = nowStr()
  return {
    id: uid(),
    name: node.name || `${method} ${url}`,
    method: method as Method,
    url,
    params,
    headers,
    bodyType,
    body,
    formBody: [],
    categoryId,
    projectId: null,
    preScript: postmanExtractScript(node, 'prerequest'),
    postScript: postmanExtractScript(node, 'test'),
    sortOrder: 0,
    createTime: ts,
    updateTime: ts,
  }
}

/** 导出为 Postman Collection v2.1 */
export function exportPostman(categories: Category[], requests: SavedRequest[]): string {
  const childrenMap = new Map<string | null, Category[]>()
  for (const c of categories) {
    const arr = childrenMap.get(c.parentId ?? null) || []
    arr.push(c)
    childrenMap.set(c.parentId ?? null, arr)
  }
  const reqsByCat = new Map<string | null, SavedRequest[]>()
  for (const r of requests) {
    const arr = reqsByCat.get(r.categoryId ?? null) || []
    arr.push(r)
    reqsByCat.set(r.categoryId ?? null, arr)
  }

  function buildItems(parentId: string | null): any[] {
    const items: any[] = []
    for (const c of childrenMap.get(parentId) || []) {
      items.push({ name: c.name, item: buildItems(c.id) })
    }
    for (const r of reqsByCat.get(parentId) || []) {
      items.push(requestToPostmanItem(r))
    }
    return items
  }

  const doc = {
    info: { name: 'CanghaiApi Export', schema: 'https://schema.getpostman.com/json/collection/v2.1.0/collection.json' },
    item: buildItems(null),
  }
  return JSON.stringify(doc, null, 2)
}

function requestToPostmanItem(r: SavedRequest): any {
  const headers = (r.headers || []).filter(h => h.enabled && h.key)
    .map(h => ({ key: h.key, value: h.value }))
  const query = (r.params || []).filter(p => p.enabled && p.key)
    .map(p => ({ key: p.key, value: p.value }))
  const item: any = {
    name: r.name,
    request: {
      method: r.method,
      header: headers,
      url: { raw: r.url, query },
    },
  }
  if (r.bodyType === 'json' && r.body) {
    item.request.body = { mode: 'raw', raw: r.body }
  } else if (r.bodyType === 'form' && r.formBody?.length) {
    item.request.body = {
      mode: 'urlencoded',
      urlencoded: r.formBody.filter(f => f.enabled && f.key).map(f => ({ key: f.key, value: f.value })),
    }
  }
  return item
}

// ============================================================
// 3. cURL 命令解析（单条）
// ============================================================

export function parseCurl(text: string): ParsedCollection {
  const req = parseSingleCurl(text)
  if (!req) return { categories: [], requests: [] }
  const ts = nowStr()
  req.id = uid()
  req.createTime = ts
  req.updateTime = ts
  return { categories: [], requests: [req] }
}

function parseSingleCurl(text: string): SavedRequest | null {
  // 去掉换行续行符
  const raw = text.replace(/\\\s*\n/g, ' ').trim()
  if (!raw.toLowerCase().startsWith('curl')) return null

  const params: KV[] = []
  const headers: KV[] = []
  let method: Method = 'GET'
  let url = ''
  let body = ''

  // 用简易 tokenizer 处理引号
  const tokens = tokenize(raw.slice(4))
  for (let i = 0; i < tokens.length; i++) {
    const t = tokens[i]
    if (t === '-X' || t === '--request') {
      method = (tokens[++i] || 'GET').toUpperCase() as Method
    } else if (t === '-H' || t === '--header') {
      const h = tokens[++i] || ''
      const idx = h.indexOf(':')
      if (idx > 0) headers.push({ key: h.slice(0, idx).trim(), value: h.slice(idx + 1).trim(), enabled: true })
    } else if (t === '-d' || t === '--data' || t === '--data-raw' || t === '--data-binary') {
      body = tokens[++i] || ''
    } else if (t === '-u' || t === '--user') {
      const u = tokens[++i] || ''
      const enc = typeof btoa !== 'undefined' ? btoa(u) : Buffer.from(u).toString('base64')
      headers.push({ key: 'Authorization', value: `Basic ${enc}`, enabled: true })
    } else if (t.startsWith('-') && t.length > 1 && t[1] !== '-') {
      // 合并短选项，如 -fsS
      i++
    } else if (t.startsWith('--')) {
      i++ // 跳过未知长选项的值
    } else if (!url && (t.startsWith('http://') || t.startsWith('https://') || t.startsWith('{{') || t.startsWith('/'))) {
      url = t
    }
  }

  // 拆分 query 到 params
  try {
    const u = new URL(url)
    u.searchParams.forEach((v, k) => params.push({ key: k, value: v, enabled: true }))
    u.search = ''
    url = u.toString()
  } catch { /* relative url，忽略 */ }

  let bodyType: SavedRequest['bodyType'] = 'none'
  if (body) {
    const ct = headers.find(h => h.key.toLowerCase() === 'content-type')?.value || ''
    bodyType = ct.includes('json') || looksJson(body) ? 'json' : 'text'
  }

  return {
    id: '',
    name: `${method} ${url || 'cURL 导入'}`,
    method,
    url,
    params,
    headers,
    bodyType,
    body,
    formBody: [],
    categoryId: null,
    projectId: null,
    preScript: '',
    postScript: '',
    sortOrder: 0,
    createTime: '',
    updateTime: '',
  }
}

function tokenize(s: string): string[] {
  const out: string[] = []
  let cur = ''
  let quote: '"' | "'" | null = null
  for (let i = 0; i < s.length; i++) {
    const c = s[i]
    if (quote) {
      if (c === quote) { quote = null }
      else { cur += c }
    } else if (c === '"' || c === "'") {
      quote = c
    } else if (c === ' ' || c === '\t' || c === '\n') {
      if (cur) { out.push(cur); cur = '' }
    } else {
      cur += c
    }
  }
  if (cur) out.push(cur)
  return out
}

function looksJson(s: string): boolean {
  const t = s.trim()
  return (t.startsWith('{') && t.endsWith('}')) || (t.startsWith('[') && t.endsWith(']'))
}

/** 单请求导出为 cURL */
export function exportCurl(req: SavedRequest): string {
  const parts: string[] = ['curl']
  if (req.method !== 'GET') parts.push(`-X ${req.method}`)
  let url = req.url
  const enabledParams = (req.params || []).filter(p => p.enabled && p.key)
  if (enabledParams.length) {
    const qs = enabledParams.map(p => `${encodeURIComponent(p.key)}=${encodeURIComponent(p.value)}`).join('&')
    url += (url.includes('?') ? '&' : '?') + qs
  }
  parts.push(`'${url}'`)
  for (const h of (req.headers || []).filter(h => h.enabled && h.key)) {
    parts.push(`-H '${h.key}: ${h.value}'`)
  }
  if (req.bodyType !== 'none' && req.body) {
    parts.push(`-d '${req.body.replace(/'/g, "'\\''")}'`)
  }
  return parts.join(' \\\n  ')
}

// ============================================================
// 4. 现有私有格式（canghai v1）解析 —— 复用 useSavedRequests 的导入入口
// ============================================================

export interface CanghaiExport {
  version: 1
  exportedAt: number
  categories: Category[]
  requests: SavedRequest[]
}

export function parseCanghai(text: string): CanghaiExport {
  return JSON.parse(text) as CanghaiExport
}

// ============================================================
// 5. APIPost 8 导出格式解析
//    结构：{ apis: [...] }，节点通过 target_id / parent_id 构建树
//    target_type: 'folder' -> 目录；'api' -> 接口
// ============================================================

interface ApipostNode {
  target_id?: string
  parent_id?: string
  target_type?: string
  name?: string
  method?: string
  url?: string
  sort?: number
  request?: ApipostRequest
}

interface ApipostKv {
  key?: string
  value?: string
  is_checked?: boolean | number
}

interface ApipostBody {
  /** body 模式：json / form-data / urlencoded / raw / binary / none */
  mode?: string
  parameter?: ApipostKv[]
  raw?: string
  raw_parameter?: ApipostKv[]
  binary?: string | Record<string, any>
  language?: string
}

interface ApipostTask {
  /** 任务类型，脚本任务为 customScript */
  type?: string
  /** 是否启用：1 启用，-1 禁用 */
  enabled?: number
  /** 脚本内容（customScript 任务） */
  data?: string
}

interface ApipostRequest {
  body?: ApipostBody
  header?: { parameter?: ApipostKv[] }
  query?: { parameter?: ApipostKv[] }
  cookie?: { parameter?: ApipostKv[] }
  pre_tasks?: ApipostTask[]
  post_tasks?: ApipostTask[]
}

function apipostKvToKV(list: ApipostKv[] | undefined): KV[] {
  if (!Array.isArray(list)) return []
  return list
    .filter(p => p && p.key !== undefined)
    .map(p => ({
      key: String(p.key ?? ''),
      value: String(p.value ?? ''),
      enabled: p.is_checked === undefined ? true : p.is_checked === true || p.is_checked === 1,
    }))
}

function apipostExtractScript(tasks: ApipostTask[] | undefined): string {
  if (!Array.isArray(tasks)) return ''
  const scripts = tasks
    // APIPost 8 的脚本任务：type=customScript，内容在 data 字段，enabled=1 表示启用
    .filter(t => t && t.type === 'customScript' && t.enabled !== -1 && t.data)
    .map(t => t.data as string)
  return scripts.join('\n\n')
}

/**
 * 解析 APIPost 8 的 global.envs：
 * env_var_list 为 { 变量名: { value, current_value, description } } 形式
 * server_list[].uri 为服务前置地址，作为 baseUrl 变量一并导入（不覆盖同名变量）
 */
function parseApipostEnvironments(global: any): ParsedEnvironment[] {
  const envs: any[] = Array.isArray(global?.envs) ? global.envs : []
  const out: ParsedEnvironment[] = []

  for (const env of envs) {
    if (!env) continue
    const variables: ParsedEnvironment['variables'] = []

    const varMap: Record<string, any> = env.env_var_list && typeof env.env_var_list === 'object'
      ? env.env_var_list
      : {}
    for (const [key, def] of Object.entries(varMap)) {
      if (!key) continue
      // value 为远程值，current_value 为本地值，两者都可能为空
      const raw = def?.value ?? def?.current_value ?? ''
      variables.push({
        key,
        value: raw == null ? '' : String(raw),
        enabled: true,
        sortOrder: variables.length + 1,
      })
    }

    // 服务前置地址：作为 baseUrl 变量，使导入的接口能直接解析 {{baseUrl}}
    const servers: any[] = Array.isArray(env.server_list) ? env.server_list : []
    const uri = servers
      .map(s => (typeof s?.uri === 'string' ? s.uri.trim() : ''))
      .find(u => !!u) || ''
    if (uri && !variables.some(v => v.key === 'baseUrl')) {
      variables.push({ key: 'baseUrl', value: uri, enabled: true, sortOrder: variables.length + 1 })
    }

    out.push({
      id: env.env_id ? String(env.env_id) : uid(),
      name: env.name || '未命名环境',
      variables,
    })
  }

  return out
}

/** 解析 APIPost 8 导出的 JSON 文件 */
export function parseApipost(text: string): ParsedCollection {
  const json = JSON.parse(text)
  const apis: ApipostNode[] = Array.isArray(json?.apis) ? json.apis : []
  const categories: Category[] = []
  const requests: SavedRequest[] = []
  const environments = parseApipostEnvironments(json?.global)

  for (const node of apis) {
    const id = node.target_id || ''
    // APIPost 顶层节点 parent_id 为 "0"，归一化为根（null）
    const parentId = !node.parent_id || node.parent_id === '0' ? null : node.parent_id

    if (node.target_type === 'folder') {
      categories.push({
        id,
        name: node.name || '未命名目录',
        parentId,
        projectId: null,
        sortOrder: node.sort ?? categories.length + 1,
        expanded: true,
      })
      continue
    }

    // api 节点
    const req = node.request || {}
    const method = (node.method || 'GET').toUpperCase() as Method
    const url = node.url || ''

    // body：APIPost 8 用 request.body.mode 标记类型（json / form-data / urlencoded / raw / binary / none）
    const bd = req.body || {}
    const formBodyList = apipostKvToKV(bd.parameter)
    const rawText = typeof bd.raw === 'string' ? bd.raw : ''
    const mode = (bd.mode || '').toLowerCase()
    // 未显式给出 mode 时按内容兜底推断
    const effectiveMode = mode || (formBodyList.length ? 'form-data' : rawText ? 'json' : 'none')

    let bodyType: BodyType = 'none'
    let body = ''
    let formBody: KV[] = []
    if (effectiveMode === 'form-data' || effectiveMode === 'urlencoded' || effectiveMode === 'x-www-form-urlencoded') {
      bodyType = 'form'
      formBody = formBodyList
    } else if (effectiveMode === 'json') {
      bodyType = 'json'
      body = rawText
    } else if (effectiveMode === 'text' || mode === 'xml' || mode === 'html' || mode === 'javascript' || mode === 'js') {
      bodyType = 'text'
      body = rawText
    } else if (effectiveMode === 'binary') {
      bodyType = 'text'
      body = typeof bd.binary === 'string' ? bd.binary : (bd.binary as any)?.path || ''
    }

    const ts = nowStr()
    requests.push({
      id,
      name: node.name || `${method} ${url}`,
      method,
      url,
      params: apipostKvToKV(req.query?.parameter),
      headers: apipostKvToKV(req.header?.parameter),
      bodyType,
      body,
      formBody,
      categoryId: parentId,
      projectId: null,
      preScript: apipostExtractScript(req.pre_tasks),
      postScript: apipostExtractScript(req.post_tasks),
      sortOrder: node.sort ?? 0,
      createTime: ts,
      updateTime: ts,
    })
  }

  return { categories, requests, environments }
}

/** 判断文本是否为 APIPost 8 导出格式 */
export function isApipost(text: string): boolean {
  try {
    const json = JSON.parse(text)
    return Array.isArray(json?.apis) && json.apis.every(
      (n: any) => n && (n.target_type === 'folder' || n.target_type === 'api' || n.target_id !== undefined)
    )
  } catch {
    return false
  }
}
