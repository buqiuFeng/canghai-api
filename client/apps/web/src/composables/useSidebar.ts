import { ref } from 'vue'

const SIDEBAR_MIN = 160
const SIDEBAR_MAX = 420
const SIDEBAR_KEY = 'canghai-sidebar-w'

export function useSidebar() {
  const sidebarRef = ref<HTMLElement | null>(null)
  const sidebarWidth = ref<number>(
    Math.max(SIDEBAR_MIN, Math.min(SIDEBAR_MAX, Number(localStorage.getItem(SIDEBAR_KEY)) || 230))
  )

  let _resizing = false

  function startSidebarResize(e: MouseEvent) {
    e.preventDefault()
    _resizing = true
    const startX = e.clientX
    const startW = sidebarWidth.value
    document.body.style.cursor = 'col-resize'
    document.body.style.userSelect = 'none'

    const onMove = (ev: MouseEvent) => {
      if (!_resizing) return
      const delta = ev.clientX - startX
      const next = Math.max(SIDEBAR_MIN, Math.min(SIDEBAR_MAX, startW + delta))
      sidebarWidth.value = next
    }
    const onUp = () => {
      _resizing = false
      document.body.style.cursor = ''
      document.body.style.userSelect = ''
      document.removeEventListener('mousemove', onMove)
      document.removeEventListener('mouseup', onUp)
      localStorage.setItem(SIDEBAR_KEY, String(sidebarWidth.value))
    }
    document.addEventListener('mousemove', onMove)
    document.addEventListener('mouseup', onUp)
  }

  return { sidebarRef, sidebarWidth, startSidebarResize }
}
