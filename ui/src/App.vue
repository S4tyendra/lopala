<script setup lang="ts">
import '@fontsource/jetbrains-mono/400.css'
import '@fontsource/jetbrains-mono/500.css'
import { ref, watch, computed, onMounted, onUnmounted, nextTick } from 'vue'
import MenuBar from './components/MenuBar.vue'
import Dock from './components/Dock.vue'
import RemoteCursors from './components/RemoteCursors.vue'
import WindowFrame from './components/WindowFrame.vue'
import TerminalApp from './components/apps/TerminalApp.vue'
import FilesApp from './components/apps/FilesApp.vue'
import MessagesApp from './components/apps/MessagesApp.vue'
import CanvasApp from './components/apps/CanvasApp.vue'
import ScreenshotApp from './components/apps/ScreenshotApp.vue'
import ScreenViewApp from './components/apps/ScreenViewApp.vue'
import TaskmanagerApp from './components/apps/TaskmanagerApp.vue'
import CodeEditorApp from './components/apps/CodeEditorApp.vue'
import MediaViewerApp from './components/apps/MediaViewerApp.vue'
import HelpApp from './components/apps/HelpApp.vue'
import Notifications from './components/Notifications.vue'
import SpotlightSearch from './components/SpotlightSearch.vue'

import {
  myId, myName, myColor, windows, users, chats,
  channels, currentWorkspace, workspaceCount, wsSend, connectWs, canvasHistory,
} from './composables/useWs'
import {
  visibleWindows, sortedWindows, onDragMove, onDragEnd, focusWindow, nextZ, syncZTop, minimizedSlots,
  spawnWindow
} from './composables/useWindows'
import { checkAndInitTerminals, writeToTerminal, disposeTerminal } from './composables/useTerminals'
import { initFileState, loadFiles, applyRemoteFileState } from './composables/useFiles'
import { drawStroke, clearCanvas, disposeCanvas, drawRemoteLine } from './composables/useCanvas'
import { applyRemoteScreenshotState } from './composables/useScreenshot'
import { notif } from './composables/useNotifications'

// ─── Clock ────────────────────────────────────────────────────────────────────
const clock = ref('')
const updateClock = () => { clock.value = new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }) }

// ─── Active app for menu bar ──────────────────────────────────────────────────
const activeApp = ref('Finder')
const APP_NAMES: Record<string, string> = {
  terminal: 'Terminal', files: 'Finder', messages: 'Messages',
  canvas: 'Canvas', screenshot: 'Screenshot', screenview: 'Screen View',
  taskmanager: 'Task Manager', editor: 'Code Editor', help: 'Help',
}

// ─── WS Event Handler ─────────────────────────────────────────────────────────
const handleEvent = (msg: any) => {
  switch (msg.type) {
    case 'SyncState': {
      const s = msg.state
      // Preserve local windows (like Help dialogs) across state syncs
      const locals = Object.values(windows.value).filter(w => w.local)
      windows.value = s.windows
      for (const w of locals) windows.value[w.id] = w

      users.value = s.users
      chats.value = s.chats
      channels.value = s.channels ?? []
      workspaceCount.value = s.workspace_count
      canvasHistory.value = s.canvas_strokes ?? {}
      if (s.users[myId.value]) myColor.value = s.users[myId.value].color

      nextTick(() => {
        checkAndInitTerminals(currentWorkspace.value)
        // Show help every time user unlocks/connects unless disabled
        const hideHelp = localStorage.getItem('hide_help_forever') === 'true'
        const hostHelp = Object.values(windows.value).some(w => w.app === 'help' && w.local)
        if (!hostHelp && !hideHelp) spawnWindow('help', { title: 'Help', local: true })
      })
      break
    }
    case 'UserJoined':
      users.value[msg.user.id] = msg.user
      if (msg.user.id === myId.value) myColor.value = msg.user.color
      else notif.userJoined(msg.user.name || msg.user.id)
      break
    case 'UserLeft':
      if (users.value[msg.id]) {
        notif.userLeft(users.value[msg.id].name || msg.id)
        delete users.value[msg.id]
      }
      break
    case 'CursorMove':
      if (msg.id !== myId.value && users.value[msg.id]) {
        users.value[msg.id].x = msg.x
        users.value[msg.id].y = msg.y
        users.value[msg.id].workspace = msg.workspace
      }
      break
    case 'SpawnWindow':
      windows.value[msg.window.id] = msg.window
      syncZTop(msg.window.z)  // keep local zTop in sync with server-assigned z
      nextTick(() => {
        const w = msg.window
        if (w.app === 'terminal') checkAndInitTerminals(currentWorkspace.value)
        if (w.app === 'files') initFileState(w.id)
      })
      break
    case 'UpdateWindow':
      if (windows.value[msg.window.id]) {
        Object.assign(windows.value[msg.window.id], msg.window)
        syncZTop(msg.window.z)  // keep zTop consistent across all clients
      } else {
        windows.value[msg.window.id] = msg.window
      }
      break
    case 'CloseWindow': {
      const w = windows.value[msg.id]
      if (w?.local) return // ignore server close for local windows
      if (w?.app === 'terminal') disposeTerminal(msg.id)
      if (w?.app === 'canvas' && w.canvasId) disposeCanvas(w.canvasId)
      delete windows.value[msg.id]
      break
    }
    case 'UpdateTitle':
      if (windows.value[msg.id]) windows.value[msg.id].title = msg.title
      break
    case 'PtyOut':
      writeToTerminal(msg.id, msg.data)
      break
    case 'HistoryData':
      // Server sends this ONLY to the requesting client — replay directly
      writeToTerminal(msg.id, msg.data)
      break
    case 'ChatMsg':
      chats.value.push(msg.msg)
      break
    case 'ChannelCreated':
      if (!channels.value.find(c => c.id === msg.channel.id)) channels.value.push(msg.channel)
      break
    case 'SetWorkspaceCount':
      workspaceCount.value = msg.count
      break
    case 'CanvasDraw':
      drawStroke(msg.stroke)
      break
    case 'CanvasLiveLine':
      // Remote user live brush stroke — draw immediately
      drawRemoteLine(msg.canvas_id, msg.from, msg.to, msg.color, msg.size)
      break
    case 'CanvasClear':
      clearCanvas(msg.canvas_id)
      break
    case 'FileSync':
      applyRemoteFileState(msg.state)
      break
    case 'ScreenshotSync':
      applyRemoteScreenshotState(msg.state)
      break
    case 'LatencyBroadcast':
      if (users.value[msg.id]) users.value[msg.id].latency_ms = msg.latency_ms
      break
  }
}

// ─── Desktop initialized via 'app-login' event ────────────────────────────────

// ─── Mouse event handlers (global) ───────────────────────────────────────────
let lastCursorEmit = 0
const onGlobalMouseMove = (e: MouseEvent) => {
  const now = Date.now()
  if (now - lastCursorEmit > 40 && myName.value) {
    lastCursorEmit = now
    wsSend({ type: 'CursorMove', id: myId.value, x: e.clientX, y: e.clientY, workspace: currentWorkspace.value })
  }
  onDragMove(e)
}

const onGlobalMouseUp = () => {
  const { resizedWinId } = onDragEnd()
  // Terminal fit happens via ResizeObserver automatically
}

const onKeyDown = (e: KeyboardEvent) => {
  const mod = e.metaKey || e.ctrlKey
  if (mod && e.key === 't') { e.preventDefault(); wsSend({ type: 'SpawnWindow', window: { id: Math.random().toString(36).substring(7), app: 'terminal', workspace: currentWorkspace.value, x: 100, y: 80, w: 700, h: 460, z: nextZ(), minimized: false, maximized: false, title: 'Terminal' } }) }
}

// ─── Window-focus tracking for menu bar active app name ─────────────────────
watch(visibleWindows, (wins) => {
  if (!wins.length) { activeApp.value = 'Finder'; return }
  const top = wins[wins.length - 1]
  activeApp.value = APP_NAMES[top?.app] ?? ''
})

// ─── Workspace change: re-init terminals ─────────────────────────────────────
watch(currentWorkspace, (ws) => {
  nextTick(() => checkAndInitTerminals(ws))
})

// ─── Handle opening a channel in messages window ─────────────────────────────
const openChannel = (winId: string, channelId: string) => {
  const win = windows.value[winId]
  if (!win) return
  const ch = channels.value.find(c => c.id === channelId)
  win.channel = channelId
  win.title = `Messages — ${ch?.name ?? channelId}`
  wsSend({ type: 'UpdateWindow', window: win })
}

// ─── Lifecycle ────────────────────────────────────────────────────────────────
let clockTimer: ReturnType<typeof setInterval>
onMounted(() => {
  updateClock()
  clockTimer = setInterval(updateClock, 15000)
  window.addEventListener('mousemove', onGlobalMouseMove)
  window.addEventListener('mouseup', onGlobalMouseUp)
  window.addEventListener('keydown', onKeyDown)

  window.addEventListener('app-login', () => {
    myName.value = (window as any).lopalaName || `user_${myId.value.substring(0, 4)}`
    connectWs(handleEvent, {
      onOpen: () => {
        notif.wsConnected()
        window.dispatchEvent(new Event("ws-connected"))
      },
      onClose: () => {
        notif.wsDisconnected()
        window.dispatchEvent(new Event("ws-error"))
      },
    })
  })
})

onUnmounted(() => {
  clearInterval(clockTimer)
  window.removeEventListener('mousemove', onGlobalMouseMove)
  window.removeEventListener('mouseup', onGlobalMouseUp)
  window.removeEventListener('keydown', onKeyDown)
})
</script>

<template>
  <!-- ── Desktop ─────────────────────────────────────────────────────────────── -->
  <div class="fixed inset-0 overflow-hidden select-none" style="background:url('/bg.svg') center/cover">
    <!-- Menu Bar (z: 2147483640) -->
    <MenuBar :clock="clock" :active-app="activeApp" />

    <!-- Remote Cursors (z: 2147483647 — topmost) -->
    <RemoteCursors />

    <!-- Windows layer -->
    <div class="absolute inset-0 top-7 bottom-20">
      <WindowFrame v-for="win in sortedWindows" :key="win.id" :win="win" v-show="win.workspace === currentWorkspace"
        @mousedown="focusWindow(win.id)">
        <TerminalApp v-if="win.app === 'terminal'" :win-id="win.id" />
        <FilesApp v-else-if="win.app === 'files'" :win-id="win.id" />
        <MessagesApp v-else-if="win.app === 'messages'" :win-id="win.id" :channel="win.channel ?? 'global'"
          @open-channel="(ch) => openChannel(win.id, ch)" />
        <CanvasApp v-else-if="win.app === 'canvas'" :win-id="win.id" :canvas-id="win.canvasId ?? win.id" :win-w="win.w"
          :win-h="win.h" />
        <ScreenshotApp v-else-if="win.app === 'screenshot'" :win-id="win.id" />
        <ScreenViewApp v-else-if="win.app === 'screenview'" :win-id="win.id" />
        <TaskmanagerApp v-else-if="win.app === 'taskmanager'" :win-id="win.id" />
        <CodeEditorApp v-else-if="win.app === 'editor'" :win-id="win.id" />
        <HelpApp v-else-if="win.app === 'help'" :win-id="win.id" />
        <MediaViewerApp v-else-if="win.app === 'media'" :win-id="win.id" />
        <div v-else class="p-8 text-center text-white/20">Unknown App: {{ win.app }}</div>
      </WindowFrame>
    </div>

    <!-- Notifications (z: 2147483645 — just below cursor layer) -->
    <Notifications />

    <!-- Spotlight search (Ctrl+K / Cmd+K) -->
    <SpotlightSearch />

    <!-- Dock (z: 2147483630) -->
    <Dock />
  </div>
</template>

<style>
*,
*::before,
*::after {
  box-sizing: border-box;
}

.emoji,
[data-emoji] {
  font-family: 'Apple Color Emoji', 'Segoe UI Emoji', 'Noto Color Emoji', emoji;
}

@keyframes winEnter {
  from {
    opacity: 0;
    transform: scale(0.96) translateY(10px);
  }

  to {
    opacity: 1;
    transform: scale(1) translateY(0);
  }
}

@keyframes popIn {
  from {
    opacity: 0;
    transform: scale(0.95);
  }

  to {
    opacity: 1;
    transform: scale(1);
  }
}

::selection {
  background: rgba(0, 122, 255, 0.3);
}

::-webkit-scrollbar {
  width: 5px;
  height: 5px;
}

::-webkit-scrollbar-track {
  background: transparent;
}

::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.1);
  border-radius: 10px;
  transition: background 0.2s var(--ease-out);
}

::-webkit-scrollbar-thumb:hover {
  background: rgba(255, 255, 255, 0.2);
}

/* Base button active state */
button:active {
  transform: scale(0.97);
  transition: transform 150ms var(--ease-out);
}

/* xterm overrides */
.xterm-viewport::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.12);
}
</style>
