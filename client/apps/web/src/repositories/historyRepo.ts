import { invokeUnwrap } from '@/lib/tauri'
import type { HistoryItem } from '@/types'
import { useDataModeStore } from '@/stores/dataMode'

/**
 * 请求历史数据仓库 —— 收敛历史记录的本地库读写命令。
 * 历史按在线/离线模式物理隔离（通过 dataMode 参数）。
 */
export function useHistoryRepo() {
  const mode = useDataModeStore()
  const modeArg = () => ({ dataMode: mode.dataMode })

  return {
    save(item: HistoryItem) {
      return invokeUnwrap<void>('save_history', { item, ...modeArg() })
    },
    remove(id: string) {
      return invokeUnwrap<void>('delete_history_item', { id, ...modeArg() })
    },
    clear() {
      return invokeUnwrap<void>('clear_history', { ...modeArg() })
    },
    /** 读取历史；失败返回 null（由调用方决定是否保留现有列表）。 */
    async fetchAll(): Promise<HistoryItem[] | null> {
      try {
        const list = await invokeUnwrap<HistoryItem[]>('get_history', { ...modeArg() })
        return Array.isArray(list) ? list : null
      } catch (e) {
        console.warn('[historyRepo] 读取历史失败', e)
        return null
      }
    },
    /** 历史上限（后端 `db::MAX_HISTORY` 为单一事实源）；失败返回 null 由调用方保留默认值。 */
    async fetchLimit(): Promise<number | null> {
      try {
        const n = await invokeUnwrap<number>('get_history_limit')
        return typeof n === 'number' ? n : null
      } catch {
        return null
      }
    },
  }
}
