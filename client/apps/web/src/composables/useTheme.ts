import { ref, watch } from 'vue'

/**
 * 主题（浅色 / 暗色）—— 模块级单例。
 *
 * 实现要点：
 * - 通过给 <html> 切换 `dark` class 驱动 Element Plus 暗色（其 css-vars 以 `html.dark` 为选择器），
 *   同时写入 `data-theme` 供自定义样式与调试使用；
 * - 持久化到 localStorage（`canghai-theme`），首次进入若未设置则跟随系统 `prefers-color-scheme`；
 * - `initTheme()` 需在挂载前调用，令首帧即为目标主题，避免「闪白」。
 */
export type ThemeMode = 'light' | 'dark'

const STORAGE_KEY = 'canghai-theme'

function readInitial(): ThemeMode {
  try {
    const saved = localStorage.getItem(STORAGE_KEY)
    if (saved === 'light' || saved === 'dark') return saved
  } catch {
    /* localStorage 不可用（隐私模式等）：忽略 */
  }
  if (typeof window !== 'undefined' && window.matchMedia?.('(prefers-color-scheme: dark)').matches) {
    return 'dark'
  }
  return 'light'
}

const theme = ref<ThemeMode>(readInitial())

function apply(mode: ThemeMode): void {
  if (typeof document === 'undefined') return
  const el = document.documentElement
  el.classList.toggle('dark', mode === 'dark')
  el.dataset.theme = mode
  // 让原生控件（滚动条、表单、弹窗）跟随主题
  el.style.colorScheme = mode
}

/** 应用启动时调用：把持久化/系统主题即时生效（需在 app.mount 之前） */
export function initTheme(): void {
  apply(theme.value)
}

watch(theme, (mode) => {
  apply(mode)
  try {
    localStorage.setItem(STORAGE_KEY, mode)
  } catch {
    /* ignore */
  }
})

export function useTheme() {
  function toggleTheme(): void {
    theme.value = theme.value === 'dark' ? 'light' : 'dark'
  }
  function setTheme(mode: ThemeMode): void {
    theme.value = mode
  }
  return { theme, toggleTheme, setTheme }
}
