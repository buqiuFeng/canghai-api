import { ref } from 'vue'
import { useSync } from './useSync'
import { useDataMode } from './useDataMode'
import { useAuditRepo } from '@/repositories/auditRepo'
// 共享类型统一收敛到 @/types（Phase 8.3：打断 repository ↔ composable 循环），
// 此处 re-export 兼容既有 import 路径。
import type { AuditEntry, AuditQuery } from '@/types'

export type { AuditEntry, AuditQuery }

export function useAudit() {
  const { getSyncConfig } = useSync()
  const logs = ref<AuditEntry[]>([])
  const loading = ref(false)
  const repo = useAuditRepo()

  /** 查询审计日志（需要已配置后端同步地址 + 登录 token） */
  async function query(opts: AuditQuery = {}) {
    loading.value = true
    try {
      // 离线模式下云端不可达：不再调用 call_server_api 查询审计日志。
      if (useDataMode().dataMode.value === 'offline') {
        return
      }
      const cfg = await getSyncConfig()
      if (!cfg?.serverUrl) return
      // 通过 Rust call_server_api 转发，避免 WebView 跨域 CORS，
      // Rust 侧会自动注入 mode/token 并完成鉴权。
      const list = await repo.query(opts)
      if (list) logs.value = list
    } catch {
      // 后端未部署或接口不可用，静默忽略
    } finally {
      loading.value = false
    }
  }

  return { logs, loading, query }
}
