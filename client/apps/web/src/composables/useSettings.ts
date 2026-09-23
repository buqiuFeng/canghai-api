import { useSettingsStore } from '@/stores/settings'

export type { GlobalSettings } from '@/stores/settings'

/**
 * 全局设置 —— 兼容包装器。
 *
 * 状态已迁入 Pinia `useSettingsStore`；此处保持原有导出名不变：
 * - 其它模块仍可 `import { settings, loadSettings, saveSettings } from '@/composables/useSettings'`；
 * - `settings` 仍为 reactive 对象（`settings.serverUrl` 直接读写）。
 */
const store = useSettingsStore()

export const settings = store.settings
export const loadSettings = store.loadSettings
export const saveSettings = store.saveSettings

export function useSettings() {
  return {
    settings: store.settings,
    loadSettings: store.loadSettings,
    saveSettings: store.saveSettings,
  }
}
