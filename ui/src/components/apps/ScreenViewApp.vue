<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from 'vue'
import { wsSend } from '../../composables/useWs'

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

// ─── Double-buffer crossfade ──────────────────────────────────────────────────
// Two img slots (A/B). Active slot shows latest frame, inactive fades out.
// No key-based remounting → zero flicker.
const slotA = ref<string | null>(null)
const slotB = ref<string | null>(null)
const activeSlot = ref<'a' | 'b'>('a')

function pushFrame(path: string) {
  const url = `/api/files/download?path=${encodeURIComponent(path)}`
  if (activeSlot.value === 'a') {
    slotB.value = url   // preload into inactive slot
    activeSlot.value = 'b'
  } else {
    slotA.value = url
    activeSlot.value = 'a'
  }
}

// ─── Live state ───────────────────────────────────────────────────────────────
const isLive = ref(false)

// ─── WS listener (dedicated socket, avoids event-bus coupling) ───────────────
let ws: WebSocket | null = null

const handleMsg = (e: MessageEvent) => {
  try {
    const msg = JSON.parse(e.data)
    if (msg.type === 'ScreenFrame' && msg.display === selectedDisplay.value && isLive.value) {
      pushFrame(msg.path)
    }
  } catch {}
}

const connectWs = () => {
  const proto = location.protocol === 'https:' ? 'wss' : 'ws'
  ws = new WebSocket(`${proto}://${location.host}/_ws`)
  ws.addEventListener('message', handleMsg)
  ws.addEventListener('close', () => {
    ws = null
    if (isLive.value) setTimeout(connectWs, 1000)
  })
}

// ─── Stream control ───────────────────────────────────────────────────────────
const startStream = () => {
  if (!selectedDisplay.value) return
  isLive.value = true
  wsSend({ type: 'StartStream', display: selectedDisplay.value })
}

const stopStream = () => {
  if (!selectedDisplay.value) return
  isLive.value = false
  wsSend({ type: 'StopStream', display: selectedDisplay.value })
}

const toggleLive = () => isLive.value ? stopStream() : startStream()

// ─── Display change ───────────────────────────────────────────────────────────
watch(selectedDisplay, (next, prev) => {
  if (prev && isLive.value) wsSend({ type: 'StopStream', display: prev })
  slotA.value = null
  slotB.value = null
  activeSlot.value = 'a'
  if (next && isLive.value) wsSend({ type: 'StartStream', display: next })
})

// ─── Lifecycle ────────────────────────────────────────────────────────────────
onMounted(() => {
  connectWs()
  fetchDisplays()
})

onUnmounted(() => {
  if (isLive.value && selectedDisplay.value) {
    wsSend({ type: 'StopStream', display: selectedDisplay.value })
  }
  if (ws) { ws.removeEventListener('message', handleMsg); ws.close() }
})
</script>

<template>
  <div class="flex flex-col h-full overflow-hidden" style="background:#111; color:#fff; font-size:13px">

    <!-- ── Header ─────────────────────────────────────────────────────────── -->
    <div class="flex-none flex items-center justify-between px-3 py-2 border-b"
      style="border-color:rgba(255,255,255,0.1); background:rgba(255,255,255,0.03)">

      <select :value="selectedDisplay"
        @change="e => (selectedDisplay = (e.target as HTMLSelectElement).value)"
        class="rounded-md px-2 py-1 outline-none cursor-pointer"
        style="background:rgba(255,255,255,0.1); border:1px solid rgba(255,255,255,0.15); color:#fff">
        <option v-for="d in displays" :key="d.name" :value="d.name" style="background:#1a1a1a">
          {{ d.name }} — {{ d.description }}
        </option>
      </select>

      <button @click="toggleLive" :disabled="!selectedDisplay"
        class="flex items-center gap-2 px-4 py-1.5 rounded-md font-medium transition-all duration-150 disabled:opacity-40 disabled:cursor-not-allowed"
        :style="isLive
          ? 'background:rgba(239,68,68,0.2); color:#f87171; border:1px solid rgba(239,68,68,0.35)'
          : 'background:rgba(34,197,94,0.15); color:#4ade80; border:1px solid rgba(34,197,94,0.35)'">
        <span v-if="isLive" class="relative flex h-2 w-2">
          <span class="animate-ping absolute inline-flex h-full w-full rounded-full opacity-75"
            style="background:#f87171"></span>
          <span class="relative inline-flex rounded-full h-2 w-2" style="background:#ef4444"></span>
        </span>
        {{ isLive ? 'Stop' : 'Go Live' }}
      </button>
    </div>

    <!-- ── Viewport ───────────────────────────────────────────────────────── -->
    <div class="flex-1 relative overflow-hidden" style="background:#000">

      <!-- Slot A -->
      <img v-if="slotA" :src="slotA"
        class="absolute inset-0 w-full h-full object-contain transition-opacity duration-150"
        :style="{ opacity: activeSlot === 'a' ? 1 : 0 }"
        draggable="false" />

      <!-- Slot B -->
      <img v-if="slotB" :src="slotB"
        class="absolute inset-0 w-full h-full object-contain transition-opacity duration-150"
        :style="{ opacity: activeSlot === 'b' ? 1 : 0 }"
        draggable="false" />

      <!-- Idle state -->
      <div v-if="!slotA && !slotB"
        class="absolute inset-0 flex flex-col items-center justify-center gap-3 select-none"
        style="color:rgba(255,255,255,0.2)">
        <svg xmlns="http://www.w3.org/2000/svg" width="52" height="52" viewBox="0 0 24 24"
          fill="none" stroke="currentColor" stroke-width="0.75">
          <rect x="2" y="3" width="20" height="14" rx="2" ry="2"/>
          <line x1="8" y1="21" x2="16" y2="21"/>
          <line x1="12" y1="17" x2="12" y2="21"/>
        </svg>
        <span>Press <b style="color:#4ade80">Go Live</b> to start</span>
      </div>

      <!-- LIVE badge -->
      <div v-if="isLive"
        class="absolute top-2.5 left-2.5 flex items-center gap-1.5 px-2 py-0.5 rounded-full text-[11px] font-bold uppercase tracking-wider"
        style="background:rgba(239,68,68,0.82); backdrop-filter:blur(6px)">
        <span class="relative flex h-2 w-2">
          <span class="animate-ping absolute inline-flex h-full w-full rounded-full bg-red-200 opacity-75"></span>
          <span class="relative inline-flex rounded-full h-2 w-2 bg-white"></span>
        </span>
        LIVE
      </div>
    </div>

  </div>
</template>
