import '@/stores/pinia'
import { defineStore } from 'pinia'
import { ref } from 'vue'

export type DataMode = 'online' | 'offline'

const STORAGE_KEY = 'canghai.dataMode'

function readInitial(): DataMode {
  try {
    const v = localStorage.getItem(STORAGE_KEY)
    if (v === 'offline' || v === 'online') return v
  } catch {
    /* ignore */
  }
  return 'online'
}

/**
 * 数据模式（在线/离线）集中状态。
 * 原先散落在 `composables/useDataMode.ts` 的模块级单例，现收敛为 Pinia store；
 * `useDataMode()` 保留为兼容包装器，调用方无需改动。
 */
export const useDataModeStore = defineStore('dataMode', () => {
  const dataMode = ref<DataMode>(readInitial())

  function setMode(mode: DataMode) {
    if (dataMode.value === mode) return
    dataMode.value = mode
    try {
      localStorage.setItem(STORAGE_KEY, mode)
    } catch {
      /* ignore */
    }
  }

  function toggle() {
    setMode(dataMode.value === 'online' ? 'offline' : 'online')
  }

  // 作为参数透传给所有 Tauri 命令，实现在线/离线物理库隔离。
  // 注意：Tauri 命令参数在 JS 端用 camelCase（自动映射 Rust 端 snake_case，如 dataMode -> data_mode）。
  function modeArg() {
    return { dataMode: dataMode.value }
  }

  return { dataMode, setMode, toggle, modeArg }
})
