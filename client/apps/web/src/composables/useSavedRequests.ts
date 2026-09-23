import { watch } from 'vue'
import { useProjects } from './useProjects'
import { useDataMode } from './useDataMode'
import type { Category } from '@/composables/useCategories'
import type { BodyType, KV, Method, SavedRequest } from '@/types'
import { uid, now } from '@/utils'
import {
  parseOpenApi,
  parsePostman,
  parseCurl,
  parseCanghai,
  parseApipost,
  isApipost,
  exportOpenApi as expOpenApi,
  exportPostman as expPostman,
  exportCurl as expCurl,
  type ParsedCollection,
  type ParsedEnvironment,
} from './useImporters'
import { isServerNewer } from './useServerApi'
import { useConflict } from './useConflict'
import { useEnvironments } from './useEnvironments'
import { useRequestRepo, type RequestVersionMeta } from '@/repositories/requestRepo'
import { savedRequests, useSavedRequestsStore } from '@/stores/savedRequests'

// 实体类型统一由 @/types 提供（Phase 6.2 #4 / 8.3：消除双份定义与 repository 反向依赖），
// 此处 re-export 兼容既有 import 路径。
export type { SavedRequest }

// 导入导出 JSON 格式
export interface ExportData {
  version: 1
  exportedAt: number
  categories: Category[]
  requests: SavedRequest[]
  /** 导入时携带的环境（含变量），来自 APIPost 等含环境定义的格式 */
  environments?: ParsedEnvironment[]
}

const { selectedProjectId } = useProjects()
const { dataMode } = useDataMode()
const repo = useRequestRepo()
const { openConflict } = useConflict()
const { createEnv, saveVariable } = useEnvironments()
// Phase 8.4：列表状态已收归 Pinia store；本 composable 只保留业务编排
const savedRequestsStore = useSavedRequestsStore()

function isOnline() {
  return dataMode.value === 'online'
}

/**
 * 用服务端返回的权威时间戳 / 版本号校准本地「同步基线」。
 *
 * `serverUpdateTime` 与 `syncVersion` 是冲突判定的本地基线（含义是「我上次看到的服务端状态」），
 * **只能来自服务端**。此前保存/更新后直接拿本地构造的 `req` 缓存，基线用的是客户端时钟
 * （且取自请求发出之前），而服务端时间是请求处理完成之后 → 服务端必然「更新」，
 * 于是「刚保存完再更新」就必然弹出冲突框（跨时区时还会差 8 小时，必错）。
 *
 * 这里只覆盖同步基线字段，内容仍以本地 `req` 为准（含服务端精简响应里没有的字段）。
 */
function applyServerStamp(
  target: SavedRequest,
  from?: { updateTime?: string; syncVersion?: number } | null,
): void {
  if (!from) return
  if (from.updateTime) {
    target.updateTime = from.updateTime
    target.serverUpdateTime = from.updateTime
  }
  if (typeof from.syncVersion === 'number') {
    target.syncVersion = from.syncVersion
  }
}

async function load() {
  try {
    savedRequests.value = await repo.fetchLocal(selectedProjectId.value ?? '')
  } catch { /* ignore */ }
}

// 切换项目时自动重新加载
watch([selectedProjectId], () => {
  load()
})

// 切换在线/离线模式时重新加载隔离数据（跳过初始触发，避免与 onMounted 重复）
watch(dataMode, () => {
  load()
}, { flush: 'post' })

function getRequestsByCategory(categoryId: string | null): SavedRequest[] {
  return savedRequests.value.filter(r => r.categoryId === categoryId)
}

async function saveRequest(data: {
  name: string
  method: string
  url: string
  params: KV[]
  headers: KV[]
  bodyType: string
  body: string
  formBody: KV[]
  categoryId: string | null
  projectId?: string | null
  preScript?: string
  postScript?: string
}): Promise<SavedRequest> {
  const currentTime = now()
  const maxOrder = Math.max(0, ...savedRequests.value
    .filter(r => r.categoryId === data.categoryId)
    .map(r => r.sortOrder ?? 0))
  const req: SavedRequest = {
    id: uid(),
    name: data.name,
    method: data.method as Method,
    url: data.url,
    params: data.params,
    headers: data.headers,
    bodyType: data.bodyType as BodyType,
    body: data.body,
    formBody: data.formBody,
    categoryId: data.categoryId,
    projectId: data.projectId ?? null,
    preScript: data.preScript ?? '',
    postScript: data.postScript ?? '',
    sortOrder: maxOrder + 1,
    createTime: currentTime,
    updateTime: currentTime,
  }
  if (isOnline()) {
    // 服务端 create 接口仅返回精简对象（不含 params/body 等），此处仍需调用以在远端落库；
    // 本地缓存必须使用原始 req（含完整字段），否则刷新后这些字段会丢失。
    const created = await repo.createServer({
      id: req.id,
      projectId: req.projectId ?? '',
      name: req.name,
      method: req.method,
      url: req.url,
      categoryId: req.categoryId,
      params: req.params,
      headers: req.headers,
      bodyType: req.bodyType,
      body: req.body,
      formBody: req.formBody,
      preScript: req.preScript,
      postScript: req.postScript,
      sortOrder: req.sortOrder,
    })
    // 用服务端落库后的时间戳/版本号校准基线，避免「刚保存完再更新」被误判为冲突
    applyServerStamp(req, created)
    // 注意：必须用原始 req（含完整字段）缓存到本地
    await repo.cacheFromServer(req, req.projectId)
    // Phase 7.5：写操作后不再全量重拉，直接就地更新本地列表
    savedRequestsStore.upsert(req)
    return req
  }
  await repo.persistLocal(req)
  savedRequestsStore.upsert(req)
  return req
}

async function updateRequest(req: SavedRequest) {
  req.updateTime = now()
  if (isOnline()) {
    const pid = req.projectId || selectedProjectId.value || ''
    const serverList = await repo.listServer(pid)
    const server = serverList.find(s => s.id === req.id)
    const local = savedRequests.value.find(r => r.id === req.id)
    if (server && isServerNewer(server.updateTime, local?.serverUpdateTime, server.syncVersion, local?.syncVersion)) {
      openConflict({
        title: `接口 - ${local?.name ?? req.name}`,
        server: server as unknown as Record<string, unknown>,
        local: req as unknown as Record<string, unknown>,
        fields: [
          { key: 'name', label: '名称' },
          { key: 'method', label: '方法' },
          { key: 'url', label: 'URL' },
          { key: 'categoryId', label: '分类' },
        ],
        onConfirm: async () => {
          const updated = await repo.updateServer({
            id: req.id,
            projectId: pid,
            name: req.name,
            method: req.method,
            url: req.url,
            categoryId: req.categoryId,
            params: req.params,
            headers: req.headers,
            bodyType: req.bodyType,
            body: req.body,
            formBody: req.formBody,
            preScript: req.preScript,
            postScript: req.postScript,
            sortOrder: req.sortOrder,
          })
          // 覆盖成功后同样以服务端返回值为新基线
          applyServerStamp(req, updated)
          await repo.cacheFromServer(req, pid)
          savedRequestsStore.upsert(req)
        },
      })
      return
    }
    const updated = await repo.updateServer({
      id: req.id,
      projectId: pid,
      name: req.name,
      method: req.method,
      url: req.url,
      categoryId: req.categoryId,
      params: req.params,
      headers: req.headers,
      bodyType: req.bodyType,
      body: req.body,
      formBody: req.formBody,
      preScript: req.preScript,
      postScript: req.postScript,
      sortOrder: req.sortOrder,
    })
    // 用服务端落库后的时间戳/版本号校准基线；内容仍以本地 req 为准，避免字段丢失
    applyServerStamp(req, updated)
    await repo.cacheFromServer(req, pid)
    savedRequestsStore.upsert(req)
    return
  }
  await repo.updateLocal(req)
  savedRequestsStore.upsert(req)
}

async function deleteRequest(id: string) {
  if (isOnline()) {
    try {
      await repo.deleteServer(id)
    } catch (e) {
      console.error('[online] 删除接口失败（已回退本地删除）:', e)
    }
  }
  await repo.deleteLocal(id).catch(() => {})
  savedRequestsStore.removeById(id)
}

// ====== 导出功能 ======
function exportRequests(categories: Category[]): ExportData {
  return {
    version: 1,
    exportedAt: Date.now(),
    categories: JSON.parse(JSON.stringify(categories)),
    requests: JSON.parse(JSON.stringify(savedRequests.value)),
  }
}

function downloadJson(data: ExportData, filename: string) {
  const json = JSON.stringify(data, null, 2)
  const blob = new Blob([json], { type: 'application/json;charset=utf-8' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = filename
  a.click()
  URL.revokeObjectURL(url)
}

/** 通用文本下载（用于 OpenAPI/Postman/cURL 导出） */
function downloadText(content: string, filename: string, mime = 'text/plain;charset=utf-8') {
  const blob = new Blob([content], { type: mime })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = filename
  a.click()
  URL.revokeObjectURL(url)
}

// ====== 标准格式导出 ======
function exportOpenApi(categories: Category[]): string {
  return expOpenApi(categories, savedRequests.value)
}
function exportPostman(categories: Category[]): string {
  return expPostman(categories, savedRequests.value)
}
function exportCurlForAll(categories: Category[]): string {
  void categories
  return savedRequests.value.map(r => expCurl(r)).join('\n\n')
}

// ====== 导入功能 ======
async function importRequests(
  data: ExportData,
  addCategory: (name: string, parentId: string | null) => Category | null,
  projectId?: string | null,
): Promise<{ imported: number; skipped: number; envImported: number; envVarImported: number }> {
  if (!data || !Array.isArray(data.requests)) {
    throw new Error('无效的导入文件格式')
  }

  let imported = 0
  let skipped = 0

  // 0. 导入环境（含环境变量），如 APIPost 的 global.envs
  let envImported = 0
  let envVarImported = 0
  for (const env of data.environments || []) {
    try {
      const created = await createEnv(env.name)
      envImported++
      for (const v of env.variables || []) {
        if (!v.key) continue
        try {
          await saveVariable(created.id, v.key, v.value)
          envVarImported++
        } catch { /* 单条变量失败不影响其余 */ }
      }
    } catch { /* 单个环境失败不影响其余导入 */ }
  }

  // 1. 导入分类（按层级顺序：先导入顶级，再导入子级）
  // 建立旧ID → 新ID 的映射
  const catIdMap = new Map<string, string>()
  const importCats = data.categories || []

  // 按层级排序：先导入 parentId 为 null 的，再导入有父级的
  const sorted = [...importCats].sort((a, b) => {
    if (!a.parentId && b.parentId) return -1
    if (a.parentId && !b.parentId) return 1
    return 0
  })

  for (const cat of sorted) {
    const newParentId = cat.parentId ? (catIdMap.get(cat.parentId) ?? null) : null
    const newCat = addCategory(cat.name, newParentId)
    if (newCat) {
      catIdMap.set(cat.id, newCat.id)
    }
  }

  // 2. 导入请求
  for (const req of data.requests) {
    // 生成新 ID 避免冲突
    const newId = uid()
    const newCatId = req.categoryId ? (catIdMap.get(req.categoryId) ?? null) : null
    const currentTime = now()
    const maxOrder = Math.max(0, ...savedRequests.value
      .filter(r => r.categoryId === newCatId)
      .map(r => r.sortOrder ?? 0))

    const newReq: SavedRequest = {
      id: newId,
      name: req.name || '未命名接口',
      method: req.method || 'GET',
      url: req.url || '',
      params: req.params || [],
      headers: req.headers || [],
      bodyType: req.bodyType || 'none',
      body: req.body || '',
      formBody: req.formBody || [],
      categoryId: newCatId,
      projectId: projectId ?? req.projectId ?? null,
      preScript: req.preScript ?? '',
      postScript: req.postScript ?? '',
      sortOrder: maxOrder + 1,
      createTime: currentTime,
      updateTime: currentTime,
    }

    try {
      await repo.persistLocal(newReq)
      savedRequests.value.push(newReq)
      imported++
    } catch {
      skipped++
    }
  }

  return { imported, skipped, envImported, envVarImported }
}

function parseImportFile(file: File): Promise<ExportData> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader()
    reader.onload = () => {
      const text = (reader.result as string) || ''
      try {
        const parsed = detectAndParse(text)
        resolve(parsed)
      } catch (e) {
        reject(new Error('文件解析失败：' + ((e as Error).message || '不是有效的 JSON 格式')))
      }
    }
    reader.onerror = () => reject(new Error('文件读取失败'))
    reader.readAsText(file)
  })
}

/**
 * 读取文件原文（不解析）。
 *
 * 在线导入把文件内容原样上传给服务端解析（见 `uploadImportFile`），
 * 前端不再重复实现一份解析逻辑；离线模式仍然走 {@link parseImportFile} 本地解析。
 */
function readImportFileText(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader()
    reader.onload = () => resolve((reader.result as string) || '')
    reader.onerror = () => reject(new Error('文件读取失败'))
    reader.readAsText(file)
  })
}

/** 自动识别文件格式：OpenAPI / Postman / cURL / 自有格式 */
function detectAndParse(text: string): ExportData {
  const trimmed = text.trim()
  // cURL：纯文本以 curl 开头
  if (trimmed.toLowerCase().startsWith('curl')) {
    const col = parseCurl(text)
    return { version: 1, exportedAt: Date.now(), categories: col.categories, requests: col.requests }
  }
  let json: any
  try {
    json = JSON.parse(trimmed)
  } catch {
    // 多行 cURL 拼接（非严格 JSON）
    if (text.includes('curl ')) {
      const col = parseCurl(text)
      return { version: 1, exportedAt: Date.now(), categories: col.categories, requests: col.requests }
    }
    throw new Error('不是有效的 JSON 格式')
  }
  if (json.openapi || (json.swagger && typeof json.swagger === 'string')) {
    const col: ParsedCollection = parseOpenApi(text)
    return { version: 1, exportedAt: Date.now(), categories: col.categories, requests: col.requests }
  }
  if (json.info?.schema?.includes('getpostman.com') || Array.isArray(json.item) || Array.isArray(json.items)) {
    const col: ParsedCollection = parsePostman(text)
    return { version: 1, exportedAt: Date.now(), categories: col.categories, requests: col.requests }
  }
  if (isApipost(text)) {
    const col: ParsedCollection = parseApipost(text)
    return {
      version: 1,
      exportedAt: Date.now(),
      categories: col.categories,
      requests: col.requests,
      environments: col.environments,
    }
  }
  // 自有格式
  const own = parseCanghai(text)
  if (own.version === 1 && Array.isArray(own.requests)) {
    return own
  }
  throw new Error('无法识别的文件格式（支持 OpenAPI / Postman / cURL / APIPost8 / CanghaiApi 备份）')
}

// Phase 8.4：移除模块级 `load()` 副作用 —— 「import 即发 RPC」使单测无法隔离，
// 也与「各视图自行加载数据」的既有约定冲突（见 AppLayout 注释）。
// 首屏由 CategoryTree / ProjectSelectView 在自身 onMounted 中显式调用 load()，
// 切换项目 / 数据模式仍由上方两个 watch 触达。

export function useSavedRequests() {
  return {
    savedRequests,
    getRequestsByCategory,
    saveRequest,
    updateRequest,
    deleteRequest,
    load,
    exportRequests,
    exportOpenApi,
    exportPostman,
    exportCurlForAll,
    downloadJson,
    downloadText,
    importRequests,
    parseImportFile,
    readImportFileText,
    listRequestVersions,
    getRequestVersionSnapshot,
    restoreRequestVersion,
  }
}

export type { RequestVersionMeta }

export async function listRequestVersions(requestId: string): Promise<RequestVersionMeta[]> {
  return repo.listVersions(requestId)
}

/** 读取指定版本的完整内容，用于版本对比。 */
export async function getRequestVersionSnapshot(requestId: string, version: number): Promise<SavedRequest> {
  return repo.getVersionSnapshot(requestId, version)
}

export async function restoreRequestVersion(requestId: string, version: number): Promise<SavedRequest> {
  return repo.restoreVersion(requestId, version)
}
