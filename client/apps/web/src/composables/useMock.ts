import { ref, onUnmounted } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { useMockRepo } from '@/repositories/mockRepo'
// 共享类型统一收敛到 @/types（Phase 8.3：打断 repository ↔ composable 循环）。
import type { MockRoute } from '@/types'

export type { MockRoute }

const repo = useMockRepo()

export interface MockLog {
  time_ms: number
  method: string
  path: string
  matched: boolean
  status: number
}

export function useMock() {
  const routes = ref<MockRoute[]>([
    { path: '/api/hello', method: 'GET', status: 200, body: '{"message":"hello"}', content_type: 'application/json' },
  ])
  const port = ref(8787)
  const running = ref(false)
  const logs = ref<MockLog[]>([])
  let unlisten: UnlistenFn | null = null

  async function ensureListener() {
    if (unlisten) return
    unlisten = await listen<MockLog>('mock-log', (e) => {
      logs.value.unshift(e.payload)
      if (logs.value.length > 200) logs.value.pop()
    })
  }

  async function start() {
    await ensureListener()
    const res = await repo.start(routes.value, port.value)
    if (!res.success) throw new Error(res.msg || '启动 Mock 服务失败')
    running.value = true
  }

  async function stop() {
    const res = await repo.stop()
    if (!res.success) throw new Error(res.msg || '停止 Mock 服务失败')
    running.value = false
  }

  function addRoute() {
    routes.value.push({ path: '/api/new', method: 'GET', status: 200, body: '{}', content_type: 'application/json' })
  }
  function removeRoute(i: number) {
    routes.value.splice(i, 1)
  }

  onUnmounted(() => {
    if (unlisten) unlisten()
  })

  return { routes, port, running, logs, start, stop, addRoute, removeRoute }
}
