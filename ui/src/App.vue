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
import Notifications from './components/Notifications.vue'

import {
  myId, myName, myColor, showNamePrompt, windows, users, chats,
  channels, currentWorkspace, workspaceCount, wsSend, connectWs, canvasHistory,
} from './composables/useWs'
import { visibleWindows, onDragMove, onDragEnd, focusWindow, nextZ, syncZTop, minimizedSlots } from './composables/useWindows'
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
}

// ─── WS Event Handler ─────────────────────────────────────────────────────────
const handleEvent = (msg: any) => {
  switch (msg.type) {
    case 'SyncState': {
      const s = msg.state
      windows.value = s.windows
      users.value = s.users
      chats.value = s.chats
      channels.value = s.channels ?? []
      workspaceCount.value = s.workspace_count
      canvasHistory.value = s.canvas_strokes ?? {}
      if (s.users[myId.value]) myColor.value = s.users[myId.value].color
      nextTick(() => checkAndInitTerminals(currentWorkspace.value))
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
        if (w.app === 'files') initFileState()
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
  }
}

// ─── Join ─────────────────────────────────────────────────────────────────────
const nameInput = ref('')
const join = () => {
  myName.value = nameInput.value.trim() || `user_${myId.value.substring(0, 4)}`
  showNamePrompt.value = false
  connectWs(handleEvent, {
    onOpen: () => notif.wsConnected(),
    onClose: () => notif.wsDisconnected(),
  })
}

// ─── Mouse event handlers (global) ───────────────────────────────────────────
let lastCursorEmit = 0
const onGlobalMouseMove = (e: MouseEvent) => {
  const now = Date.now()
  if (now - lastCursorEmit > 40 && !showNamePrompt.value) {
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
  activeApp.value = APP_NAMES[top?.app] ?? 'Finder'
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
})

onUnmounted(() => {
  clearInterval(clockTimer)
  window.removeEventListener('mousemove', onGlobalMouseMove)
  window.removeEventListener('mouseup', onGlobalMouseUp)
  window.removeEventListener('keydown', onKeyDown)
})
</script>

<template>
  <!-- ── Name Prompt ─────────────────────────────────────────────────────────── -->
  <div v-if="showNamePrompt" class="fixed inset-0 flex items-center justify-center"
    style="z-index:2147483646;background:rgba(0,0,0,0.65);backdrop-filter:blur(8px)">
    <div class="rounded-2xl p-8 w-[360px] shadow-2xl"
      style="background:rgba(28,28,32,0.96);border:1px solid rgba(255,255,255,0.12);animation:winEnter 250ms cubic-bezier(0.23,1,0.32,1) both">
      <h2 class="text-white font-semibold text-[18px] mb-1">Welcome to Lopala</h2>
      <p class="text-[13px] mb-6" style="color:rgba(255,255,255,0.4)">Enter a display name to join the shared workspace.
      </p>
      <input v-model="nameInput" @keyup.enter="join" placeholder="Your name" autofocus
        class="w-full rounded-xl px-4 py-3 text-[14px] text-white outline-none mb-4 transition-[box-shadow] duration-150"
        style="background:rgba(255,255,255,0.07);border:1px solid rgba(255,255,255,0.1)"
        @focus="(e) => (e.target as HTMLInputElement).style.boxShadow = '0 0 0 2px #0a84ff66'"
        @blur="(e) => (e.target as HTMLInputElement).style.boxShadow = ''" />
      <button @click="join"
        class="w-full py-3 rounded-xl text-white font-semibold text-[14px] transition-[filter] duration-150 hover:brightness-110 active:scale-[0.97]"
        style="background:#0a84ff">
        Join Session
      </button>
    </div>
  </div>

  <!-- ── Desktop ─────────────────────────────────────────────────────────────── -->
  <div class="fixed inset-0 overflow-hidden select-none" style="background:url('/bg.svg') center/cover">
    <!-- Menu Bar (z: 2147483640) -->
    <MenuBar :clock="clock" :active-app="activeApp" />

    <!-- Remote Cursors (z: 2147483647 — topmost) -->
    <RemoteCursors />

    <!-- Windows layer -->
    <div class="absolute inset-0 top-7 bottom-20">
      <WindowFrame v-for="win in visibleWindows" :key="win.id" :win="win" @mousedown="focusWindow(win.id)">
        <TerminalApp v-if="win.app === 'terminal'" :win-id="win.id" />
        <FilesApp v-else-if="win.app === 'files'" :win-id="win.id" />
        <MessagesApp v-else-if="win.app === 'messages'" :win-id="win.id" :channel="win.channel ?? 'global'"
          @open-channel="(ch) => openChannel(win.id, ch)" />
        <CanvasApp v-else-if="win.app === 'canvas'" :win-id="win.id" :canvas-id="win.canvasId ?? win.id" :win-w="win.w"
          :win-h="win.h" />
        <ScreenshotApp v-else-if="win.app === 'screenshot'" :win-id="win.id" />
        <ScreenViewApp v-else-if="win.app === 'screenview'" :win-id="win.id" />
      </WindowFrame>
    </div>

    <!-- Notifications (z: 2147483645 — just below cursor layer) -->
    <Notifications />

    <!-- Dock (z: 2147483630) -->
    <Dock />
  </div>
</template>

<style>
/* ── Samsung emoji font ─────────────────────────────────────────────────────── */
@font-face {
  font-family: 'SamsungOneUI';
  src: url('/fonts/SamsungOneUI.ttf') format('truetype');
  font-display: swap;
}

@font-face {
  font-family: 'JetBrains Mono Nerd Font';
  src: url('/fonts/JetBrainsMonoNerdFont.ttf') format('truetype');
  font-display: swap;
}

*,
*::before,
*::after {
  box-sizing: border-box;
}

body {
  margin: 0;
  overflow: hidden;
  font-family: -apple-system, BlinkMacSystemFont, "SF Pro Display", "Inter", "Segoe UI", sans-serif;
  -webkit-font-smoothing: antialiased;
}

/* Emoji elements use Samsung font first */
.emoji,
[data-emoji] {
  font-family: 'SamsungOneUI', 'SamsungColorEmoji', 'Noto Color Emoji', emoji;
}

@keyframes winEnter {
  from {
    opacity: 0;
    transform: scale(0.94) translateY(6px);
  }

  to {
    opacity: 1;
    transform: scale(1) translateY(0);
  }
}

@keyframes popIn {
  from {
    opacity: 0;
    transform: scale(0.88);
  }

  to {
    opacity: 1;
    transform: scale(1);
  }
}

::-webkit-scrollbar {
  width: 6px;
  height: 6px;
}

::-webkit-scrollbar-track {
  background: transparent;
}

::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.18);
  border-radius: 3px;
}

::-webkit-scrollbar-thumb:hover {
  background: rgba(255, 255, 255, 0.28);
}

/* xterm overrides */
.xterm-viewport::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.12);
}
</style>
