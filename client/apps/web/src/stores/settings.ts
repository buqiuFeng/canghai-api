import '@/stores/pinia'
import { defineStore } from 'pinia'
import { reactive } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { DEFAULT_SERVER_URL } from '@/types'
import { invokeUnwrap } from '@/lib/tauri'
import { syncServerUrl } from '@/stores/session'

export interface GlobalSettings {
  serverUrl: string
  /** 是否允许发送请求到私有/保留/内网地址（如 172.16.x.x、192.168.x.x、127.0.0.1）。
   *  API 调试工具常需访问内网服务，开启后跳过内网拦截（云元数据仍强制拦截）。默认关闭。 */
  allowPrivateAddress: boolean
}

const SETTINGS_KEY = 'canghai-global-settings'

/** 同步从 localStorage 读取初始值 */
function loadInitialSettings(): GlobalSettings {
  try {
    const raw = localStorage.getItem(SETTINGS_KEY)
    if (raw) {
      const saved = JSON.parse(raw) as Partial<GlobalSettings>
      return {
        serverUrl: saved.serverUrl || DEFAULT_SERVER_URL,
        allowPrivateAddress: !!saved.allowPrivateAddress,
      }
    }
  } catch {
    /* ignore */
  }
  return { serverUrl: DEFAULT_SERVER_URL, allowPrivateAddress: false }
}

/**
 * 全局设置集中状态（原 `composables/useSettings.ts` 的模块级单例）。
 * `useSettings.ts` 保持导出 `settings`/`loadSettings`/`saveSettings` 同名，调用方无需改动。
 */
export const useSettingsStore = defineStore('settings', () => {
  const settings = reactive<GlobalSettings>(loadInitialSettings())

  /**
   * 加载全局设置（异步，从 Tauri 持久化配置补充）。
   *
   * 复用启动期 `initAuth` 已读到的 `syncServerUrl` 快照：否则刷新页面时
   * `App.vue::initAuth` / 本函数 / `AppLayout::loadSyncConfig` 会对同一条
   * `get_sync_config` 连发 3 次 IPC。
   */
  async function loadSettings(): Promise<GlobalSettings> {
    if (syncServerUrl.value) {
      settings.serverUrl = syncServerUrl.value
      return { ...settings }
    }
    try {
      const cfg = await invokeUnwrap<{ serverUrl?: string }>('get_sync_config')
      if (cfg?.serverUrl) {
        settings.serverUrl = cfg.serverUrl
        syncServerUrl.value = cfg.serverUrl
      }
    } catch {
      /* tauri not available */
    }
    return { ...settings }
  }

  /** 保存全局设置 */
  async function saveSettings(s: GlobalSettings): Promise<void> {
    settings.serverUrl = s.serverUrl
    settings.allowPrivateAddress = s.allowPrivateAddress
    localStorage.setItem(
      SETTINGS_KEY,
      JSON.stringify({ serverUrl: s.serverUrl, allowPrivateAddress: s.allowPrivateAddress }),
    )

    // 同步写入 Tauri 配置：**只上报本次真正修改的字段**。
    // Rust 侧 `set_sync_config` 是「读-合并-写」（仅非空字段覆盖），因此这里不再需要
    // 「先 get_sync_config 读出 teamId/authToken 再原样写回」——那种读改写正是最容易
    // 把登录态写坏的方式：一旦读到的令牌为空（钥匙串暂不可用等），回写就会抹掉已落盘令牌。
    try {
      await invoke('set_sync_config', {
        config: {
          serverUrl: s.serverUrl,
          allowPrivateAddress: s.allowPrivateAddress,
        },
      })
    } catch {
      /* ignore */
    }
  }

  return { settings, loadSettings, saveSettings }
})
