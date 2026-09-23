import '@/stores/pinia'
import { defineStore } from 'pinia'
import { ref } from 'vue'

export interface ConflictField {
  /** 数据字段名 */
  key: string
  /** 展示用中文名 */
  label: string
}

export interface ConflictInfo {
  /** 弹窗标题，例如「接口冲突 - xxx」 */
  title: string
  /** 服务器最新版本（左侧只读展示） */
  server: Record<string, unknown>
  /** 本地正在编辑、准备保存的版本（右侧展示） */
  local: Record<string, unknown>
  /** 需要对比的字段 */
  fields: ConflictField[]
  /** 用户确认「仍要保存」后执行的保存逻辑（直连后端） */
  onConfirm: () => Promise<void> | void
}

/**
 * 编辑冲突弹窗的全局状态（原 `composables/useConflict.ts` 的模块级单例）。
 * 在线模式下编辑「接口 / 接口分类 / 环境变量」时，若服务器数据比本地缓存的
 * serverUpdateTime 更新，则弹对比框确认后再保存。`useConflict()` 保留同名 API。
 */
export const useConflictStore = defineStore('conflict', () => {
  const conflict = ref<ConflictInfo | null>(null)
  /** onConfirm 执行中的加载态 */
  const resolving = ref(false)

  function openConflict(info: ConflictInfo) {
    conflict.value = info
  }
  function closeConflict() {
    if (resolving.value) return
    conflict.value = null
  }
  function setResolving(v: boolean) {
    resolving.value = v
  }

  return { conflict, resolving, openConflict, closeConflict, setResolving }
})
