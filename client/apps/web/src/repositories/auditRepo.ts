import { invokeApi } from '@/lib/server'
import type { AuditEntry, AuditQuery } from '@/types'

/**
 * 审计日志数据仓库 —— 经 Rust `call_server_api` 转发后端 `/api/v1/audit/query`（仅在线可用）。
 *
 * 统一走 `invokeApi`：由它负责 token 注入、信封拆包与错误归一。
 * 此前此处直接 `invoke<string>` 后再 `JSON.parse`，与 Rust「返回对象而非字符串」的
 * 契约变更脱节（Phase 6 已改为对象透传），且未携带 token 导致 401。
 */
export function useAuditRepo() {
  return {
    /** 查询审计日志；失败抛错（由调用方决定提示方式）。 */
    async query(opts: AuditQuery = {}): Promise<AuditEntry[] | null> {
      const data = await invokeApi<{ list?: AuditEntry[] } | null>('/api/v1/audit/query', {
        entityType: opts.entityType ?? '',
        entityId: opts.entityId ?? '',
        keyword: opts.keyword ?? '',
        page: opts.page ?? 0,
        size: opts.size ?? 50,
      })
      return data?.list ?? []
    },
  }
}
