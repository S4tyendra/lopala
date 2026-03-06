import { ref } from 'vue'
import type { AppWindow, User, ChatMessage, Channel, CanvasStroke } from '../types'

// ─── Reactive shared state (singleton pattern via module scope) ───────────────
export const myId = ref(Math.random().toString(36).substring(7))
export const myName = ref('')
export const myColor = ref('#e8f4ff')
export const showNamePrompt = ref(true)
export const myLatency = ref<number | null>(null)

export const currentWorkspace = ref(0)
export const workspaceCount = ref(4)
export const users = ref<Record<string, User>>({})
export const windows = ref<Record<string, AppWindow>>({})
export const chats = ref<ChatMessage[]>([])
export const channels = ref<Channel[]>([])

export const ws = ref<WebSocket | null>(null)

// Canvas strokes replayed from server on sync
export const canvasHistory = ref<Record<string, CanvasStroke[]>>({})

// ─── Light user color palette (readable on dark + white canvas) ───────────────
export const USER_COLORS = [
  '#60a5fa', '#34d399', '#fb923c', '#f472b6',
  '#a78bfa', '#facc15', '#22d3ee', '#f87171',
]

let reconnectTimer: ReturnType<typeof setTimeout> | null = null

function buildUrl() {
  const proto = location.protocol === 'https:' ? 'wss:' : 'ws:'
  return `${proto}//${location.host}/_ws`
}

let pingTimer: ReturnType<typeof setInterval> | null = null
let pendingPingTs: number | null = null

export function connectWs(onEvent: (msg: any) => void, callbacks?: { onOpen?: () => void; onClose?: () => void }) {
  if (ws.value && ws.value.readyState < 2) ws.value.close()

  const socket = new WebSocket(buildUrl())
  ws.value = socket

  socket.onopen = () => {
    socket.send(JSON.stringify({
      type: 'UserJoined',
      user: { id: myId.value, name: myName.value, x: 0, y: 0, workspace: currentWorkspace.value, color: '' }
    }))
    callbacks?.onOpen?.()

    // Latency ping loop — every 5s
    if (pingTimer) clearInterval(pingTimer)
    pingTimer = setInterval(() => {
      if (socket.readyState !== WebSocket.OPEN) return
      pendingPingTs = Date.now()
      socket.send(JSON.stringify({ type: 'Ping', ts: pendingPingTs }))
    }, 5000)
  }

  socket.onmessage = (e) => {
    try {
      const msg = JSON.parse(e.data)
      if (msg.type === 'Pong' && pendingPingTs !== null) {
        const latency = Date.now() - pendingPingTs
        myLatency.value = latency
        pendingPingTs = null
        // Broadcast our latency so other clients can display it
        wsSend({ type: 'LatencyBroadcast', id: myId.value, latency_ms: latency })
        return
      }
      onEvent(msg)
    } catch {}
  }

  socket.onclose = () => {
    if (pingTimer) { clearInterval(pingTimer); pingTimer = null }
    callbacks?.onClose?.()
    // Reconnect after 2s
    reconnectTimer = setTimeout(() => connectWs(onEvent, callbacks), 2000)
  }
}

export function wsSend(obj: object) {
  if (ws.value?.readyState === WebSocket.OPEN) {
    ws.value.send(JSON.stringify(obj))
  }
}
