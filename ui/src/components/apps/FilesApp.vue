<script setup lang="ts">
import { onMounted, onUnmounted, ref, computed, watch, nextTick } from 'vue'
import {
  getFileState, initFileState, broadcastFileState,
  loadFiles, openEntry, renameFile, deleteFiles, copyFiles, moveFiles,
  fileIcon, fileSizeHuman, formatDate,
} from '../../composables/useFiles'
import { spawnWindow, focusWindow, nextZ } from '../../composables/useWindows'
import { currentWorkspace, wsSend, windows } from '../../composables/useWs'
import FileUploader from './FileUploader.vue'

const props = defineProps<{ winId: string }>()
const s = computed(() => getFileState(props.winId))
const showUploader = ref(false)

onMounted(() => initFileState(props.winId))
onUnmounted(() => { if (s.value) s.value.contextMenu = null })

// ── Navigation ────────────────────────────────────────────────────────────────
const goUp = () => {
  const p = s.value.path
  const parent = p === '/' ? '/' : p.split('/').slice(0, -1).join('/') || '/'
  loadFiles(props.winId, parent)
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
  broadcastFileState(props.winId)
}
const selectedPaths = computed(() => s.value ? Array.from(s.value.selected) : [])

// ── Context Menu ──────────────────────────────────────────────────────────────
const ctxEl = ref<HTMLElement | null>(null)

const openContextMenu = (e: MouseEvent, entry: any | null) => {
  e.preventDefault()
  if (!s.value) return
  if (entry && !s.value.selected.has(entry.path)) {
    s.value.selected = new Set([entry.path])
    broadcastFileState(props.winId)
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
const open = async (entry: any) => {
  if (entry.is_dir) {
    await loadFiles(props.winId, entry.path)
    return
  }

  const m = entry.mime
  const match = m.match(/^(image\/|video\/|application\/pdf)/)
  if (match) {
    const type = m.startsWith('image/') ? 'image' : m.startsWith('video/') ? 'video' : 'pdf'
    // Find existing Media Viewer window OR spawn new one
    const existingId = Object.keys(windows.value).find(id => windows.value[id].app === 'media')
    
    if (existingId) {
      const win = windows.value[existingId]
      const media = [...(win.args?.media || [])]
      const index = media.findIndex(m => m.path === entry.path)
      
      if (index === -1) {
        media.push({ path: entry.path, name: entry.name, type })
        wsSend({
          type: 'UpdateWindow',
          window: { 
            ...win, 
            args: { ...win.args, media, activeIndex: media.length - 1 },
            z: nextZ()
          }
        })
      } else {
        wsSend({
          type: 'UpdateWindow',
          window: { 
            ...win, 
            args: { ...win.args, activeIndex: index },
            z: nextZ()
          }
        })
      }
    } else {
      spawnWindow('media', {
        title: 'Media Viewer',
        args: { media: [{ path: entry.path, name: entry.name, type }], activeIndex: 0 }
      })
    }
  } else {
    await openEntry(props.winId, entry)
  }
}

const copy = () => {
  if (!s.value) return
  s.value.clipboard = { op: 'copy', paths: selectedPaths.value }
  broadcastFileState(props.winId)
  closeContextMenu()
}

const cut = () => {
  if (!s.value) return
  s.value.clipboard = { op: 'cut', paths: selectedPaths.value }
  broadcastFileState(props.winId)
  closeContextMenu()
}

const paste = async () => {
  if (!s.value?.clipboard) return
  const { op, paths } = s.value.clipboard
  const dest = s.value.path
  if (op === 'copy') await copyFiles(props.winId, paths, dest)
  else await moveFiles(props.winId, paths, dest)
  if (op === 'cut') {
    s.value.clipboard = null
    broadcastFileState(props.winId)
  }
  closeContextMenu()
}

const doDelete = async () => {
  if (!s.value) return
  const paths = selectedPaths.value
  if (!paths.length) return
  const names = paths.map(p => p.split('/').pop()).join(', ')
  if (!confirm(`Delete ${names}?`)) return
  await deleteFiles(props.winId, paths)
  s.value.selected = new Set()
  broadcastFileState(props.winId)
  closeContextMenu()
}

const startRename = () => {
  if (!s.value?.contextMenu?.entry) return
  const entry = s.value.contextMenu.entry
  s.value.renaming = { path: entry.path, name: entry.name }
  broadcastFileState(props.winId)
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
    broadcastFileState(props.winId) // Live typig sync!
  }
}

const commitRename = async () => {
  if (!s.value?.renaming) return
  const { path, name } = s.value.renaming
  s.value.renaming = null
  broadcastFileState(props.winId)
  const origName = path.split('/').pop()!
  if (name !== origName && name.trim()) await renameFile(props.winId, path, name.trim())
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
    wsSend({ type: 'PtyIn', id, data: `cd "${s.value.path}" && clear\r` })
  }, 200)
}

const editorEntry = () => {
  if (!s.value?.contextMenu?.entry) return
  const entry = s.value.contextMenu.entry
  closeContextMenu()

  if (!entry.is_dir) {
    const existing = Object.values(windows.value).find(w => w.app === 'editor' && w.args?.mode === 'file')
    if (existing) {
      if (!existing.args.files) existing.args.files = []
      if (!existing.args.files.includes(entry.path)) existing.args.files.push(entry.path)
      existing.args.activeFile = entry.path
      wsSend({ type: 'UpdateWindow', window: existing })
      focusWindow(existing.id)
      return
    }
  }

  spawnWindow('editor', { 
    title: 'Code Editor', 
    args: { 
      mode: entry.is_dir ? 'dir' : 'file', 
      dirPath: entry.is_dir ? entry.path : undefined,
      files: entry.is_dir ? [] : [entry.path],
      activeFile: entry.is_dir ? undefined : entry.path
    } 
  })
}

const editorHere = () => {
  closeContextMenu()
  spawnWindow('editor', { 
    title: 'Code Editor', 
    args: { 
      mode: 'dir', 
      dirPath: s.value?.path || '/', 
      files: [] 
    } 
  })
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
          <button @click.stop="loadFiles(props.winId, part.path)" class="hover:brightness-150 transition-[filter] duration-100 truncate max-w-[80px]">{{ part.name }}</button>
          <span v-if="i < pathParts.length - 1" class="opacity-40">/</span>
        </template>
      </div>

      <!-- Clipboard indicator -->
      <div v-if="s.clipboard" class="flex items-center gap-1.5 text-[10px] px-2.5 py-1 rounded-full shadow-lg border border-[#fb923c]/30" style="background:rgba(251,146,60,0.15);color:#fb923c">
        <svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2"/><rect x="8" y="2" width="8" height="4" rx="1" ry="1"/></svg>
        <span class="font-bold tracking-tight">{{ s.clipboard.paths.length }} item{{ s.clipboard.paths.length > 1 ? 's' : '' }}</span>
        <button @click.stop="s.clipboard = null; broadcastFileState(props.winId)" class="opacity-60 hover:opacity-100 ml-1 transition-opacity">
          <svg viewBox="0 0 24 24" width="10" height="10" fill="none" stroke="currentColor" stroke-width="3"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
        </button>
      </div>

      <!-- View toggle -->
      <div class="flex gap-1">
        <button @click.stop="s.viewMode = 'grid'" :style="s.viewMode==='grid'?'color:white;background:rgba(255,255,255,0.12)':'color:rgba(255,255,255,0.3)'" class="p-1.5 rounded transition-all duration-100 active:scale-95">
          <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2.5"><rect x="3" y="3" width="7" height="7" rx="1"/><rect x="14" y="3" width="7" height="7" rx="1"/><rect x="3" y="14" width="7" height="7" rx="1"/><rect x="14" y="14" width="7" height="7" rx="1"/></svg>
        </button>
        <button @click.stop="s.viewMode = 'list'" :style="s.viewMode==='list'?'color:white;background:rgba(255,255,255,0.12)':'color:rgba(255,255,255,0.3)'" class="p-1.5 rounded transition-all duration-100 active:scale-95">
          <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2.5"><line x1="3" y1="6" x2="21" y2="6"/><line x1="3" y1="12" x2="21" y2="12"/><line x1="3" y1="18" x2="21" y2="18"/></svg>
        </button>
        <!-- Upload button -->
        <button
          @click.stop="showUploader = true"
          title="Upload files"
          class="px-2.5 py-1 rounded-md text-[11px] font-bold tracking-tight transition-all duration-100 flex items-center gap-1.5 active:scale-95"
          style="color:rgba(255,255,255,0.65);border:1px solid rgba(255,255,255,0.12)"
          :style="showUploader ? 'color:white;background:rgba(96,165,250,0.18);border-color:rgba(96,165,250,0.4)' : ''"
        >
          <svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="17 8 12 3 7 8"/><line x1="12" y1="3" x2="12" y2="15"/></svg>
          Upload
        </button>
      </div>
    </div>

    <!-- Main area: files + preview -->
    <div class="flex-1 flex overflow-hidden">

      <!-- File list -->
      <div ref="listEl" :class="['overflow-y-auto', s.preview ? 'w-[55%]' : 'flex-1']" 
        @scroll="onScroll"
        @click.stop="s.selected = new Set(); broadcastFileState(props.winId)" 
        @contextmenu.prevent="openContextMenu($event, null)">

        <!-- Loading -->
        <div v-if="s.loading" class="flex items-center justify-center py-16 text-[12px]" style="color:rgba(255,255,255,0.2)">Loading…</div>

        <!-- Grid view -->
        <div v-else-if="s.viewMode === 'grid'" class="p-3 grid gap-2 content-start" style="grid-template-columns: repeat(auto-fill, minmax(80px, 1fr))">
          <div v-for="entry in s.entries" :key="entry.path"
            @click.stop="toggleSelect($event, entry.path)"
            @dblclick.stop="openEntry(props.winId, entry)"
            @contextmenu.stop.prevent="openContextMenu($event, entry)"
            :style="s.selected.has(entry.path)
              ? 'background:rgba(96,165,250,0.2);outline:1.5px solid rgba(96,165,250,0.5);'
              : 'background:transparent'"
            class="flex flex-col items-center gap-1 p-2 rounded-xl cursor-pointer transition-all duration-200 ease-out select-none active:scale-95"
            :class="s.selected.has(entry.path) ? '' : 'hover:bg-white/5'">

            <!-- Rename mode -->
            <span v-if="s.renaming?.path !== entry.path" class="w-10 h-10 leading-none flex items-center justify-center text-white/70" v-html="fileIcon(entry)"></span>
            <input v-else
              id="rename-input"
              :value="s.renaming.name"
              @input="onRenameInput"
              @keyup.enter="commitRename"
              @keyup.escape="s.renaming = null; broadcastFileState(props.winId)"
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
            @dblclick.stop="openEntry(props.winId, entry)"
            @contextmenu.stop.prevent="openContextMenu($event, entry)"
            :style="s.selected.has(entry.path)
              ? 'background:rgba(96,165,250,0.18)'
              : ''"
            class="flex items-center gap-3 px-3 py-1.5 rounded-lg cursor-pointer transition-all duration-150 ease-out select-none active:scale-[0.99]"
            :class="s.selected.has(entry.path) ? '' : 'hover:bg-white/5'">

            <span class="w-5 h-5 flex items-center justify-center shrink-0 text-white/50" v-html="fileIcon(entry)"></span>

            <span v-if="s.renaming?.path !== entry.path" class="flex-1 text-[12px] truncate" style="color:rgba(255,255,255,0.85)">{{ entry.name }}</span>
            <input v-else
              id="rename-input"
              :value="s.renaming.name"
              @input="onRenameInput"
              @keyup.enter="commitRename" @keyup.escape="s.renaming = null; broadcastFileState(props.winId)" @blur="commitRename" @click.stop
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
            <button @click="s.preview = null; broadcastFileState(props.winId)" class="text-[10px] opacity-40 hover:opacity-80">✕</button>
          </div>
        </div>
        <div class="flex-1 overflow-auto bg-[#0a0a0c]">
          <div v-if="s.preview.content === '__no_preview__'" class="flex flex-col items-center justify-center h-full gap-4 text-white/20">
            <span v-if="previewEntry" class="w-16 h-16 opacity-40" v-html="fileIcon(previewEntry)"></span>
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
        class="fixed z-99999 py-1 rounded-xl shadow-2xl overflow-hidden"
        style="min-width:200px;background:rgba(30,30,35,0.92);border:1px solid rgba(255,255,255,0.08);backdrop-filter:blur(24px) saturate(140%)"
        :style="{ left: s.contextMenu.x + 'px', top: s.contextMenu.y + 'px' }"
        @click.stop>

        <template v-if="s.contextMenu.entry">
          <button @click="openEntry(props.winId, s.contextMenu.entry)" class="ctx-item">
            <span class="opacity-60" v-html="fileIcon(s.contextMenu.entry)"></span>
            {{ s.contextMenu.entry.is_dir ? 'Open Folder' : 'View Content' }}
          </button>
          <button @click="editorEntry" class="ctx-item">
            <span class="opacity-60"><svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M12 20h9"/><path d="M16.5 3.5a2.121 2.121 0 0 1 3 3L7 19l-4 1 1-4L16.5 3.5z"/></svg></span>
            Open in Code Editor
          </button>
          <div class="ctx-divider"/>
          <button @click="copy" class="ctx-item">
            <span class="opacity-60"><svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2.5"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg></span>
            Copy
          </button>
          <button @click="cut" class="ctx-item">
            <span class="opacity-60"><svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2.5"><circle cx="6" cy="6" r="3"/><circle cx="6" cy="18" r="3"/><line x1="20" y1="4" x2="8.12" y2="15.88"/><line x1="14.47" y1="14.48" x2="20" y2="20"/><polyline points="8.12 8.12 12 12"/></svg></span>
            Cut
          </button>
        </template>

        <button v-if="s.clipboard" @click="paste" class="ctx-item">
          <span class="opacity-60 text-orange-400"><svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="9 11 12 14 22 4"/><path d="M21 12v7a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11"/></svg></span>
          Paste {{ s.clipboard.paths.length }} item{{ s.clipboard.paths.length > 1 ? 's' : '' }}
        </button>

        <template v-if="s.contextMenu.entry">
          <div class="ctx-divider"/>
          <button @click="startRename" class="ctx-item">
            <span class="opacity-60"><svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M12 20h9"/><path d="M16.5 3.5a2.121 2.121 0 0 1 3 3L7 19l-4 1 1-4L16.5 3.5z"/></svg></span>
            Rename Item
          </button>
          <button @click="s.contextMenu?.entry && downloadEntry(s.contextMenu.entry.path)" class="ctx-item">
            <span class="opacity-60"><svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg></span>
            Download File
          </button>
          <div class="ctx-divider"/>
          <button @click="doDelete" class="ctx-item ctx-danger">
            <span class="opacity-60"><svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="3 6 5 6 21 6"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/><line x1="10" y1="11" x2="10" y2="17"/><line x1="14" y1="11" x2="14" y2="17"/></svg></span>
            Move to Trash
          </button>
        </template>

        <template v-else>
          <div class="ctx-divider" v-if="s.clipboard"/>
          <button @click="editorHere" class="ctx-item">
             <span class="opacity-60"><svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="16 18 22 12 16 6"/><polyline points="8 6 2 12 8 18"/></svg></span>
             Open Editor in Folder
          </button>
          <button @click="terminalHere" class="ctx-item">
             <span class="opacity-60"><svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="4 17 10 11 4 5"/><line x1="12" y1="19" x2="20" y2="19"/></svg></span>
             Open Terminal Here
          </button>
          <button @click="loadFiles(props.winId, s.path)" class="ctx-item">
             <span class="opacity-60"><svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M23 4v6h-6"/><path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"/></svg></span>
             Refresh Listing
          </button>
        </template>
      </div>
    </Teleport>

    <!-- ── File Uploader Modal ───────────────────────────────────────── -->
    <Teleport to="body">
      <Transition name="up-fade">
        <div
          v-if="showUploader"
          class="upload-overlay"
          @click.self="showUploader = false"
        >
          <FileUploader
            :dest-dir="s.path"
            @done="loadFiles(props.winId, s.path)"
            @close="showUploader = false"
          />
        </div>
      </Transition>
    </Teleport>
  </div>
</template>

<style scoped>
.ctx-item {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  padding: 8px 14px;
  text-align: left;
  font-size: 13px;
  font-weight: 500;
  color: rgba(255,255,255,0.85);
  cursor: pointer;
  transition: background 150ms var(--ease-out);
}
.ctx-item:hover { background: rgba(96,165,250,0.15); color: white; }
.ctx-item:active { transform: scale(0.98); }
.ctx-item > span { font-size: 15px; width: 18px; text-align: center; }
.ctx-danger { color: #f87171; }
.ctx-danger:hover { background: rgba(248,113,113,0.15); color: #fca5a5; }
.ctx-divider { height: 1px; background: rgba(255,255,255,0.06); margin: 4px 0; }

/* ── Upload overlay ─────────────────────────────────────────────────────────── */
.upload-overlay {
  position: fixed;
  inset: 0;
  z-index: 99998;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0,0,0,0.4);
  backdrop-filter: blur(8px);
}
.up-fade-enter-active, .up-fade-leave-active { transition: opacity 250ms var(--ease-out), transform 250ms var(--ease-out); }
.up-fade-enter-from, .up-fade-leave-to { opacity: 0; transform: scale(0.96) translateY(10px); }
</style>
