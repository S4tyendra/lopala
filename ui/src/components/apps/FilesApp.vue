<script setup lang="ts">
import { onMounted, onUnmounted, ref, computed, watch, nextTick } from 'vue'
import {
  globalFileState as s, initFileState, broadcastFileState,
  loadFiles, openEntry, renameFile, deleteFiles, copyFiles, moveFiles,
  fileIcon, fileSizeHuman, formatDate,
} from '../../composables/useFiles'
import { spawnWindow } from '../../composables/useWindows'
import { currentWorkspace } from '../../composables/useWs'

onMounted(() => initFileState())
onUnmounted(() => { if (s.value) s.value.contextMenu = null })

// ── Navigation ────────────────────────────────────────────────────────────────
const goUp = () => {
  const p = s.value.path
  const parent = p === '/' ? '/' : p.split('/').slice(0, -1).join('/') || '/'
  loadFiles(parent)
}

// ── Scroll Sync ───────────────────────────────────────────────────────────────
const listEl = ref<HTMLElement | null>(null)
let scrollTimeout: any = null
const onScroll = (e: Event) => {
  if (!s.value) return
  const top = (e.target as HTMLElement).scrollTop
  // Ignore scroll events if they match our incoming sync state
  if (Math.abs(s.value.scrollTop - top) < 2) return
  s.value.scrollTop = top
  if (scrollTimeout) clearTimeout(scrollTimeout)
  scrollTimeout = setTimeout(broadcastFileState, 100) // debounce
}

// Sync back to DOM when remote state changes
watch(() => s.value.scrollTop, (newVal) => {
  if (listEl.value && Math.abs(listEl.value.scrollTop - newVal) > 2) {
    listEl.value.scrollTo({ top: newVal, behavior: 'smooth' })
  }
})

// ── Selection ─────────────────────────────────────────────────────────────────
const toggleSelect = (e: MouseEvent, path: string) => {
  if (!s.value) return
  if (e.ctrlKey || e.metaKey) {
    if (s.value.selected.has(path)) s.value.selected.delete(path)
    else s.value.selected.add(path)
  } else {
    s.value.selected = new Set([path])
  }
  broadcastFileState()
}
const selectedPaths = computed(() => s.value ? Array.from(s.value.selected) : [])

// ── Context Menu ──────────────────────────────────────────────────────────────
const ctxEl = ref<HTMLElement | null>(null)

const openContextMenu = (e: MouseEvent, entry: any | null) => {
  e.preventDefault()
  if (!s.value) return
  if (entry && !s.value.selected.has(entry.path)) {
    s.value.selected = new Set([entry.path])
    broadcastFileState()
  }
  s.value.contextMenu = { x: e.clientX, y: e.clientY, entry }
}

const closeContextMenu = () => {
  if (s.value) s.value.contextMenu = null
}

const onDocClick = (e: MouseEvent) => {
  if (ctxEl.value && !ctxEl.value.contains(e.target as Node)) closeContextMenu()
}
onMounted(() => document.addEventListener('click', onDocClick, true))
onUnmounted(() => document.removeEventListener('click', onDocClick, true))

// ── Operations ────────────────────────────────────────────────────────────────
const copy = () => {
  if (!s.value) return
  s.value.clipboard = { op: 'copy', paths: selectedPaths.value }
  broadcastFileState()
  closeContextMenu()
}

const cut = () => {
  if (!s.value) return
  s.value.clipboard = { op: 'cut', paths: selectedPaths.value }
  broadcastFileState()
  closeContextMenu()
}

const paste = async () => {
  if (!s.value?.clipboard) return
  const { op, paths } = s.value.clipboard
  const dest = s.value.path
  if (op === 'copy') await copyFiles(paths, dest)
  else await moveFiles(paths, dest)
  if (op === 'cut') {
    s.value.clipboard = null
    broadcastFileState()
  }
  closeContextMenu()
}

const doDelete = async () => {
  if (!s.value) return
  const paths = selectedPaths.value
  if (!paths.length) return
  const names = paths.map(p => p.split('/').pop()).join(', ')
  if (!confirm(`Delete ${names}?`)) return
  await deleteFiles(paths)
  s.value.selected = new Set()
  broadcastFileState()
  closeContextMenu()
}

const startRename = () => {
  if (!s.value?.contextMenu?.entry) return
  const entry = s.value.contextMenu.entry
  s.value.renaming = { path: entry.path, name: entry.name }
  broadcastFileState()
  closeContextMenu()
  
  // auto focus input on next tick
  nextTick(() => {
    const el = document.getElementById('rename-input') as HTMLInputElement
    if (el) { el.focus(); el.select() }
  })
}

const onRenameInput = (e: Event) => {
  if (s.value?.renaming) {
    s.value.renaming.name = (e.target as HTMLInputElement).value
    broadcastFileState() // Live typig sync!
  }
}

const commitRename = async () => {
  if (!s.value?.renaming) return
  const { path, name } = s.value.renaming
  s.value.renaming = null
  broadcastFileState()
  const origName = path.split('/').pop()!
  if (name !== origName && name.trim()) await renameFile(path, name.trim())
}

const downloadEntry = (path: string) => {
  const a = document.createElement('a')
  a.href = `/api/files/download?path=${encodeURIComponent(path)}`
  a.download = path.split('/').pop()!
  a.click()
  closeContextMenu()
}

const terminalHere = () => {
  // spawn terminal at current path. We can't strictly tell the server PTY cwd without a command, 
  // but we can spawn the terminal and send a "cd" sequence manually!
  const id = spawnWindow('terminal', { title: 'Terminal' })
  closeContextMenu()
  // Tiny delay to ensure PTY is ready on backend, then send 'cd path\n'
  setTimeout(() => {
    import('../../composables/useWs').then(m => {
      m.wsSend({ type: 'PtyIn', id, data: `cd "${s.value.path}" && clear\r` })
    })
  }, 200)
}

// ── View + Path Breadcrumb ────────────────────────────────────────────────────
const pathParts = computed(() => {
  const parts = s.value.path.split('/').filter(Boolean)
  const result: { name: string; path: string }[] = [{ name: '/', path: '/' }]
  let acc = ''
  for (const p of parts) {
    acc += '/' + p
    result.push({ name: p, path: acc })
  }
  return result
})

const mimeToUrl = (path: string) => `/api/files/download?path=${encodeURIComponent(path)}`

// Helper for Vue template preview access
const previewEntry = computed(() => {
  if (!s.value.preview) return null
  return s.value.entries.find(e => e.path === s.value.preview!.path)
})
</script>

<template>
  <div class="absolute inset-0 flex flex-col" style="background:rgba(18,18,22,0.85)" @click="closeContextMenu" @contextmenu.prevent="openContextMenu($event, null)">

    <!-- Toolbar -->
    <div class="flex items-center gap-2 px-3 h-9 shrink-0 border-b" style="background:rgba(0,0,0,0.2);border-color:rgba(255,255,255,0.06)">
      <button @click.stop="goUp" title="Go up" class="px-2 py-1 rounded text-[12px] transition-[background] duration-100 hover:brightness-125" style="color:rgba(255,255,255,0.45)">←</button>

      <!-- Breadcrumb -->
      <div class="flex items-center gap-1 flex-1 overflow-hidden text-[11px]" style="color:rgba(255,255,255,0.35)">
        <template v-for="(part, i) in pathParts" :key="part.path">
          <button @click.stop="loadFiles(part.path)" class="hover:brightness-150 transition-[filter] duration-100 truncate max-w-[80px]">{{ part.name }}</button>
          <span v-if="i < pathParts.length - 1" class="opacity-40">/</span>
        </template>
      </div>

      <!-- Clipboard indicator -->
      <div v-if="s.clipboard" class="flex items-center gap-1 text-[10px] px-2 py-0.5 rounded shadow-sm" style="background:rgba(251,146,60,0.15);color:#fb923c">
        <span>{{ s.clipboard.op === 'copy' ? '📋' : '✂️' }}</span>
        <span>{{ s.clipboard.paths.length }} item{{ s.clipboard.paths.length > 1 ? 's' : '' }}</span>
        <button @click.stop="s.clipboard = null; broadcastFileState()" class="opacity-60 hover:opacity-100 ml-1">✕</button>
      </div>

      <!-- View toggle -->
      <div class="flex gap-1">
        <button @click.stop="s.viewMode = 'grid'" :style="s.viewMode==='grid'?'color:white;background:rgba(255,255,255,0.12)':'color:rgba(255,255,255,0.3)'" class="px-2 py-1 rounded text-[12px] transition-[background] duration-100">⊞</button>
        <button @click.stop="s.viewMode = 'list'" :style="s.viewMode==='list'?'color:white;background:rgba(255,255,255,0.12)':'color:rgba(255,255,255,0.3)'" class="px-2 py-1 rounded text-[12px] transition-[background] duration-100">☰</button>
      </div>
    </div>

    <!-- Main area: files + preview -->
    <div class="flex-1 flex overflow-hidden">

      <!-- File list -->
      <div ref="listEl" :class="['overflow-y-auto', s.preview ? 'w-[55%]' : 'flex-1']" 
        @scroll="onScroll"
        @click.stop="s.selected = new Set(); broadcastFileState()" 
        @contextmenu.prevent="openContextMenu($event, null)">

        <!-- Loading -->
        <div v-if="s.loading" class="flex items-center justify-center py-16 text-[12px]" style="color:rgba(255,255,255,0.2)">Loading…</div>

        <!-- Grid view -->
        <div v-else-if="s.viewMode === 'grid'" class="p-3 grid gap-2 content-start" style="grid-template-columns: repeat(auto-fill, minmax(80px, 1fr))">
          <div v-for="entry in s.entries" :key="entry.path"
            @click.stop="toggleSelect($event, entry.path)"
            @dblclick.stop="openEntry(entry)"
            @contextmenu.stop.prevent="openContextMenu($event, entry)"
            :style="s.selected.has(entry.path)
              ? 'background:rgba(96,165,250,0.2);outline:1.5px solid rgba(96,165,250,0.5);'
              : 'background:transparent'"
            class="flex flex-col items-center gap-1 p-2 rounded-xl cursor-pointer transition-[background] duration-100 hover:brightness-125 select-none"
            :class="s.selected.has(entry.path) ? '' : 'hover:bg-white/5'">

            <!-- Rename mode -->
            <span v-if="s.renaming?.path !== entry.path" class="text-2xl leading-none">{{ fileIcon(entry) }}</span>
            <input v-else
              id="rename-input"
              :value="s.renaming.name"
              @input="onRenameInput"
              @keyup.enter="commitRename"
              @keyup.escape="s.renaming = null; broadcastFileState()"
              @blur="commitRename"
              @click.stop
              class="text-center text-[10px] w-full rounded px-1 outline-none"
              style="background:rgba(96,165,250,0.2);color:white;border:1px solid #60a5fa"
            />

            <span v-if="s.renaming?.path !== entry.path" class="text-[10px] text-center w-full truncate" style="color:rgba(255,255,255,0.75)">{{ entry.name }}</span>
          </div>
        </div>

        <!-- List view -->
        <div v-else class="p-2 flex flex-col gap-px">
          <!-- Header -->
          <div class="flex items-center gap-3 px-3 py-1 text-[10px] font-semibold uppercase tracking-wider sticky top-0 bg-black/40 z-10 backdrop-blur-sm shadow-sm" style="color:rgba(255,255,255,0.25)">
            <span class="flex-1">Name</span>
            <span class="w-20 text-right">Size</span>
            <span class="w-24 text-right">Modified</span>
          </div>
          <div v-for="entry in s.entries" :key="entry.path"
            @click.stop="toggleSelect($event, entry.path)"
            @dblclick.stop="openEntry(entry)"
            @contextmenu.stop.prevent="openContextMenu($event, entry)"
            :style="s.selected.has(entry.path)
              ? 'background:rgba(96,165,250,0.18)'
              : ''"
            class="flex items-center gap-3 px-3 py-1.5 rounded-lg cursor-pointer transition-[background] duration-100 select-none"
            :class="s.selected.has(entry.path) ? '' : 'hover:bg-white/5'">

            <span class="text-sm leading-none shrink-0">{{ fileIcon(entry) }}</span>

            <span v-if="s.renaming?.path !== entry.path" class="flex-1 text-[12px] truncate" style="color:rgba(255,255,255,0.85)">{{ entry.name }}</span>
            <input v-else
              id="rename-input"
              :value="s.renaming.name"
              @input="onRenameInput"
              @keyup.enter="commitRename" @keyup.escape="s.renaming = null; broadcastFileState()" @blur="commitRename" @click.stop
              class="flex-1 text-[12px] rounded px-1 outline-none"
              style="background:rgba(96,165,250,0.2);color:white;border:1px solid #60a5fa" />

            <span class="w-20 text-right text-[10px] shrink-0" style="color:rgba(255,255,255,0.3)">{{ entry.is_dir ? '—' : fileSizeHuman(entry.size) }}</span>
            <span class="w-24 text-right text-[10px] shrink-0" style="color:rgba(255,255,255,0.25)">{{ formatDate(entry.modified) }}</span>
          </div>
        </div>
      </div>

      <!-- Preview panel -->
      <div v-if="s.preview" class="flex flex-col overflow-hidden border-l" style="width:45%;border-color:rgba(255,255,255,0.05)">
        <div class="h-8 flex items-center justify-between px-3 shrink-0 border-b" style="border-color:rgba(255,255,255,0.05);background:rgba(0,0,0,0.15)">
          <span class="text-[11px] truncate" style="color:rgba(255,255,255,0.4)">{{ s.preview.path.split('/').pop() }}</span>
          <div class="flex items-center gap-2 shrink-0 ml-2">
            <a :href="mimeToUrl(s.preview.path)" download class="text-[10px] hover:brightness-125 transition" style="color:#60a5fa">⬇ Save</a>
            <button @click="s.preview = null; broadcastFileState()" class="text-[10px] opacity-40 hover:opacity-80">✕</button>
          </div>
        </div>
        <div class="flex-1 overflow-auto bg-[#0a0a0c]">
          <div v-if="s.preview.content === '__no_preview__'" class="flex flex-col items-center justify-center h-full gap-3" style="color:rgba(255,255,255,0.2)">
            <span v-if="previewEntry" class="text-4xl">{{ fileIcon(previewEntry) }}</span>
            <span class="text-[12px]">No preview available</span>
          </div>
          <template v-else-if="s.preview.content === '__embed__' && previewEntry">
            <img v-if="/^image\//.test(previewEntry.mime)" :src="mimeToUrl(s.preview.path)" class="max-w-full max-h-full object-contain block m-auto p-2" />
            <video v-else-if="/^video\//.test(previewEntry.mime)" :src="mimeToUrl(s.preview.path)" controls class="w-full h-full object-contain" />
            <audio v-else-if="/^audio\//.test(previewEntry.mime)" :src="mimeToUrl(s.preview.path)" controls class="w-full mt-8 px-4" />
            <object v-else :data="mimeToUrl(s.preview.path)" class="w-full h-full" />
          </template>
          <pre v-else class="text-[11px] p-3 leading-relaxed overflow-auto h-full" style="font-family:'JetBrains Mono',monospace;color:rgba(100,255,140,0.8);white-space:pre-wrap;word-break:break-all">{{ s.preview.content }}</pre>
        </div>
      </div>
    </div>

    <!-- Status bar -->
    <div class="h-6 flex items-center px-3 gap-3 shrink-0 border-t text-[10px]" style="border-color:rgba(255,255,255,0.05);background:rgba(0,0,0,0.15);color:rgba(255,255,255,0.25)">
      <span>{{ s.entries.length ?? 0 }} items</span>
      <span v-if="s.selected.size">· {{ s.selected.size }} selected</span>
      <span class="ml-auto font-mono opacity-60">{{ s.path }}</span>
    </div>

    <!-- Context Menu -->
    <Teleport to="body">
      <div v-if="s.contextMenu" ref="ctxEl"
        class="fixed z-[99999] py-1 rounded-xl shadow-2xl overflow-hidden"
        style="min-width:200px;background:rgba(30,30,35,0.92);border:1px solid rgba(255,255,255,0.08);backdrop-filter:blur(24px) saturate(140%)"
        :style="{ left: s.contextMenu.x + 'px', top: s.contextMenu.y + 'px' }"
        @click.stop>

        <template v-if="s.contextMenu.entry">
          <button @click="openEntry(s.contextMenu.entry)" class="ctx-item">
            <span>{{ s.contextMenu.entry.is_dir ? '📂' : '👁️' }}</span>
            {{ s.contextMenu.entry.is_dir ? 'Open' : 'Preview' }}
          </button>
          <div class="ctx-divider"/>
          <button @click="copy" class="ctx-item"><span>📋</span>Copy</button>
          <button @click="cut" class="ctx-item"><span>✂️</span>Cut</button>
        </template>

        <button v-if="s.clipboard" @click="paste" class="ctx-item">
          <span>📥</span>Paste {{ s.clipboard.paths.length }} item{{ s.clipboard.paths.length > 1 ? 's' : '' }}
        </button>

        <template v-if="s.contextMenu.entry">
          <div class="ctx-divider"/>
          <button @click="startRename" class="ctx-item"><span>✏️</span>Rename</button>
          <button @click="s.contextMenu?.entry && downloadEntry(s.contextMenu.entry.path)" class="ctx-item"><span>⬇️</span>Download</button>
          <div class="ctx-divider"/>
          <button @click="doDelete" class="ctx-item ctx-danger"><span>🗑️</span>Delete</button>
        </template>

        <template v-else>
          <button v-if="s.clipboard" @click="paste" class="ctx-item"><span>📥</span>Paste</button>
          <div class="ctx-divider"/>
          <button @click="terminalHere" class="ctx-item"><span>⌨️</span>Open Terminal Here</button>
          <button @click="loadFiles(s.path)" class="ctx-item"><span>🔄</span>Refresh</button>
        </template>
      </div>
    </Teleport>
  </div>
</template>

<style scoped>
.ctx-item {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  padding: 6px 14px;
  text-align: left;
  font-size: 13px;
  font-weight: 500;
  color: rgba(255,255,255,0.85);
  cursor: pointer;
  transition: background 100ms;
}
.ctx-item:hover { background: rgba(96,165,250,0.15); color: white; }
.ctx-item > span { font-size: 15px; width: 18px; text-align: center; }
.ctx-danger { color: #f87171; }
.ctx-danger:hover { background: rgba(248,113,113,0.15); color: #fca5a5; }
.ctx-divider { height: 1px; background: rgba(255,255,255,0.06); margin: 4px 0; }
</style>
