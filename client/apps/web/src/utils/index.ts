import type { KV } from '@/types'

/** 生成唯一 ID */
export function uid(): string {
  return Date.now().toString(36) + '-' + Math.random().toString(36).slice(2, 8)
}

/** 应用统一时区：Asia/Shanghai（UTC+8，固定偏移，无夏令时） */
export const APP_TZ_OFFSET_MS = 8 * 60 * 60 * 1000

/**
 * 当前时间（**Asia/Shanghai**，YYYY-MM-DD HH:mm:ss），全前端统一实现。
 *
 * 三端时间基准统一为北京时间（与 Java `BaseService.now()`、Rust `db::now_timestamp()` 一致）。
 * 实现按固定 +8 偏移换算后再取 UTC 字段，因此**与本机系统时区无关**：
 * 若改用 `getFullYear()/getHours()` 这类本地取法，在非中国时区的机器上会写入另一套
 * 基准的时间，破坏同步合并与冲突判定所依赖的字符串比较。
 * 展示时须经 {@link parseServerTime} 解析（该函数按 +08:00 解读存储值）。
 */
export function now(): string {
  const d = new Date(Date.now() + APP_TZ_OFFSET_MS)
  const pad = (n: number) => n.toString().padStart(2, '0')
  return `${d.getUTCFullYear()}-${pad(d.getUTCMonth() + 1)}-${pad(d.getUTCDate())} ${pad(d.getUTCHours())}:${pad(d.getUTCMinutes())}:${pad(d.getUTCSeconds())}`
}

/**
 * 将服务端时间字符串（`YYYY-MM-DD HH:mm:ss`，基准 **Asia/Shanghai**）解析为毫秒时间戳。
 *
 * `new Date('YYYY-MM-DD HH:mm:ss')` 会按**本机时区**解释裸字符串，而存储基准固定为 UTC+8，
 * 故此处显式补 `+08:00`；已带时区标识（`Z` / `±hh:mm`）的字符串直接解析。
 */
export function parseServerTime(s?: string | null): number {
  if (!s) return NaN
  const normalized = s.includes('T') ? s : s.replace(' ', 'T')
  if (/([zZ]|[+-]\d{2}:?\d{2})$/.test(normalized)) return Date.parse(normalized)
  return Date.parse(normalized + '+08:00')
}

/** 判断字符串是否类似 JSON */
export function looksLikeJson(s: string): boolean {
  const t = s.trim()
  return (t.startsWith('{') && t.endsWith('}')) || (t.startsWith('[') && t.endsWith(']'))
}

/** HTTP 状态码对应的标签类型 */
export function statusTagTypeOf(status?: number): 'success' | 'warning' | 'danger' | 'info' {
  if (!status) return 'info'
  if (status >= 200 && status < 300) return 'success'
  if (status >= 300 && status < 400) return 'warning'
  if (status >= 400) return 'danger'
  return 'info'
}

/** HTTP 方法对应的标签类型 */
export function methodTagType(m: string): 'success' | 'primary' | 'warning' | 'danger' | 'info' {
  switch (m) {
    case 'GET': return 'success'
    case 'POST': return 'primary'
    case 'PUT': return 'warning'
    case 'PATCH': return 'warning'
    case 'DELETE': return 'danger'
    default: return 'info'
  }
}

/** 格式化文件大小 */
export function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(2)} KB`
  return `${(bytes / 1024 / 1024).toFixed(2)} MB`
}

/**
 * 格式化时间戳为 `MM-DD HH:mm:ss`。
 *
 * 按应用基准 Asia/Shanghai 渲染（同样用「+8 偏移后取 UTC 字段」的写法），
 * 因此与存储值一致、且不随查看者所在时区变化。非法输入返回 `-`。
 */
export function formatTime(ts: number): string {
  if (!Number.isFinite(ts)) return '-'
  const d = new Date(ts + APP_TZ_OFFSET_MS)
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${pad(d.getUTCMonth() + 1)}-${pad(d.getUTCDate())} ${pad(d.getUTCHours())}:${pad(d.getUTCMinutes())}:${pad(d.getUTCSeconds())}`
}

/** KV 列表转对象（仅保留 enabled 且 key 非空的项） */
export function kvToObject(list: KV[]): Record<string, string> {
  const out: Record<string, string> = {}
  for (const item of list) {
    if (item.enabled && item.key.trim()) out[item.key.trim()] = item.value
  }
  return out
}

/** 缩短 URL 用于显示 */
export function shortenUrl(url: string): string {
  try {
    const u = new URL(url)
    return u.hostname + (u.pathname !== '/' ? u.pathname : '')
  } catch {
    return url.length > 30 ? url.slice(0, 30) + '…' : url
  }
}

/** 剥离 JSON 中的 // 和 /* 注释，保留字符串内的内容 */
export function stripJsonComments(input: string): string {
  let result = ''
  let i = 0
  const len = input.length
  while (i < len) {
    const ch = input[i]
    // 字符串字面量：原样复制直到关闭引号
    if (ch === '"') {
      result += ch
      i++
      while (i < len) {
        const c = input[i]
        result += c
        if (c === '\\') {
          i++
          if (i < len) { result += input[i]; i++ }
        } else if (c === '"') {
          i++
          break
        } else {
          i++
        }
      }
      continue
    }
    // 单行注释 //
    if (ch === '/' && i + 1 < len && input[i + 1] === '/') {
      i += 2
      while (i < len && input[i] !== '\n') i++
      if (i < len) { result += '\n'; i++ }
      continue
    }
    // 多行注释 /* ... */
    if (ch === '/' && i + 1 < len && input[i + 1] === '*') {
      i += 2
      while (i < len) {
        if (input[i] === '*' && i + 1 < len && input[i + 1] === '/') {
          i += 2
          break
        }
        i++
      }
      continue
    }
    result += ch
    i++
  }
  return result
}
