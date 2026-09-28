import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { Method, BodyType, KV, ResponseInfo, FormDataPart } from '@/types'
import { kvToObject, stripJsonComments } from '@/utils'
import { settings } from '@/composables/useSettings'
import { useHttpRepo } from '@/repositories/httpRepo'

const repo = useHttpRepo()

/** 构建 URL（拼接 query params） */
export function buildUrl(url: string, params: KV[]): string {
  const base = url.trim()
  if (!base) throw new Error('请输入请求 URL')
  const entries = Object.entries(kvToObject(params))
  if (!entries.length) return base
  const qs = entries.map(([k, v]) => `${encodeURIComponent(k)}=${encodeURIComponent(v)}`).join('&')
  return base.includes('?') ? `${base}&${qs}` : `${base}?${qs}`
}

/** 多部件表单（multipart/form-data）的单个字段（与 Rust `FormPart` 对应）。 */
export interface MultipartPart {
  /** 字段名 */
  name: string
  /** 文本值（非文件字段） */
  value?: string
  /** 文件名（文件字段，如 'a.png'） */
  filename?: string
  /** 文件 MIME（文件字段） */
  contentType?: string
  /** 文件内容（base64，不含 data: 前缀；文件字段） */
  data?: string
}

/** 构建请求体 */
export function buildBody(
  method: Method,
  bodyType: BodyType,
  body: string,
  formBody: KV[],
  formData?: FormDataPart[],
): { body: string | null; contentType: string | null; multipart: MultipartPart[] | null } {
  if (['GET', 'HEAD'].includes(method)) return { body: null, contentType: null, multipart: null }
  switch (bodyType) {
    case 'json': {
      if (!body) return { body: null, contentType: null, multipart: null }
      const cleaned = stripJsonComments(body).trim()
      return cleaned
        ? { body: cleaned, contentType: 'application/json', multipart: null }
        : { body: null, contentType: null, multipart: null }
    }
    case 'form': {
      const params = kvToObject(formBody)
      const qs = new URLSearchParams(params).toString()
      return qs
        ? { body: qs, contentType: 'application/x-www-form-urlencoded', multipart: null }
        : { body: null, contentType: null, multipart: null }
    }
    case 'formdata': {
      // 仅取「启用且字段名非空」的部件；文件字段需有文件名（内容可空，允许上传空文件）
      const parts: MultipartPart[] = (formData ?? [])
        .filter(p => p.enabled && p.key.trim())
        .map(p => {
          if (p.type === 'file') {
            return {
              name: p.key.trim(),
              filename: p.fileName || p.key.trim(),
              contentType: p.fileMime || undefined,
              data: p.fileData || undefined,
            }
          }
          return { name: p.key.trim(), value: p.value ?? '' }
        })
      // 返回 multipart，由 Rust 端组装边界；此处不设置 Content-Type（reqwest 自动带 boundary）
      return { body: null, contentType: null, multipart: parts }
    }
    case 'text':
      return body
        ? { body, contentType: 'text/plain', multipart: null }
        : { body: null, contentType: null, multipart: null }
    default:
      return { body: null, contentType: null, multipart: null }
  }
}

/** 发送 HTTP 请求（调用 Tauri 后端） */
export async function invokeHttpRequest(
  method: Method,
  url: string,
  headers: Record<string, string>,
  body: string | null,
  multipart?: MultipartPart[] | null,
): Promise<ResponseInfo> {
  const raw = await repo.send({ method, url, headers, body, multipart: multipart ?? undefined, allowPrivate: settings.allowPrivateAddress })
  if (!raw.success) throw new Error(raw.msg || '请求失败')
  const data = raw.data
  const headerList = data.headers.map(([k, v]) => ({ key: k, value: v }))
  const ct = headerList.find(h => h.key.toLowerCase() === 'content-type')?.value || ''
  return {
    status: data.status,
    statusText: data.statusText,
    timeMs: data.timeMs,
    size: data.size,
    headers: headerList,
    headerCount: headerList.length,
    body: data.body,
    contentType: ct,
  }
}

export interface StreamOptions {
  method: Method
  url: string
  headers: Record<string, string>
  body: string | null
  /** 多部件表单（multipart/form-data），与 invokeHttpRequest 同语义 */
  multipart?: MultipartPart[] | null
  /** 每块增量回调：text 为该块数据（SSE 已解析为 data 内容），done=true 表示结束 */
  onChunk: (text: string, done: boolean, isSse: boolean) => void
  signal?: AbortSignal
}

/**
 * 流式发送 HTTP 请求（SSE / 大响应体逐步展示）。
 * 通过监听后端推送的 `http-stream-chunk` 事件实时累积内容，最终返回完整响应。
 */
export async function invokeHttpRequestStream(opts: StreamOptions): Promise<ResponseInfo> {
  const requestId = `stream-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`
  const headers = { ...opts.headers, 'x-canghai-request-id': requestId }

  let unlisten: UnlistenFn | null = null
  let acc = ''

  const donePromise = new Promise<void>((resolve) => {
    listen<{ requestId: string; data: string; sse: boolean; done: boolean }>(
      'http-stream-chunk',
      (e) => {
        const p = e.payload
        if (p.requestId !== requestId) return
        if (p.data) {
          acc += p.data
          opts.onChunk(p.data, false, p.sse)
        }
        if (p.done) {
          if (unlisten) unlisten()
          resolve()
        }
      },
    ).then((fn) => {
      unlisten = fn
    })
  })

  const rawPromise = repo.sendStream({ method: opts.method, url: opts.url, headers, body: opts.body, multipart: opts.multipart ?? undefined, stream: true, allowPrivate: settings.allowPrivateAddress })

  const [raw] = await Promise.all([rawPromise, donePromise])
  if (!raw.success) throw new Error(raw.msg || '请求失败')
  const data = raw.data

  const headerList = data.headers.map(([k, v]) => ({ key: k, value: v }))
  const ct = headerList.find(h => h.key.toLowerCase() === 'content-type')?.value || ''
  return {
    status: data.status,
    statusText: data.statusText,
    timeMs: data.timeMs,
    size: data.size,
    headers: headerList,
    headerCount: headerList.length,
    body: data.body,
    contentType: ct,
  }
}
