<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { wsSend, ws } from '../../composables/useWs'

// ─── Types ────────────────────────────────────────────────────────────────────
interface ProcessInfo {
  pid: number
  name: string
  cpu: number
  mem_mb: number
  user: string
  command: string
}

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

// ─── State ────────────────────────────────────────────────────────────────────
const procs = ref<ProcessInfo[]>([])
const vitals = ref<Vitals | null>(null)
const prevDiskRead = ref(0)
const prevDiskWrite = ref(0)
const diskReadRate = ref(0)
const diskWriteRate = ref(0)
const cpuHistory = ref<number[]>([])
const ramHistory = ref<number[]>([])
const MAX_HISTORY = 60

const sortCol = ref<'cpu' | 'mem_mb' | 'pid' | 'name'>('cpu')
const sortAsc = ref(false)
const filter = ref('')
const loading = ref(false)
const killing = ref<number | null>(null)

// ─── Computed ─────────────────────────────────────────────────────────────────
const filteredProcs = computed(() => {
  let list = [...procs.value]
  if (filter.value) {
    const q = filter.value.toLowerCase()
    list = list.filter(p => p.name.toLowerCase().includes(q) || p.command.toLowerCase().includes(q) || String(p.pid).includes(q))
  }
  list.sort((a, b) => {
    const av = a[sortCol.value], bv = b[sortCol.value]
    const cmp = typeof av === 'string' ? (av as string).localeCompare(bv as string) : (av as number) - (bv as number)
    return sortAsc.value ? cmp : -cmp
  })
  return list
})

const ramPercent = computed(() => {
  if (!vitals.value) return 0
  return Math.round((vitals.value.ram_used_mb / vitals.value.ram_total_mb) * 100)
})

// ─── Fetch processes ──────────────────────────────────────────────────────────
let pollTimer: ReturnType<typeof setInterval> | null = null

async function fetchProcs() {
  loading.value = true
  try {
    const res = await fetch('/api/system/ps')
    if (res.ok) procs.value = await res.json()
  } catch {}
  loading.value = false
}

async function killProc(pid: number) {
  killing.value = pid
  try {
    await fetch('/api/system/kill', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ pid }),
    })
    await fetchProcs()
  } catch {}
  killing.value = null
}

// ─── WS vitals listener ──────────────────────────────────────────────────────
function onWsMsg(e: MessageEvent) {
  try {
    const msg = JSON.parse(e.data)
    if (msg.type === 'SystemVitals') {
      const v = msg.vitals as Vitals
      vitals.value = v

      // CPU/RAM history for sparklines
      cpuHistory.value.push(v.cpu_percent)
      if (cpuHistory.value.length > MAX_HISTORY) cpuHistory.value.shift()
      ramHistory.value.push(ramPercent.value)
      if (ramHistory.value.length > MAX_HISTORY) ramHistory.value.shift()

      // Disk rate calculation
      if (prevDiskRead.value > 0) {
        diskReadRate.value = Math.max(0, (v.disk_read_bytes - prevDiskRead.value) / 2) // per sec (2s interval)
        diskWriteRate.value = Math.max(0, (v.disk_write_bytes - prevDiskWrite.value) / 2)
      }
      prevDiskRead.value = v.disk_read_bytes
      prevDiskWrite.value = v.disk_write_bytes
    }
  } catch {}
}

function fmtBytes(b: number): string {
  if (b < 1024) return `${b} B/s`
  if (b < 1024 ** 2) return `${(b / 1024).toFixed(1)} KB/s`
  return `${(b / (1024 ** 2)).toFixed(1)} MB/s`
}

function sparklinePath(data: number[], w: number, h: number): string {
  if (data.length < 2) return ''
  const max = Math.max(...data, 1)
  const step = w / (MAX_HISTORY - 1)
  return data.map((v, i) => {
    const x = i * step
    const y = h - (v / max) * h
    return `${i === 0 ? 'M' : 'L'}${x.toFixed(1)},${y.toFixed(1)}`
  }).join(' ')
}

function setSort(col: typeof sortCol.value) {
  if (sortCol.value === col) sortAsc.value = !sortAsc.value
  else { sortCol.value = col; sortAsc.value = false }
}

onMounted(() => {
  fetchProcs()
  pollTimer = setInterval(fetchProcs, 5000) // refresh process list every 5s
  ws.value?.addEventListener('message', onWsMsg)
})

onUnmounted(() => {
  if (pollTimer) clearInterval(pollTimer)
  ws.value?.removeEventListener('message', onWsMsg)
})

// Re-attach WS listener when socket reconnects
watch(ws, (newWs, oldWs) => {
  oldWs?.removeEventListener('message', onWsMsg)
  newWs?.addEventListener('message', onWsMsg)
})
</script>

<template>
  <div class="tm-root">
    <!-- ── Vitals Dashboard ─────────────────────────────────────────── -->
    <div class="vitals-row">
      <!-- CPU Gauge -->
      <div class="gauge-card">
        <div class="gauge-header">
          <span class="gauge-label">CPU</span>
          <span class="gauge-value cpu">{{ vitals ? vitals.cpu_percent.toFixed(1) : '—' }}%</span>
        </div>
        <svg class="sparkline" viewBox="0 0 200 40" preserveAspectRatio="none">
          <path :d="sparklinePath(cpuHistory, 200, 40)" fill="none" stroke="url(#cpuGrad)" stroke-width="1.5" />
          <defs><linearGradient id="cpuGrad" x1="0" y1="0" x2="1" y2="0"><stop offset="0" stop-color="#60a5fa"/><stop offset="1" stop-color="#a78bfa"/></linearGradient></defs>
        </svg>
        <div v-if="vitals" class="core-dots">
          <div v-for="(c, i) in vitals.cpu_per_core" :key="i" class="core-dot"
            :style="{ background: `hsl(${220 - c * 1.2}, 80%, ${65 - c * 0.3}%)`, opacity: 0.4 + c / 166 }"
            :title="`Core ${i}: ${c.toFixed(0)}%`" />
        </div>
      </div>

      <!-- RAM Gauge -->
      <div class="gauge-card">
        <div class="gauge-header">
          <span class="gauge-label">Memory</span>
          <span class="gauge-value ram">{{ vitals ? `${vitals.ram_used_mb}` : '—' }} / {{ vitals ? vitals.ram_total_mb : '—' }} MB</span>
        </div>
        <svg class="sparkline" viewBox="0 0 200 40" preserveAspectRatio="none">
          <path :d="sparklinePath(ramHistory, 200, 40)" fill="none" stroke="url(#ramGrad)" stroke-width="1.5" />
          <defs><linearGradient id="ramGrad" x1="0" y1="0" x2="1" y2="0"><stop offset="0" stop-color="#34d399"/><stop offset="1" stop-color="#22d3ee"/></linearGradient></defs>
        </svg>
        <div class="ram-bar-track">
          <div class="ram-bar-fill" :style="{ width: ramPercent + '%' }" />
        </div>
      </div>

      <!-- Swap -->
      <div class="gauge-card mini">
        <div class="gauge-header">
          <span class="gauge-label">Swap</span>
          <span class="gauge-value swap">{{ vitals ? `${vitals.swap_used_mb} / ${vitals.swap_total_mb} MB` : '—' }}</span>
        </div>
      </div>

      <!-- Disk I/O -->
      <div class="gauge-card mini">
        <div class="gauge-header">
          <span class="gauge-label">Disk I/O</span>
          <span class="gauge-value disk">↑{{ fmtBytes(diskWriteRate) }} ↓{{ fmtBytes(diskReadRate) }}</span>
        </div>
      </div>
    </div>

    <!-- ── Process Table ────────────────────────────────────────────── -->
    <div class="toolbar">
      <input v-model="filter" placeholder="Filter processes…" class="filter-input" />
      <button @click="fetchProcs" class="refresh-btn" :class="{ spin: loading }">⟳</button>
      <span class="proc-count">{{ filteredProcs.length }} processes</span>
    </div>

    <div class="table-wrap">
      <table>
        <thead>
          <tr>
            <th @click="setSort('pid')" class="sortable" :class="{ active: sortCol === 'pid' }">PID {{ sortCol === 'pid' ? (sortAsc ? '↑' : '↓') : '' }}</th>
            <th @click="setSort('name')" class="sortable" :class="{ active: sortCol === 'name' }">Name {{ sortCol === 'name' ? (sortAsc ? '↑' : '↓') : '' }}</th>
            <th @click="setSort('cpu')" class="sortable" :class="{ active: sortCol === 'cpu' }">CPU % {{ sortCol === 'cpu' ? (sortAsc ? '↑' : '↓') : '' }}</th>
            <th @click="setSort('mem_mb')" class="sortable" :class="{ active: sortCol === 'mem_mb' }">Mem MB {{ sortCol === 'mem_mb' ? (sortAsc ? '↑' : '↓') : '' }}</th>
            <th>Action</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="p in filteredProcs" :key="p.pid" :class="{ hot: p.cpu > 50 }">
            <td class="pid-cell">{{ p.pid }}</td>
            <td class="name-cell" :title="p.command">{{ p.name }}</td>
            <td class="cpu-cell">
              <div class="cpu-bar-bg"><div class="cpu-bar-fg" :style="{ width: Math.min(p.cpu, 100) + '%' }" /></div>
              <span>{{ p.cpu.toFixed(1) }}</span>
            </td>
            <td class="mem-cell">{{ p.mem_mb }}</td>
            <td>
              <button @click.stop="killProc(p.pid)" class="kill-btn" :disabled="killing === p.pid">
                {{ killing === p.pid ? '…' : '✕' }}
              </button>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>

<style scoped>
.tm-root {
  display: flex; flex-direction: column; height: 100%;
  background: rgba(12,12,16,0.95); color: #c8c8d0;
  font-family: 'JetBrains Mono', 'IBM Plex Mono', monospace;
  font-size: 12px; overflow: hidden;
}

/* ── Vitals ───────────────────────────────────────────────────────── */
.vitals-row {
  display: flex; gap: 8px; padding: 10px 12px 6px;
  border-bottom: 1px solid rgba(255,255,255,0.06);
  background: rgba(0,0,0,0.2); flex-shrink: 0;
}
.gauge-card {
  flex: 1; padding: 8px 10px;
  border-radius: 10px; background: rgba(255,255,255,0.03);
  border: 1px solid rgba(255,255,255,0.06);
  display: flex; flex-direction: column; gap: 4px; min-width: 0;
}
.gauge-card.mini { flex: 0.6; }
.gauge-header { display: flex; justify-content: space-between; align-items: center; }
.gauge-label { font-size: 10px; text-transform: uppercase; letter-spacing: 0.08em; color: rgba(255,255,255,0.35); font-weight: 600; }
.gauge-value { font-size: 11px; font-weight: 600; }
.gauge-value.cpu { color: #a78bfa; }
.gauge-value.ram { color: #34d399; }
.gauge-value.swap { color: #fb923c; }
.gauge-value.disk { color: #60a5fa; }

.sparkline { width: 100%; height: 28px; }

.core-dots { display: flex; gap: 2px; flex-wrap: wrap; }
.core-dot { width: 6px; height: 6px; border-radius: 50%; transition: all 300ms; }

.ram-bar-track { height: 3px; border-radius: 99px; background: rgba(255,255,255,0.07); overflow: hidden; }
.ram-bar-fill { height: 100%; border-radius: 99px; background: linear-gradient(90deg, #34d399, #22d3ee); transition: width 500ms ease; }

/* ── Toolbar ──────────────────────────────────────────────────────── */
.toolbar {
  display: flex; align-items: center; gap: 8px;
  padding: 6px 12px; flex-shrink: 0;
  border-bottom: 1px solid rgba(255,255,255,0.05);
}
.filter-input {
  flex: 1; padding: 5px 10px; border-radius: 8px;
  background: rgba(255,255,255,0.05); border: 1px solid rgba(255,255,255,0.08);
  color: white; font-size: 11px; outline: none; font-family: inherit;
  transition: border-color 150ms;
}
.filter-input:focus { border-color: rgba(96,165,250,0.5); }
.refresh-btn {
  background: rgba(255,255,255,0.06); border: 1px solid rgba(255,255,255,0.08);
  color: rgba(255,255,255,0.5); width: 28px; height: 28px;
  border-radius: 8px; cursor: pointer; font-size: 14px;
  transition: all 150ms; display: flex; align-items: center; justify-content: center;
}
.refresh-btn:hover { background: rgba(255,255,255,0.1); color: white; }
.refresh-btn.spin { animation: spin 500ms linear; }
@keyframes spin { to { transform: rotate(360deg); } }
.proc-count { font-size: 10px; color: rgba(255,255,255,0.25); white-space: nowrap; }

/* ── Table ────────────────────────────────────────────────────────── */
.table-wrap { flex: 1; overflow-y: auto; overflow-x: hidden; }
table { width: 100%; border-collapse: collapse; }
thead { position: sticky; top: 0; z-index: 2; }
th {
  padding: 6px 10px; text-align: left; font-size: 10px;
  text-transform: uppercase; letter-spacing: 0.08em;
  color: rgba(255,255,255,0.3); font-weight: 600;
  background: rgba(0,0,0,0.4); backdrop-filter: blur(8px);
  border-bottom: 1px solid rgba(255,255,255,0.06);
  user-select: none;
}
th.sortable { cursor: pointer; }
th.sortable:hover { color: rgba(255,255,255,0.6); }
th.active { color: #60a5fa; }
td {
  padding: 4px 10px; border-bottom: 1px solid rgba(255,255,255,0.03);
  white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
  max-width: 200px;
}
tr:hover { background: rgba(255,255,255,0.03); }
tr.hot { background: rgba(248,113,113,0.06); }
.pid-cell { color: rgba(255,255,255,0.3); font-size: 10px; }
.name-cell { color: rgba(255,255,255,0.85); font-weight: 500; }

.cpu-cell {
  display: flex; align-items: center; gap: 6px;
}
.cpu-bar-bg { width: 50px; height: 3px; border-radius: 99px; background: rgba(255,255,255,0.07); overflow: hidden; flex-shrink: 0; }
.cpu-bar-fg { height: 100%; border-radius: 99px; background: linear-gradient(90deg, #60a5fa, #f87171); transition: width 300ms; }
.mem-cell { color: rgba(255,255,255,0.5); }

.kill-btn {
  width: 22px; height: 22px; border-radius: 6px;
  background: rgba(248,113,113,0.1); border: 1px solid rgba(248,113,113,0.2);
  color: #f87171; cursor: pointer; font-size: 11px;
  display: flex; align-items: center; justify-content: center;
  transition: all 120ms;
}
.kill-btn:hover { background: rgba(248,113,113,0.25); color: white; }
.kill-btn:disabled { opacity: 0.3; cursor: not-allowed; }
</style>
