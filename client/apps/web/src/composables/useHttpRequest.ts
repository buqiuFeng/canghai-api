import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { Method, BodyType, KV, ResponseInfo } from '@/types'
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

/** 构建请求体 */
export function buildBody(
  method: Method,
  bodyType: BodyType,
  body: string,
  formBody: KV[],
): { body: string | null; contentType: string | null } {
  if (['GET', 'HEAD'].includes(method)) return { body: null, contentType: null }
  switch (bodyType) {
    case 'json': {
      if (!body) return { body: null, contentType: null }
      const cleaned = stripJsonComments(body).trim()
      return cleaned
        ? { body: cleaned, contentType: 'application/json' }
        : { body: null, contentType: null }
    }
    case 'form': {
      const params = kvToObject(formBody)
      const qs = new URLSearchParams(params).toString()
      return qs
        ? { body: qs, contentType: 'application/x-www-form-urlencoded' }
        : { body: null, contentType: null }
    }
    case 'text':
      return body
        ? { body, contentType: 'text/plain' }
        : { body: null, contentType: null }
    default:
      return { body: null, contentType: null }
  }
}

/** 发送 HTTP 请求（调用 Tauri 后端） */
export async function invokeHttpRequest(
  method: Method,
  url: string,
  headers: Record<string, string>,
  body: string | null,
): Promise<ResponseInfo> {
  const raw = await repo.send({ method, url, headers, body, allowPrivate: settings.allowPrivateAddress })
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

  const rawPromise = repo.sendStream({ method: opts.method, url: opts.url, headers, body: opts.body, stream: true, allowPrivate: settings.allowPrivateAddress })

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
