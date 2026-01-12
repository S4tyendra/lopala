<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { ws } from '../../composables/useWs'

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

const sortCol = ref<'cpu' | 'mem_mb' | 'pid' | 'name' | 'user'>('cpu')
const sortAsc = ref(false)
const filter = ref('')
const loading = ref(false)
const killing = ref<number | null>(null)

// ─── Computed ─────────────────────────────────────────────────────────────────
const filteredProcs = computed(() => {
  let list = [...procs.value]
  if (filter.value) {
    const q = filter.value.toLowerCase()
    list = list.filter(p => p.name.toLowerCase().includes(q) || p.command.toLowerCase().includes(q) || String(p.pid).includes(q) || p.user.toLowerCase().includes(q))
  }
  list.sort((a, b) => {
    const av = a[sortCol.value], bv = b[sortCol.value]
    const cmp = typeof av === 'string' ? (av as string).localeCompare(bv as string) : (av as number) - (bv as number)
    return sortAsc.value ? cmp : -cmp
  })
  return list
})

// ─── Fetch processes ──────────────────────────────────────────────────────────
let pollTimer: ReturnType<typeof setInterval> | null = null

async function fetchProcs() {
  loading.value = true
  try {
    const res = await fetch(`/api/system/ps?_=${Date.now()}`)
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
  if (b < 1024) return `${b} B`
  if (b < 1024 ** 2) return `${(b / 1024).toFixed(1)} KB`
  return `${(b / (1024 ** 2)).toFixed(1)} MB`
}

function setSort(col: typeof sortCol.value) {
  if (sortCol.value === col) sortAsc.value = !sortAsc.value
  else { sortCol.value = col; sortAsc.value = false }
}

onMounted(() => {
  fetchProcs()
  pollTimer = setInterval(fetchProcs, 3000) // refresh process list every 3s
  ws.value?.addEventListener('message', onWsMsg)
})

onUnmounted(() => {
  if (pollTimer) clearInterval(pollTimer)
  ws.value?.removeEventListener('message', onWsMsg)
})

watch(ws, (newWs, oldWs) => {
  oldWs?.removeEventListener('message', onWsMsg)
  newWs?.addEventListener('message', onWsMsg)
})
</script>

<template>
  <div class="flex flex-col h-full overflow-hidden"
    style="background:#0a0a0b; color:#c8c8d0;">
    
    <!-- ── Header Toolbar ────────────────────────────────────────────── -->
    <div class="toolbar">
      <div class="search-wrap">
        <span class="search-icon">🔍</span>
        <input v-model="filter" placeholder="Search processes…" class="filter-input" spellcheck="false" />
      </div>
      <div style="flex:1"></div>
      <button @click="fetchProcs" class="btn-icon" :class="{ spin: loading }" title="Refresh">⟳</button>
    </div>

    <!-- ── Process Table ────────────────────────────────────────────── -->
    <div class="table-wrap">
      <table>
        <thead>
          <tr>
            <th @click="setSort('name')" class="sortable col-name" :class="{ active: sortCol === 'name' }">Process Name <span>{{ sortCol === 'name' ? (sortAsc ? '↑' : '↓') : '' }}</span></th>
            <th @click="setSort('cpu')" class="sortable text-right" :class="{ active: sortCol === 'cpu' }">% CPU <span>{{ sortCol === 'cpu' ? (sortAsc ? '↑' : '↓') : '' }}</span></th>
            <th @click="setSort('mem_mb')" class="sortable text-right" :class="{ active: sortCol === 'mem_mb' }">Memory <span>{{ sortCol === 'mem_mb' ? (sortAsc ? '↑' : '↓') : '' }}</span></th>
            <th @click="setSort('user')" class="sortable" :class="{ active: sortCol === 'user' }">User <span>{{ sortCol === 'user' ? (sortAsc ? '↑' : '↓') : '' }}</span></th>
            <th @click="setSort('pid')" class="sortable" :class="{ active: sortCol === 'pid' }">PID <span>{{ sortCol === 'pid' ? (sortAsc ? '↑' : '↓') : '' }}</span></th>
            <th class="col-cmd">Action</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="p in filteredProcs" :key="p.pid" :class="{ 'high-cpu': p.cpu > 20 }">
            <td class="col-name" :title="p.command">
              <span class="proc-icon">⚙️</span>
              <span class="proc-name">{{ p.name }}</span>
            </td>
            <td class="text-right" :class="{ 'warn': p.cpu > 10, 'crit': p.cpu > 50 }">{{ p.cpu.toFixed(1) }}</td>
            <td class="text-right">{{ p.mem_mb }} MB</td>
            <td class="opacity-70">{{ p.user }}</td>
            <td class="opacity-50">{{ p.pid }}</td>
            <td class="col-cmd">
              <button @click.stop="killProc(p.pid)" class="kill-btn" :disabled="killing === p.pid" title="Force Quit">
                {{ killing === p.pid ? '…' : '✕' }}
              </button>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- ── Footer Vitals (Mac style bottom bar) ─────────────────────── -->
    <div class="footer-bar mt-auto">
      <div class="stat-group">
        <label>System CPU</label>
        <span class="val text-blue">{{ vitals ? vitals.cpu_percent.toFixed(1) : '—' }}%</span>
      </div>
      <div class="divider" />
      <div class="stat-group">
        <label>Memory Used</label>
        <span class="val text-green">{{ vitals ? vitals.ram_used_mb : '—' }} <small>MB</small></span>
      </div>
      <div class="stat-group">
        <label>Memory Total</label>
        <span class="val">{{ vitals ? vitals.ram_total_mb : '—' }} <small>MB</small></span>
      </div>
      <div class="divider" />
      <div class="stat-group">
        <label>Disk Read</label>
        <span class="val">{{ fmtBytes(diskReadRate) }}/s</span>
      </div>
      <div class="stat-group">
        <label>Disk Write</label>
        <span class="val">{{ fmtBytes(diskWriteRate) }}/s</span>
      </div>
      <div class="flex-1" />
      <div class="stat-group right">
        <label>Processes</label>
        <span class="val">{{ procs.length }}</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.taskmanager-root {
  display: flex; flex-direction: column; height: 100%;
  color: #c8c8d0; background: #111114;
}

/* ── Toolbar ──────────────────────────────────────────────────────── */
.toolbar {
  display: flex; align-items: center; gap: 8px;
  padding: 10px 16px; flex-shrink: 0;
  background: #2a2a32;
  border-bottom: 1px solid rgba(0,0,0,0.5);
  box-shadow: inset 0 -1px 0 rgba(255,255,255,0.05);
}
.search-wrap {
  position: relative; width: 260px; display: flex; align-items: center;
}
.search-icon {
  position: absolute; left: 10px; font-size: 12px; opacity: 0.5; pointer-events: none;
}
.filter-input {
  width: 100%; padding: 6px 10px 6px 30px; border-radius: 6px;
  background: rgba(0,0,0,0.3); border: 1px solid rgba(255,255,255,0.08);
  color: white; font-size: 13px; outline: none; font-family: inherit;
  transition: all 200ms var(--ease-out); box-shadow: inset 0 1px 2px rgba(0,0,0,0.2);
}
.filter-input:focus {
  background: rgba(0,0,0,0.5); border-color: rgba(96,165,250,0.5);
  box-shadow: inset 0 1px 2px rgba(0,0,0,0.2), 0 0 0 2px rgba(96,165,250,0.2);
}
.btn-icon {
  background: rgba(255,255,255,0.05); border: 1px solid rgba(255,255,255,0.08);
  color: white; width: 30px; height: 30px; border-radius: 6px;
  cursor: pointer; display: flex; align-items: center; justify-content: center;
  font-size: 16px; transition: all 150ms var(--ease-out); box-shadow: 0 1px 2px rgba(0,0,0,0.2);
}
.btn-icon:hover { background: rgba(255,255,255,0.12); border-color: rgba(255,255,255,0.15); }
.btn-icon:active { transform: scale(0.96); }
.btn-icon.spin { animation: spin 0.6s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }

/* ── Table ────────────────────────────────────────────────────────── */
.table-wrap { flex: 1; overflow-y: auto; overflow-x: hidden; background: #16161b; }
table { width: 100%; border-collapse: collapse; text-align: left; }
thead { position: sticky; top: 0; z-index: 2; }
th {
  padding: 6px 12px; font-size: 11px;
  color: rgba(255,255,255,0.6); font-weight: 500;
  background: #25252d; border-bottom: 1px solid rgba(0,0,0,0.6);
  border-right: 1px solid rgba(255,255,255,0.04);
  box-shadow: inset 0 -1px 0 rgba(255,255,255,0.03);
  user-select: none; white-space: nowrap;
}
th span { font-size: 9px; margin-left: 2px; opacity: 0.5; }
th:last-child { border-right: none; }
th.sortable { cursor: pointer; transition: background 150ms; }
th.sortable:hover { background: #2a2a32; color: white; }
th.active { color: white; }

td {
  padding: 5px 12px; border-bottom: 1px solid rgba(255,255,255,0.02);
  white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
  vertical-align: middle;
  transition: background 150ms var(--ease-out);
}
tr:hover { background: rgba(255,255,255,0.04); }
tr:nth-child(even) { background: rgba(255,255,255,0.012); }
tr.high-cpu td { background: rgba(96,165,250,0.05); }

.col-name { width: 40%; max-width: 250px; }
.col-cmd { width: 40px; text-align: center; }
.text-right { text-align: right; }

.proc-icon { font-size: 14px; margin-right: 8px; filter: grayscale(0.5); opacity: 0.8; }
.proc-name { font-weight: 500; }

.num-font { font-family: 'JetBrains Mono', 'IBM Plex Mono', monospace; font-size: 12px; }
.opacity-70 { opacity: 0.7; }
.opacity-50 { opacity: 0.5; }

.warn { color: #facc15; }
.crit { color: #f87171; font-weight: 600; }

/* Action Button */
.kill-btn {
  background: transparent; border: none; color: #f87171; cursor: pointer;
  width: 20px; height: 20px; border-radius: 4px; display: flex; align-items: center; justify-content: center;
  opacity: 0; font-size: 14px; transition: all 100ms;
}
tr:hover .kill-btn { opacity: 0.6; }
.kill-btn:hover { opacity: 1 !important; background: rgba(248,113,113,0.15); }
.kill-btn:disabled { opacity: 0.3 !important; cursor: not-allowed; }

/* ── Footer Vitals ────────────────────────────────────────────────── */
.footer-bar {
  display: flex; align-items: center; gap: 16px; flex-shrink: 0;
  padding: 0 16px; height: 44px;
  background: #2a2a32;
  border-top: 1px solid rgba(0,0,0,0.5);
  box-shadow: inset 0 1px 0 rgba(255,255,255,0.05);
}
.stat-group { display: flex; flex-direction: column; justify-content: center; gap: 2px; }
.stat-group.right { text-align: right; }
.stat-group label { font-size: 10px; text-transform: uppercase; color: rgba(255,255,255,0.4); font-weight: 600; letter-spacing: 0.05em; line-height: 1; }
.stat-group .val { font-size: 14px; font-weight: 600; line-height: 1; }
.stat-group small { font-size: 10px; opacity: 0.5; font-weight: normal; }

.divider { width: 1px; height: 24px; background: rgba(255,255,255,0.1); margin: 0 4px; }
.text-blue { color: #60a5fa; }
.text-green { color: #34d399; }
</style>
