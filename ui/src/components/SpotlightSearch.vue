<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted, nextTick } from 'vue'
import { channels, wsSend } from '../composables/useWs'
import { nextZ } from '../composables/useWindows'
import { loadFiles } from '../composables/useFiles'

// ─── Visibility ───────────────────────────────────────────────────────────────
const open = ref(false)
const query = ref('')
const inputEl = ref<HTMLInputElement | null>(null)
const selectedIdx = ref(0)

// ─── App catalogue ────────────────────────────────────────────────────────────
const APPS = [
  { id: 'terminal',   label: 'Terminal',    icon: '>_',  bg: '#1a6fdf' },
  { id: 'files',      label: 'File Manager',icon: '📁',  bg: '#e8a020' },
  { id: 'messages',   label: 'Messages',    icon: '💬',  bg: '#28a745' },
  { id: 'canvas',     label: 'Canvas',      icon: '🎨',  bg: '#7c3aed' },
  { id: 'screenshot', label: 'Screenshot',  icon: '📸',  bg: '#eab308' },
  { id: 'screenview', label: 'Screen View', icon: '📺',  bg: '#0e7490' },
]

// ─── Search results ───────────────────────────────────────────────────────────
const fileResults = ref<{ path: string; name: string }[]>([])
const rgError = ref<string | null>(null)
const searching = ref(false)
let searchDebounce: ReturnType<typeof setTimeout> | null = null

const filteredApps = computed(() =>
  !query.value
    ? APPS
    : APPS.filter(a => a.label.toLowerCase().includes(query.value.toLowerCase()))
)

const filteredChannels = computed(() =>
  !query.value
    ? channels.value
    : channels.value.filter(c =>
        c.name.toLowerCase().includes(query.value.toLowerCase())
      )
)

type ResultItem =
  | { kind: 'app';     app: typeof APPS[0] }
  | { kind: 'channel'; ch: typeof channels.value[0] }
  | { kind: 'file';    path: string; name: string }

const allResults = computed<ResultItem[]>(() => [
  ...filteredApps.value.map(a => ({ kind: 'app' as const, app: a })),
  ...filteredChannels.value.map(c => ({ kind: 'channel' as const, ch: c })),
  ...fileResults.value.map(f => ({ kind: 'file' as const, path: f.path, name: f.name })),
])

// ─── File search ──────────────────────────────────────────────────────────────
const doSearch = async (q: string) => {
  if (!q.trim()) { fileResults.value = []; rgError.value = null; return }
  searching.value = true
  try {
    const res = await fetch(`/api/search?q=${encodeURIComponent(q)}&_=${Date.now()}`)
    if (res.status === 503) {
      const err = await res.json()
      rgError.value = err.message
      // Also emit to any active terminal so the user sees the install hint
      wsSend({ type: 'PtyIn', id: '__spotlight', data:
        'echo "⚠︎  ripgrep not found. Install: sudo pacman -S --noconfirm ripgrep"\r'
      })
      fileResults.value = []
    } else {
      rgError.value = null
      fileResults.value = await res.json()
    }
  } catch {
    fileResults.value = []
  } finally {
    searching.value = false
  }
}

watch(query, (q) => {
  selectedIdx.value = 0
  if (searchDebounce) clearTimeout(searchDebounce)
  searchDebounce = setTimeout(() => doSearch(q), 220)
})

// ─── Actions ──────────────────────────────────────────────────────────────────
const spawnApp = (appId: string, extra: Record<string, unknown> = {}) => {
  wsSend({
    type: 'SpawnWindow',
    window: {
      id: Math.random().toString(36).substring(7),
      app: appId,
      workspace: 0,
      x: 120 + Math.random() * 60,
      y: 80  + Math.random() * 40,
      w: appId === 'terminal' ? 700 : 860,
      h: appId === 'terminal' ? 460 : 560,
      z: nextZ(),
      minimized: false,
      maximized: false,
      title: APPS.find(a => a.id === appId)?.label ?? appId,
      ...extra,
    }
  })
  closeSpotlight()
}

const openChannel = (chId: string, chName: string) => {
  wsSend({
    type: 'SpawnWindow',
    window: {
      id: Math.random().toString(36).substring(7),
      app: 'messages',
      workspace: 0,
      x: 140, y: 80, w: 780, h: 540,
      z: nextZ(), minimized: false, maximized: false,
      title: `Messages — ${chName}`,
      channel: chId,
    }
  })
  closeSpotlight()
}

const openFile = (path: string) => {
  const dir = path.replace(/\/[^/]+$/, '') || '/'
  loadFiles(dir) // Tell Files app to natively load the path
  wsSend({
    type: 'SpawnWindow',
    window: {
      id: Math.random().toString(36).substring(7),
      app: 'files',
      workspace: 0,
      x: 130, y: 70, w: 900, h: 580,
      z: nextZ(), minimized: false, maximized: false,
      title: 'Finder',
      initialPath: dir,
    }
  })
  closeSpotlight()
}

const activateSelected = () => {
  const item = allResults.value[selectedIdx.value]
  if (!item) return
  if (item.kind === 'app')     spawnApp(item.app.id)
  if (item.kind === 'channel') openChannel(item.ch.id, item.ch.name)
  if (item.kind === 'file')    openFile(item.path)
}

// ─── Keyboard nav ─────────────────────────────────────────────────────────────
const onKeyDown = (e: KeyboardEvent) => {
  if (!open.value && (e.metaKey || e.ctrlKey) && e.key === 'k') {
    e.preventDefault(); openSpotlight(); return
  }
  if (!open.value) return
  if (e.key === 'Escape') { closeSpotlight(); return }
  if (e.key === 'ArrowDown')  { e.preventDefault(); selectedIdx.value = Math.min(selectedIdx.value + 1, allResults.value.length - 1) }
  if (e.key === 'ArrowUp')    { e.preventDefault(); selectedIdx.value = Math.max(selectedIdx.value - 1, 0) }
  if (e.key === 'Enter')      { e.preventDefault(); activateSelected() }
}

const openSpotlight = async () => {
  open.value = true
  query.value = ''
  fileResults.value = []
  rgError.value = null
  selectedIdx.value = 0
  await nextTick()
  inputEl.value?.focus()
}

const closeSpotlight = () => {
  open.value = false
  query.value = ''
}

// ─── Lifecycle ────────────────────────────────────────────────────────────────
onMounted(() => {
  window.addEventListener('keydown', onKeyDown)
})

watch(open, async (v) => {
  if (v) {
    await nextTick()
    inputEl.value?.focus()
  }
})

onUnmounted(() => {
  window.removeEventListener('keydown', onKeyDown)
})
</script>

<template>
  <Teleport to="body">
    <Transition name="spotlight">
      <div v-if="open"
        class="fixed inset-0 flex items-start justify-center pt-[18vh]"
        style="z-index:2147483646; background:rgba(0,0,0,0.45); backdrop-filter:blur(2px)"
        @mousedown.self="closeSpotlight">

        <!-- Glass modal -->
        <div class="relative w-[620px] max-w-[92vw] rounded-2xl overflow-hidden shadow-[0_12px_48px_rgba(0,0,0,0.5)] bg-slate-900/40 backdrop-blur-2xl border border-white/10"
          style="transform: translateZ(0);">

          <!-- UI layer (above canvas) -->
          <div class="relative z-10">

            <!-- Search input -->
            <div class="flex items-center gap-3 px-5 py-4 border-b"
              style="border-color:rgba(255,255,255,0.1)">
              <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24"
                fill="none" stroke="rgba(255,255,255,0.5)" stroke-width="2"
                stroke-linecap="round" stroke-linejoin="round">
                <circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/>
              </svg>
              <input ref="inputEl" v-model="query"
                placeholder="Search apps, files, channels…"
                class="flex-1 bg-transparent outline-none text-[15px] text-white placeholder-white/30 font-medium"
              />
              <div v-if="searching" class="w-3.5 h-3.5 border-2 rounded-full animate-spin flex-none"
                style="border-color:rgba(255,255,255,0.3); border-top-color:rgba(255,255,255,0.8)" />
              <kbd class="text-[10px] px-1.5 py-0.5 rounded font-mono flex-none"
                style="background:rgba(255,255,255,0.1); color:rgba(255,255,255,0.4); border:1px solid rgba(255,255,255,0.15)">
                ESC
              </kbd>
            </div>

            <!-- rg error -->
            <div v-if="rgError"
              class="flex items-start gap-3 px-5 py-3 text-[12px]"
              style="background:rgba(239,68,68,0.12); border-bottom:1px solid rgba(239,68,68,0.2); color:#fca5a5">
              <span class="text-[16px] flex-none">⚠︎</span>
              <div>
                <div class="font-semibold">ripgrep not found</div>
                <div class="opacity-70 mt-0.5">Install hint sent to terminal →
                  <code class="font-mono" style="color:#fca5a5">sudo pacman -S --noconfirm ripgrep</code>
                </div>
              </div>
            </div>

            <!-- Results -->
            <div class="py-2 max-h-[52vh] overflow-y-auto" style="scrollbar-width:none">

              <!-- Section: Apps -->
              <div v-if="filteredApps.length">
                <div class="px-5 pt-3 pb-1 text-[10px] uppercase tracking-widest font-semibold"
                  style="color:rgba(255,255,255,0.3)">Applications</div>
                <button v-for="(item, gi) in filteredApps" :key="item.id"
                  @mouseenter="selectedIdx = gi"
                  @click="spawnApp(item.id)"
                  class="w-full flex items-center gap-3 px-5 py-2.5 text-left transition-colors duration-75 text-[13px]"
                  :style="selectedIdx === gi ? 'background:rgba(255,255,255,0.12)' : ''">
                  <span class="w-7 h-7 rounded-lg flex items-center justify-center flex-none text-[14px]"
                    :style="`background:${item.bg}33; border:1px solid ${item.bg}55`">
                    {{ item.icon }}
                  </span>
                  <span class="text-white font-medium">{{ item.label }}</span>
                  <span class="ml-auto text-[11px] opacity-30">Open app</span>
                </button>
              </div>

              <!-- Section: Channels -->
              <div v-if="filteredChannels.length">
                <div class="px-5 pt-3 pb-1 text-[10px] uppercase tracking-widest font-semibold"
                  style="color:rgba(255,255,255,0.3)">Channels</div>
                <button v-for="(ch, ci) in filteredChannels" :key="ch.id"
                  @mouseenter="selectedIdx = filteredApps.length + ci"
                  @click="openChannel(ch.id, ch.name)"
                  class="w-full flex items-center gap-3 px-5 py-2.5 text-left transition-colors duration-75 text-[13px]"
                  :style="selectedIdx === filteredApps.length + ci ? 'background:rgba(255,255,255,0.12)' : ''">
                  <span class="w-7 h-7 rounded-lg flex items-center justify-center flex-none text-[12px]"
                    style="background:rgba(40,167,69,0.2); border:1px solid rgba(40,167,69,0.4)">
                    #
                  </span>
                  <span class="text-white font-medium">{{ ch.name }}</span>
                  <span class="ml-auto text-[11px] opacity-30">Switch</span>
                </button>
              </div>

              <!-- Section: Files -->
              <div v-if="fileResults.length">
                <div class="px-5 pt-3 pb-1 text-[10px] uppercase tracking-widest font-semibold"
                  style="color:rgba(255,255,255,0.3)">Files</div>
                <button v-for="(f, fi) in fileResults" :key="f.path"
                  @mouseenter="selectedIdx = filteredApps.length + filteredChannels.length + fi"
                  @click="openFile(f.path)"
                  class="w-full flex items-center gap-3 px-5 py-2.5 text-left transition-colors duration-75 text-[13px]"
                  :style="selectedIdx === filteredApps.length + filteredChannels.length + fi ? 'background:rgba(255,255,255,0.12)' : ''">
                  <span class="w-7 h-7 rounded-lg flex items-center justify-center flex-none text-[12px]"
                    style="background:rgba(99,102,241,0.2); border:1px solid rgba(99,102,241,0.35)">
                    📄
                  </span>
                  <div class="flex-1 min-w-0">
                    <div class="text-white font-medium truncate">{{ f.name }}</div>
                    <div class="text-[11px] truncate opacity-35 font-mono mt-0.5">{{ f.path }}</div>
                  </div>
                  <span class="ml-2 text-[11px] opacity-30 flex-none">Open</span>
                </button>
              </div>

              <!-- Empty state -->
              <div v-if="query && !searching && !allResults.length && !rgError"
                class="py-8 text-center text-[13px]"
                style="color:rgba(255,255,255,0.25)">
                No results for "<span class="font-mono">{{ query }}</span>"
              </div>

              <!-- Hint when empty -->
              <div v-if="!query"
                class="px-5 py-3 text-[11px] flex items-center gap-6 border-t mt-1"
                style="border-color:rgba(255,255,255,0.06); color:rgba(255,255,255,0.2)">
                <span><kbd class="font-mono">↑↓</kbd> navigate</span>
                <span><kbd class="font-mono">↵</kbd> open</span>
                <span><kbd class="font-mono">Esc</kbd> close</span>
                <span class="ml-auto">Type to search files via <code class="font-mono">rg</code></span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.spotlight-enter-active {
  transition: opacity 250ms var(--ease-out), transform 320ms var(--ease-out);
}
.spotlight-leave-active {
  transition: opacity 180ms var(--ease-out), transform 180ms var(--ease-out);
}
.spotlight-enter-from,
.spotlight-leave-to {
  opacity: 0;
  transform: translateY(-20px) scale(0.985);
}
</style>
