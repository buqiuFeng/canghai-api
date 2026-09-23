import { storeToRefs } from 'pinia'
import { useConflictStore } from '@/stores/conflict'

export type { ConflictField, ConflictInfo } from '@/stores/conflict'

/**
 * 编辑冲突弹窗状态 —— 兼容包装器。
 * 状态已迁入 Pinia `useConflictStore`；此处保持返回 `conflict`/`resolving`（ref）与动作函数。
 */
export function useConflict() {
  const store = useConflictStore()
  const { conflict, resolving } = storeToRefs(store)
  return {
    conflict,
    resolving,
    openConflict: store.openConflict,
    closeConflict: store.closeConflict,
    setResolving: store.setResolving,
  }
}
