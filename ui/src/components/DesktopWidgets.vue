<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { ws } from '../composables/useWs'

// ─── Types ───────────────────────────────────────────────────────────────────
interface Vitals {
  cpu_percent: number
  ram_used_mb: number
  ram_total_mb: number
  swap_used_mb: number
  swap_total_mb: number
  disk_read_bytes: number
  disk_write_bytes: number
  cpu_per_core: number[]
}

// ─── State ───────────────────────────────────────────────────────────────────
const vitals = ref<Vitals | null>(null)
const cpuHistory = ref<number[]>([])
const ramHistory = ref<number[]>([])
const diskReadHistory = ref<number[]>([])
const diskWriteHistory = ref<number[]>([])
const prevDiskRead = ref(0)
const prevDiskWrite = ref(0)
const HISTORY_LEN = 40

const visible = ref(JSON.parse(localStorage.getItem('widgets_visible') ?? 'true'))

watch(visible, v => localStorage.setItem('widgets_visible', JSON.stringify(v)))

// Expose toggle for MenuBar
defineExpose({ visible })

// ─── Widget drag ─────────────────────────────────────────────────────────────
interface Pos { x: number; y: number }

const DEFAULT_POSITIONS: Record<string, Pos> = {
  cpu:  { x: 16, y: 44 },
  ram:  { x: 16, y: 248 },
  disk: { x: 16, y: 440 },
}

function loadPos(id: string): Pos {
  try {
    const s = localStorage.getItem(`widget_pos_${id}`)
    if (s) return JSON.parse(s)
  } catch {}
  return DEFAULT_POSITIONS[id]
}

const positions = ref<Record<string, Pos>>({
  cpu:  loadPos('cpu'),
  ram:  loadPos('ram'),
  disk: loadPos('disk'),
})

let dragging: { id: string; ox: number; oy: number } | null = null

function startDrag(id: string, e: MouseEvent) {
  e.preventDefault()
  const pos = positions.value[id]
  dragging = { id, ox: e.clientX - pos.x, oy: e.clientY - pos.y }
}

function onMouseMove(e: MouseEvent) {
  if (!dragging) return
  const x = Math.max(0, Math.min(window.innerWidth - 220, e.clientX - dragging.ox))
  const y = Math.max(44, Math.min(window.innerHeight - 160, e.clientY - dragging.oy))
  positions.value[dragging.id] = { x, y }
}

function onMouseUp() {
  if (dragging) {
    localStorage.setItem(`widget_pos_${dragging.id}`, JSON.stringify(positions.value[dragging.id]))
    dragging = null
  }
}

// ─── WS listener ─────────────────────────────────────────────────────────────
function push(arr: number[], val: number) {
  arr.push(val)
  if (arr.length > HISTORY_LEN) arr.splice(0, arr.length - HISTORY_LEN)
}

function onMsg(e: MessageEvent) {
  try {
    const msg = JSON.parse(e.data)
    if (msg.type !== 'SystemVitals') return
    const v: Vitals = msg.vitals
    vitals.value = v

    push(cpuHistory.value, v.cpu_percent)
    push(ramHistory.value, (v.ram_used_mb / v.ram_total_mb) * 100)

    const rd = prevDiskRead.value > 0 ? Math.max(0, (v.disk_read_bytes - prevDiskRead.value) / 2) : 0
    const wr = prevDiskWrite.value > 0 ? Math.max(0, (v.disk_write_bytes - prevDiskWrite.value) / 2) : 0
    prevDiskRead.value = v.disk_read_bytes
    prevDiskWrite.value = v.disk_write_bytes
    push(diskReadHistory.value, rd)
    push(diskWriteHistory.value, wr)
  } catch {}
}

function registerWs(socket: WebSocket | null) {
  socket?.addEventListener('message', onMsg)
}
function deregisterWs(socket: WebSocket | null) {
  socket?.removeEventListener('message', onMsg)
}

onMounted(() => {
  registerWs(ws.value)
  window.addEventListener('mousemove', onMouseMove)
  window.addEventListener('mouseup', onMouseUp)
})
onUnmounted(() => {
  deregisterWs(ws.value)
  window.removeEventListener('mousemove', onMouseMove)
  window.removeEventListener('mouseup', onMouseUp)
})
watch(ws, (n, o) => { deregisterWs(o); registerWs(n) })

// ─── SVG Sparkline helpers ───────────────────────────────────────────────────
function sparklinePath(data: number[], w: number, h: number, max?: number): string {
  if (data.length < 2) return ''
  const mx = max ?? Math.max(...data, 1)
  const step = w / (HISTORY_LEN - 1)
  const pts = Array.from({ length: HISTORY_LEN }, (_, i) => {
    const v = data[data.length - HISTORY_LEN + i] ?? 0
    return [i * step, h - (v / mx) * h]
  }).filter((_, i) => (data.length - HISTORY_LEN + i) >= 0 ? true : false)

  // Build filled area path
  if (pts.length < 2) return ''
  const line = pts.map((p, i) => `${i === 0 ? 'M' : 'L'}${p[0].toFixed(1)},${p[1].toFixed(1)}`).join(' ')
  return `${line} L${pts[pts.length-1][0].toFixed(1)},${h} L${pts[0][0].toFixed(1)},${h} Z`
}

function sparklineStroke(data: number[], w: number, h: number, max?: number): string {
  if (data.length < 2) return ''
  const mx = max ?? Math.max(...data, 1)
  const step = w / (HISTORY_LEN - 1)
  const pts: [number, number][] = []
  for (let i = 0; i < HISTORY_LEN; i++) {
    const v = data[data.length - HISTORY_LEN + i] ?? 0
    pts.push([i * step, h - (v / mx) * h])
  }
  return pts.map((p, i) => `${i === 0 ? 'M' : 'L'}${p[0].toFixed(1)},${p[1].toFixed(1)}`).join(' ')
}

// ─── Donut arc helpers ────────────────────────────────────────────────────────
function donutArc(pct: number, r: number, strokeW: number): string {
  const circ = 2 * Math.PI * r
  const dash = (pct / 100) * circ
  return `${dash.toFixed(1)} ${circ.toFixed(1)}`
}

// ─── Computed ─────────────────────────────────────────────────────────────────

const cpuPct = computed(() => vitals.value?.cpu_percent ?? 0)
const ramPct = computed(() => vitals.value ? (vitals.value.ram_used_mb / vitals.value.ram_total_mb) * 100 : 0)
const cpuColor = computed(() => cpuPct.value > 80 ? '#f87171' : cpuPct.value > 50 ? '#facc15' : '#34d399')
const ramColor = computed(() => ramPct.value > 85 ? '#f87171' : ramPct.value > 60 ? '#fb923c' : '#60a5fa')

function fmtBytes(b: number): string {
  if (b < 1024) return `${b.toFixed(0)} B`
  if (b < 1024 ** 2) return `${(b / 1024).toFixed(1)} KB`
  return `${(b / (1024 ** 2)).toFixed(1)} MB`
}

function fmtMB(mb: number): string {
  if (mb >= 1024) return `${(mb / 1024).toFixed(1)} GB`
  return `${mb} MB`
}

const DONUT_R = 38
const DONUT_CX = 48
const DONUT_CY = 48
const STROKE_W = 7

const maxDisk = computed(() => {
  const all = [...diskReadHistory.value, ...diskWriteHistory.value]
  return Math.max(...all, 1)
})
</script>

<template>
  <Transition name="widgets-fade">
    <div v-if="visible" class="widgets-layer pointer-events-none">

      <!-- ── CPU Widget ──────────────────────────────────────────────────── -->
      <div
        class="widget pointer-events-auto"
        :style="`left:${positions.cpu.x}px; top:${positions.cpu.y}px`"
        @mousedown="startDrag('cpu', $event)"
      >
        <div class="widget-header">
          <span class="widget-label">CPU</span>
          <span class="widget-value" :style="`color:${cpuColor}`">{{ cpuPct.toFixed(1) }}%</span>
        </div>

        <!-- Donut + sparkline row -->
        <div class="flex items-center gap-3 mt-2">
          <!-- Donut -->
          <svg width="96" height="96" style="flex-shrink:0">
            <!-- Track -->
            <circle :cx="DONUT_CX" :cy="DONUT_CY" :r="DONUT_R"
              fill="none" stroke="rgba(255,255,255,0.07)" :stroke-width="STROKE_W"/>
            <!-- Arc -->
            <circle :cx="DONUT_CX" :cy="DONUT_CY" :r="DONUT_R"
              fill="none" :stroke="cpuColor" :stroke-width="STROKE_W"
              stroke-linecap="round"
              :stroke-dasharray="donutArc(cpuPct, DONUT_R, STROKE_W)"
              stroke-dashoffset="0"
              transform="rotate(-90 48 48)"
              style="transition: stroke-dasharray 0.6s ease, stroke 0.4s ease"
            />
            <text x="48" y="44" text-anchor="middle" :fill="cpuColor" font-size="16" font-weight="700" font-family="JetBrains Mono, monospace">{{ cpuPct.toFixed(0) }}</text>
            <text x="48" y="58" text-anchor="middle" fill="rgba(255,255,255,0.4)" font-size="10" font-family="system-ui">percent</text>
          </svg>

          <!-- Sparkline area -->
          <svg width="110" height="56" style="flex:1; overflow:hidden">
            <defs>
              <linearGradient id="cpu-grad" x1="0" y1="0" x2="0" y2="1">
                <stop offset="0%" :stop-color="cpuColor" stop-opacity="0.35"/>
                <stop offset="100%" :stop-color="cpuColor" stop-opacity="0.0"/>
              </linearGradient>
            </defs>
            <path :d="sparklinePath(cpuHistory, 110, 56, 100)" fill="url(#cpu-grad)"/>
            <path :d="sparklineStroke(cpuHistory, 110, 56, 100)"
              fill="none" :stroke="cpuColor" stroke-width="1.5" stroke-linejoin="round" stroke-linecap="round"/>
          </svg>
        </div>

        <!-- Per-core bars (max 32 shown, wrapped, capped height) -->
        <div v-if="vitals && vitals.cpu_per_core.length" class="core-bars mt-2">
          <div
            v-for="(c, i) in vitals.cpu_per_core.slice(0, 32)" :key="i"
            class="core-bar-wrap"
            :title="`Core ${i}: ${c.toFixed(1)}%`"
          >
            <div class="core-bar-fill"
              :style="`height:${Math.max(2, (c/100)*22)}px; background:${c>80?'#f87171':c>50?'#facc15':cpuColor}`"/>
          </div>
        </div>
      </div>

      <!-- ── RAM Widget ──────────────────────────────────────────────────── -->
      <div
        class="widget pointer-events-auto"
        :style="`left:${positions.ram.x}px; top:${positions.ram.y}px`"
        @mousedown="startDrag('ram', $event)"
      >
        <div class="widget-header">
          <span class="widget-label">Memory</span>
          <span class="widget-value" :style="`color:${ramColor}`">{{ ramPct.toFixed(1) }}%</span>
        </div>

        <div class="flex items-center gap-3 mt-2">
          <!-- Donut -->
          <svg width="96" height="96" style="flex-shrink:0">
            <circle :cx="DONUT_CX" :cy="DONUT_CY" :r="DONUT_R"
              fill="none" stroke="rgba(255,255,255,0.07)" :stroke-width="STROKE_W"/>
            <circle :cx="DONUT_CX" :cy="DONUT_CY" :r="DONUT_R"
              fill="none" :stroke="ramColor" :stroke-width="STROKE_W"
              stroke-linecap="round"
              :stroke-dasharray="donutArc(ramPct, DONUT_R, STROKE_W)"
              transform="rotate(-90 48 48)"
              style="transition: stroke-dasharray 0.6s ease, stroke 0.4s ease"
            />
            <text x="48" y="42" text-anchor="middle" :fill="ramColor" font-size="11" font-weight="700" font-family="JetBrains Mono, monospace">{{ vitals ? vitals.ram_used_mb : '—' }}</text>
            <text x="48" y="54" text-anchor="middle" fill="rgba(255,255,255,0.35)" font-size="10" font-family="system-ui">/ {{ vitals ? vitals.ram_total_mb : '—' }}</text>
            <text x="48" y="66" text-anchor="middle" fill="rgba(255,255,255,0.25)" font-size="9" font-family="system-ui">MB</text>
          </svg>

          <!-- Sparkline -->
          <div class="flex flex-col flex-1 gap-1" style="min-width:0">
            <svg width="110" height="40" style="overflow:hidden; display:block">
              <defs>
                <linearGradient id="ram-grad" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="0%" :stop-color="ramColor" stop-opacity="0.3"/>
                  <stop offset="100%" :stop-color="ramColor" stop-opacity="0.0"/>
                </linearGradient>
              </defs>
              <path :d="sparklinePath(ramHistory, 110, 40, 100)" fill="url(#ram-grad)"/>
              <path :d="sparklineStroke(ramHistory, 110, 40, 100)"
                fill="none" :stroke="ramColor" stroke-width="1.5" stroke-linejoin="round" stroke-linecap="round"/>
            </svg>
            <!-- Swap row -->
            <div v-if="vitals" class="flex items-center gap-1.5" style="min-width:0">
              <span class="text-[10px] text-white/30 uppercase tracking-wider font-semibold flex-none">Swap</span>
              <div class="flex-1 h-1 rounded-full overflow-hidden" style="background:rgba(255,255,255,0.07); min-width:0">
                <div class="h-full rounded-full" style="background:#a78bfa; transition: width 0.5s ease"
                  :style="`width:${vitals.swap_total_mb > 0 ? (vitals.swap_used_mb/vitals.swap_total_mb*100) : 0}%`"/>
              </div>
              <span class="text-[10px] font-mono flex-none" style="color:#a78bfa">{{ fmtMB(vitals.swap_used_mb) }}</span>
            </div>
          </div>
        </div>
      </div>

      <!-- ── Disk I/O Widget ────────────────────────────────────────────── -->
      <div
        class="widget pointer-events-auto"
        :style="`left:${positions.disk.x}px; top:${positions.disk.y}px`"
        @mousedown="startDrag('disk', $event)"
      >
        <div class="widget-header">
          <span class="widget-label">Disk I/O</span>
        </div>

        <!-- Dual sparkline -->
        <svg width="196" height="64" class="mt-2" style="overflow:hidden; display:block">
          <defs>
            <linearGradient id="read-grad" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stop-color="#34d399" stop-opacity="0.25"/>
              <stop offset="100%" stop-color="#34d399" stop-opacity="0"/>
            </linearGradient>
            <linearGradient id="write-grad" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stop-color="#f472b6" stop-opacity="0.25"/>
              <stop offset="100%" stop-color="#f472b6" stop-opacity="0"/>
            </linearGradient>
          </defs>
          <path :d="sparklinePath(diskReadHistory, 196, 64, maxDisk)" fill="url(#read-grad)"/>
          <path :d="sparklineStroke(diskReadHistory, 196, 64, maxDisk)"
            fill="none" stroke="#34d399" stroke-width="1.5" stroke-linejoin="round" stroke-linecap="round"/>
          <path :d="sparklinePath(diskWriteHistory, 196, 64, maxDisk)" fill="url(#write-grad)"/>
          <path :d="sparklineStroke(diskWriteHistory, 196, 64, maxDisk)"
            fill="none" stroke="#f472b6" stroke-width="1.5" stroke-linejoin="round" stroke-linecap="round"/>
        </svg>

        <!-- Legend row -->
        <div class="flex justify-between mt-2">
          <div class="flex items-center gap-1.5">
            <div class="w-2 h-2 rounded-full" style="background:#34d399"/>
            <span class="text-[10px] text-white/40 font-semibold uppercase tracking-wider">Read</span>
            <span class="text-[11px] font-mono" style="color:#34d399">
              {{ fmtBytes(diskReadHistory[diskReadHistory.length - 1] ?? 0) }}/s
            </span>
          </div>
          <div class="flex items-center gap-1.5">
            <div class="w-2 h-2 rounded-full" style="background:#f472b6"/>
            <span class="text-[10px] text-white/40 font-semibold uppercase tracking-wider">Write</span>
            <span class="text-[11px] font-mono" style="color:#f472b6">
              {{ fmtBytes(diskWriteHistory[diskWriteHistory.length - 1] ?? 0) }}/s
            </span>
          </div>
        </div>
      </div>

    </div>
  </Transition>
</template>

<style scoped>
.widgets-layer {
  position: fixed;
  inset: 0;
  z-index: 10; /* above bg, below windows */
  pointer-events: none;
}

.widget {
  position: absolute;
  width: 228px;
  padding: 14px 14px 12px;
  border-radius: 18px;
  border: 1px solid rgba(255, 255, 255, 0.08);
  background: rgba(12, 12, 16, 0.72);
  backdrop-filter: blur(40px) saturate(180%);
  -webkit-backdrop-filter: blur(40px) saturate(180%);
  box-shadow:
    0 8px 32px rgba(0, 0, 0, 0.5),
    0 1px 0 rgba(255, 255, 255, 0.06) inset;
  cursor: grab;
  user-select: none;
  transition: box-shadow 120ms ease;
  overflow: hidden;
  box-sizing: border-box;
}

.widget:active {
  cursor: grabbing;
  box-shadow:
    0 20px 60px rgba(0, 0, 0, 0.7),
    0 1px 0 rgba(255, 255, 255, 0.08) inset;
}

.widget-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.widget-label {
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: rgba(255, 255, 255, 0.35);
}

.widget-value {
  font-size: 13px;
  font-weight: 700;
  font-family: 'JetBrains Mono', monospace;
  letter-spacing: -0.02em;
  transition: color 0.4s ease;
}

/* Per-core bars */
.core-bars {
  display: flex;
  align-items: flex-end;
  flex-wrap: wrap;
  gap: 2px;
  max-height: 26px;
  overflow: hidden;
}
.core-bar-wrap {
  flex: 1;
  min-width: 4px;
  height: 22px;
  display: flex;
  align-items: flex-end;
  background: rgba(255, 255, 255, 0.04);
  border-radius: 2px;
  overflow: hidden;
}
.core-bar-fill {
  width: 100%;
  border-radius: 3px;
  transition: height 0.5s ease, background 0.4s ease;
  min-height: 2px;
}

/* Transitions */
.widgets-fade-enter-active {
  transition: opacity 250ms ease, transform 300ms cubic-bezier(0.16, 1, 0.3, 1);
}
.widgets-fade-leave-active {
  transition: opacity 180ms ease, transform 200ms ease;
}
.widgets-fade-enter-from,
.widgets-fade-leave-to {
  opacity: 0;
  transform: translateX(-12px);
}
</style>
