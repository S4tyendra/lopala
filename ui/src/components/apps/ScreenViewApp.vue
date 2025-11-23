<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, computed } from 'vue'
import { wsSend } from '../../composables/useWs'
import type { FileEntry } from '../../types'

const props = defineProps<{ winId: string }>()

// ─── Display selection ────────────────────────────────────────────────────────
const displays = ref<{ name: string; description: string }[]>([])
const selectedDisplay = ref<string | null>(null)

const fetchDisplays = async () => {
  try {
    const res = await fetch('/api/displays')
    if (res.ok) {
      displays.value = await res.json()
      if (!selectedDisplay.value && displays.value.length > 0) {
        selectedDisplay.value = displays.value[0].name
      }
    }
  } catch (e) { console.error(e) }
}

// ─── Live stream state ────────────────────────────────────────────────────────
const isLive = ref(false)
const currentFramePath = ref<string | null>(null)
const frameUrl = computed(() =>
  currentFramePath.value
    ? `/api/files/download?path=${encodeURIComponent(currentFramePath.value)}`
    : null
)

// ─── Seek / Capture history ───────────────────────────────────────────────────
const historyFiles = ref<FileEntry[]>([])
const seekIndex = ref(0) // index into historyFiles, 0 = newest
const isSeeking = ref(false)

const fetchHistory = async () => {
  if (!selectedDisplay.value) return
  try {
    const display = selectedDisplay.value
    // screenshots dir for seek (permanent), streams dir for transient frames
    const screenshotDir = `/tmp/lopala/screenshots/${display}`
    const res = await fetch(`/api/files?path=${encodeURIComponent(screenshotDir)}`)
    if (!res.ok) { historyFiles.value = []; return }
    const entries: FileEntry[] = await res.json()
    historyFiles.value = entries
      .filter(e => !e.is_dir && e.name.endsWith('.png'))
      .sort((a, b) => b.modified - a.modified)
    seekIndex.value = 0
  } catch (e) { console.error(e) }
}

const seekedUrl = computed(() => {
  if (!historyFiles.value.length) return null
  const f = historyFiles.value[seekIndex.value]
  return f ? `/api/files/download?path=${encodeURIComponent(f.path)}` : null
})

const seekLabel = computed(() => {
  if (!historyFiles.value.length) return ''
  const f = historyFiles.value[seekIndex.value]
  if (!f) return ''
  return new Date(f.modified * 1000).toLocaleTimeString([], {
    hour: '2-digit', minute: '2-digit', second: '2-digit'
  })
})

// ─── WS ScreenFrame handler ───────────────────────────────────────────────────
const handleWsMessage = (event: MessageEvent) => {
  try {
    const msg = JSON.parse(event.data)
    if (
      msg.type === 'ScreenFrame' &&
      msg.display === selectedDisplay.value &&
      isLive.value &&
      !isSeeking.value
    ) {
      currentFramePath.value = msg.path
    }
  } catch {}
}

// ─── WS socket reference ─────────────────────────────────────────────────────
// We piggyback on the global WS — attach listener directly on the window event
// (App.vue exposes the raw socket events via a custom event bus)
let ws: WebSocket | null = null
const connectToWs = () => {
  const proto = location.protocol === 'https:' ? 'wss' : 'ws'
  ws = new WebSocket(`${proto}://${location.host}/_ws`)
  ws.addEventListener('message', handleWsMessage)
  ws.addEventListener('close', () => {
    ws = null
    if (isLive.value) setTimeout(connectToWs, 1000)
  })
}

// ─── Start / Stop stream ─────────────────────────────────────────────────────
const startStream = () => {
  if (!selectedDisplay.value) return
  isLive.value = true
  isSeeking.value = false
  wsSend({ type: 'StartStream', display: selectedDisplay.value })
}

const stopStream = () => {
  if (!selectedDisplay.value) return
  isLive.value = false
  wsSend({ type: 'StopStream', display: selectedDisplay.value })
}

const toggleLive = () => {
  if (isLive.value) stopStream()
  else startStream()
}

// ─── Seeking mode ─────────────────────────────────────────────────────────────
const enterSeek = () => {
  isSeeking.value = true
  if (isLive.value) stopStream()
  fetchHistory()
}

const exitSeek = () => {
  isSeeking.value = false
  currentFramePath.value = null
}

// ─── Display change ───────────────────────────────────────────────────────────
watch(selectedDisplay, (next, prev) => {
  if (prev && isLive.value) wsSend({ type: 'StopStream', display: prev })
  if (next && isLive.value) wsSend({ type: 'StartStream', display: next })
  currentFramePath.value = null
})

// ─── Lifecycle ────────────────────────────────────────────────────────────────
onMounted(() => {
  connectToWs()
  fetchDisplays()
})

onUnmounted(() => {
  if (isLive.value && selectedDisplay.value) {
    wsSend({ type: 'StopStream', display: selectedDisplay.value })
  }
  if (ws) { ws.removeEventListener('message', handleWsMessage); ws.close() }
})
</script>

<template>
  <div class="flex flex-col h-full overflow-hidden" style="background: #111; color: #fff; font-size: 13px;">

    <!-- ── Header Bar ──────────────────────────────────────────────────────── -->
    <div class="flex-none flex items-center justify-between px-3 py-2 border-b"
      style="border-color: rgba(255,255,255,0.1); background: rgba(255,255,255,0.03)">

      <!-- Display selector -->
      <div class="flex items-center gap-2">
        <select :value="selectedDisplay"
          @change="(e) => (selectedDisplay = (e.target as HTMLSelectElement).value)"
          class="rounded-md px-2 py-1 outline-none cursor-pointer"
          style="background: rgba(255,255,255,0.1); border: 1px solid rgba(255,255,255,0.15); color: #fff">
          <option v-for="d in displays" :key="d.name" :value="d.name"
            style="background: #1a1a1a">
            {{ d.name }} — {{ d.description }}
          </option>
        </select>
      </div>

      <!-- Controls -->
      <div class="flex items-center gap-2">
        <!-- Seek toggle -->
        <button @click="isSeeking ? exitSeek() : enterSeek()"
          class="flex items-center gap-1.5 px-3 py-1 rounded-md transition-colors"
          :style="isSeeking
            ? 'background: rgba(234,179,8,0.25); color: #eab308; border: 1px solid rgba(234,179,8,0.4)'
            : 'background: rgba(255,255,255,0.08); color: rgba(255,255,255,0.7); border: 1px solid rgba(255,255,255,0.12)'">
          <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24"
            fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="15 18 9 12 15 6"></polyline>
          </svg>
          Seek
        </button>

        <!-- Live toggle -->
        <button @click="toggleLive" :disabled="isSeeking"
          class="flex items-center gap-1.5 px-3 py-1 rounded-md font-medium transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
          :style="isLive
            ? 'background: rgba(239,68,68,0.25); color: #ef4444; border: 1px solid rgba(239,68,68,0.4)'
            : 'background: rgba(34,197,94,0.2); color: #22c55e; border: 1px solid rgba(34,197,94,0.4)'">
          <!-- pulsing dot when live -->
          <span v-if="isLive" class="relative flex h-2 w-2">
            <span class="animate-ping absolute inline-flex h-full w-full rounded-full bg-red-400 opacity-75"></span>
            <span class="relative inline-flex rounded-full h-2 w-2 bg-red-500"></span>
          </span>
          {{ isLive ? 'Stop' : 'Go Live' }}
        </button>
      </div>
    </div>

    <!-- ── Viewport ────────────────────────────────────────────────────────── -->
    <div class="flex-1 relative overflow-hidden flex items-center justify-center"
      style="background: #000">

      <!-- Frame display -->
      <img v-if="!isSeeking && frameUrl" :src="frameUrl" :key="frameUrl"
        class="w-full h-full object-contain" style="image-rendering: auto" />

      <img v-else-if="isSeeking && seekedUrl" :src="seekedUrl" :key="seekedUrl"
        class="w-full h-full object-contain" />

      <!-- Idle placeholder -->
      <div v-else class="flex flex-col items-center gap-3 select-none"
        style="color: rgba(255,255,255,0.3)">
        <svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24"
          fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round">
          <rect x="2" y="3" width="20" height="14" rx="2" ry="2"></rect>
          <line x1="8" y1="21" x2="16" y2="21"></line>
          <line x1="12" y1="17" x2="12" y2="21"></line>
        </svg>
        <span v-if="isSeeking">No screenshots found</span>
        <span v-else>Press <b style="color: #22c55e">Go Live</b> to start streaming</span>
      </div>

      <!-- Live badge overlay -->
      <div v-if="isLive && !isSeeking"
        class="absolute top-3 left-3 flex items-center gap-1.5 px-2 py-1 rounded-full text-[11px] font-bold uppercase tracking-wider"
        style="background: rgba(239,68,68,0.85); backdrop-filter: blur(6px)">
        <span class="relative flex h-2 w-2">
          <span class="animate-ping absolute inline-flex h-full w-full rounded-full bg-red-200 opacity-75"></span>
          <span class="relative inline-flex rounded-full h-2 w-2 bg-white"></span>
        </span>
        LIVE
      </div>

      <!-- FPS / frame path indicator -->
      <div v-if="isLive && currentFramePath"
        class="absolute bottom-2 right-3 text-[10px] font-mono"
        style="color: rgba(255,255,255,0.3)">
        {{ currentFramePath.split('/').pop() }}
      </div>
    </div>

    <!-- ── Seek bar  ────────────────────────────────────────────────────────── -->
    <div v-if="isSeeking" class="flex-none px-3 py-2 border-t"
      style="border-color: rgba(255,255,255,0.1); background: rgba(255,255,255,0.03)">

      <div v-if="historyFiles.length === 0" class="text-center py-1"
        style="color: rgba(255,255,255,0.3)">
        No screenshots in gallery yet — take some first
      </div>

      <div v-else class="flex flex-col gap-1">
        <div class="flex justify-between text-[11px]" style="color: rgba(255,255,255,0.4)">
          <span>{{ historyFiles.length }} frame{{ historyFiles.length !== 1 ? 's' : '' }}</span>
          <span>{{ seekLabel }}</span>
        </div>
        <input type="range" min="0" :max="historyFiles.length - 1"
          :value="seekIndex"
          @input="(e) => seekIndex = Number((e.target as HTMLInputElement).value)"
          class="w-full accent-yellow-400 cursor-pointer" style="height: 4px" />
        <div class="flex justify-between text-[10px]" style="color: rgba(255,255,255,0.25)">
          <span>← Newest</span>
          <span>Oldest →</span>
        </div>
      </div>
    </div>

  </div>
</template>
