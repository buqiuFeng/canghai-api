import { ref, computed, watch } from 'vue'
import type { HistoryItem, Method, KV, BodyType } from '@/types'
import { useDataMode } from './useDataMode'
import { now } from '@/utils'
import { useHistoryRepo } from '@/repositories/historyRepo'

/**
 * 历史上限：默认值仅作为后端不可达时的兜底，真实上限由 Rust `db::MAX_HISTORY`
 * 通过 `get_history_limit` 下发（单一事实源在后端），避免两处硬编码各自漂移。
 */
const maxHistory = ref(20)
let limitLoading: Promise<void> | null = null

export function useHistory(selectedCategoryId: { value: string | null }) {
  const history = ref<HistoryItem[]>([])
  const activeHistoryId = ref<string>('')
  const { dataMode } = useDataMode()
  const repo = useHistoryRepo()

  /** 首次使用时拉取后端历史上限（幂等，仅请求一次）。 */
  function ensureHistoryLimit(): Promise<void> {
    if (!limitLoading) {
      limitLoading = repo.fetchLimit().then((n) => {
        if (n && n > 0) maxHistory.value = n
      })
    }
    return limitLoading
  }

  const filteredHistory = computed(() => {
    if (!selectedCategoryId.value) return history.value
    return history.value.filter(h => h.categoryId === selectedCategoryId.value)
  })

  function pushHistory(
    form: {
      method: Method
      url: string
      params: KV[]
      headers: KV[]
      bodyType: BodyType
      body: string
      formBody: KV[]
      preScript: string
      postScript: string
      categoryId: string | undefined
    },
    status?: number,
  ) {
    const item: HistoryItem = {
      id: `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
      method: form.method,
      url: form.url,
      params: JSON.parse(JSON.stringify(form.params)),
      headers: JSON.parse(JSON.stringify(form.headers)),
      bodyType: form.bodyType,
      body: form.body,
      formBody: JSON.parse(JSON.stringify(form.formBody)),
      preScript: form.preScript,
      postScript: form.postScript,
      createTime: now(),
      status: status ?? null,
      categoryId: form.categoryId ?? null,
    }
    history.value.unshift(item)
    if (history.value.length > maxHistory.value) history.value.length = maxHistory.value
    activeHistoryId.value = item.id
    repo.save(item).catch(() => {})
    return item
  }

  function removeHistory(idx: number) {
    const removed = history.value.splice(idx, 1)[0]
    if (removed && removed.id === activeHistoryId.value) activeHistoryId.value = ''
    if (removed) {
      repo.remove(removed.id).catch(() => {})
    }
  }

  function clearHistory() {
    history.value = []
    activeHistoryId.value = ''
    repo.clear().catch(() => {})
  }

  async function loadPersistedHistory() {
    try {
      await ensureHistoryLimit()
      const rows = await repo.fetchAll()
      if (rows) history.value = rows
    } catch {
      // ignore
    }
  }

  // 切换在线/离线模式时重新加载隔离的历史数据（跳过初始触发，避免与 onMounted 重复）
  watch(dataMode, () => {
    loadPersistedHistory()
  }, { flush: 'post' })

  return {
    history,
    activeHistoryId,
    filteredHistory,
    pushHistory,
    removeHistory,
    clearHistory,
    loadPersistedHistory,
  }
}
