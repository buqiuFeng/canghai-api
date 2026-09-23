import { storeToRefs } from 'pinia'
import { useDataModeStore } from '@/stores/dataMode'

export type { DataMode } from '@/stores/dataMode'

/**
 * 数据模式（在线/离线）—— 兼容包装器。
 *
 * 状态已迁入 Pinia `useDataModeStore`；此处保持原有 API 不变
 * （`dataMode` 仍是 ref，调用方 `dataMode.value` / `modeArg()` 均无需改动）。
 */
export function useDataMode() {
  const store = useDataModeStore()
  const { dataMode } = storeToRefs(store)
  return {
    dataMode,
    setMode: store.setMode,
    toggle: store.toggle,
    modeArg: store.modeArg,
  }
}
