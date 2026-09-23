import { ref, onUnmounted } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { useWebSocketRepo } from '@/repositories/wsRepo'

const repo = useWebSocketRepo()

export interface WsLogEntry {
  id: number
  direction: 'recv' | 'sent' | 'system'
  text: string
  time: string
}

export function useWebSocket() {
  const connected = ref(false)
  const sessionId = ref<string | null>(null)
  const url = ref('wss://echo.websocket.org')
  const logs = ref<WsLogEntry[]>([])
  const error = ref<string | null>(null)
  let unlisten: UnlistenFn | null = null
  let seq = 0

  async function ensureListener() {
    if (unlisten) return
    unlisten = await listen<{
      session_id: string
      direction: string
      text: string
      time_ms: number
    }>('ws-message', (e) => {
      if (sessionId.value && e.payload.session_id !== sessionId.value) return
      const dir = (e.payload.direction === 'sent'
        ? 'sent'
        : e.payload.direction === 'recv'
          ? 'recv'
          : 'system') as WsLogEntry['direction']
      logs.value.push({
        id: ++seq,
        direction: dir,
        text: e.payload.text,
        time: new Date(e.payload.time_ms).toLocaleTimeString(),
      })
    })
  }

  async function connect(targetUrl?: string, headers: [string, string][] = []) {
    error.value = null
    await ensureListener()
    try {
      const sid = await repo.connect({ url: targetUrl ?? url.value, headers })
      sessionId.value = sid
      connected.value = true
    } catch (e) {
      error.value = String(e)
      throw e
    }
  }

  async function send(message: string) {
    if (!sessionId.value) {
      error.value = '未建立连接'
      return
    }
    await repo.send(sessionId.value, message)
  }

  async function disconnect() {
    if (!sessionId.value) return
    await repo.disconnect(sessionId.value)
    connected.value = false
    sessionId.value = null
  }

  function clearLogs() {
    logs.value = []
  }

  onUnmounted(() => {
    if (unlisten) unlisten()
  })

  return { connected, sessionId, url, logs, error, connect, send, disconnect, clearLogs }
}
