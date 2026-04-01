<script setup lang="ts">
import { ref, watch, onMounted, computed } from 'vue'
import {
  globalScreenshotState,
  broadcastScreenshotState,
  bumpScreenshotVersion,
} from '../../composables/useScreenshot'
import type { FileEntry } from '../../types'
import { spawnWindow, nextZ } from '../../composables/useWindows'
import { loadFiles, fileIcon, fileSizeHuman, formatDate } from '../../composables/useFiles'
import { windows, wsSend } from '../../composables/useWs'

const props = defineProps<{ winId: string }>()
const s = globalScreenshotState

const displays = ref<{name: string, description: string}[]>([])
const loadingDisplays = ref(false)

const files = ref<FileEntry[]>([])
const loadingFiles = ref(false)

// Fetch displays list from backend
const fetchDisplays = async () => {
  loadingDisplays.value = true
  try {
    const res = await fetch(`/api/displays?_=${Date.now()}`)
    if (res.ok) {
      displays.value = await res.json()
      if (!s.value.display && displays.value.length > 0) {
        selectDisplay(displays.value[0].name)
      }
    }
  } catch (err) {
    console.error('Failed to load displays:', err)
  } finally {
    loadingDisplays.value = false
  }
}

const fetchFiles = async (display: string) => {
  loadingFiles.value = true
  try {
    const res = await fetch(`/api/files?path=${encodeURIComponent('/tmp/latch/screenshots/' + display)}&_=${Date.now()}`)
    if (res.ok) {
      const entries: FileEntry[] = await res.json()
      // sort newest first
      files.value = entries.filter(e => !e.is_dir && e.name.endsWith('.png')).sort((a, b) => b.modified - a.modified)
    }
  } catch (err) {
    console.error('Failed to load files:', err)
  } finally {
    loadingFiles.value = false
  }
}

// Actions
const selectDisplay = (name: string) => {
  s.value.display = name
  s.value.openedImage = null
  s.value.scrollTop = 0
  bumpScreenshotVersion()
  broadcastScreenshotState()
  fetchFiles(name)
}

const openMedia = (path: string, name: string) => {
  // Find existing Media Viewer window OR spawn new one
  const existingId = Object.keys(windows.value).find(id => windows.value[id].app === 'media')
  
  if (existingId) {
    const win = windows.value[existingId]
    const media = [...(win.args?.media || [])]
    const index = media.findIndex(m => m.path === path)
    
    if (index === -1) {
      media.push({ path, name, type: 'image' })
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
      args: { media: [{ path, name, type: 'image' }], activeIndex: 0 }
    })
  }
}

const takeScreenshot = async () => {
  if (!s.value.display) return
  try {
    const res = await fetch(`/api/screenshots/take?_=${Date.now()}`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ display: s.value.display })
    })
    if (res.ok) {
      const path = await res.text()
      await fetchFiles(s.value.display)
      openMedia(path, path.split('/').pop() || 'Screenshot.png')
    } else {
      console.error('Take screenshot failed:', await res.text())
    }
  } catch (err) {
    console.error('Take screenshot error:', err)
  }
}

// Sync hooks
watch(() => s.value.display, (d) => { if (d) fetchFiles(d) })
watch(() => s.value.version, () => { if (s.value.display) fetchFiles(s.value.display) })

onMounted(() => {
  fetchDisplays()
  if (s.value.display) fetchFiles(s.value.display)
})

const openFileManager = () => {
  if (!s.value.display) return
  const path = `/tmp/latch/screenshots/${s.value.display}`
  const id = spawnWindow('files', { title: 'Finder' })
  loadFiles(id, path)
}

const formatDateTime = (ts: number) => {
  return new Date(ts * 1000).toLocaleString(undefined, {
    month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit'
  })
}
</script>

<template>
  <div class="flex flex-col h-full bg-[#0c0c0e] text-white overflow-hidden text-[13px] font-sans">
    
    <!-- Header -->
    <div class="flex-none flex items-center justify-between p-3 border-b border-white/5 bg-white/2">
      <div class="flex items-center gap-4">
        <div class="flex items-center gap-2">
          <span class="text-white/30 text-[10px] uppercase font-bold tracking-widest">Display</span>
          <select 
            :value="s.display" 
            @change="(e) => selectDisplay((e.target as HTMLSelectElement).value)"
            class="bg-white/5 hover:bg-white/10 px-2.5 py-1 rounded-md outline-none border border-white/10 transition-all cursor-pointer font-medium text-[12px] min-w-[120px]"
          >
            <option v-for="d in displays" :key="d.name" :value="d.name" class="bg-[#121214] text-white">
              {{ d.name }}
            </option>
          </select>
        </div>
      </div>

      <div class="flex items-center gap-2">
        <button @click="openFileManager" :disabled="!s.display" title="Show folder"
          class="p-2 rounded-md hover:bg-white/10 disabled:opacity-30 transition-all active:scale-95 border border-white/5">
          <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2v11z"/></svg>
        </button>
        <button 
          @click="takeScreenshot"
          :disabled="!s.display"
          class="flex items-center gap-2 px-4 py-1.5 bg-blue-500 hover:bg-blue-400 disabled:opacity-30 disabled:cursor-not-allowed rounded-md transition-all active:scale-95 font-bold text-[12px] shadow-[0_4px_12px_rgba(59,130,246,0.3)]"
        >
          <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="3"><line x1="12" y1="5" x2="12" y2="19"></line><line x1="5" y1="12" x2="19" y2="12"></line></svg>
          Snapshot
        </button>
      </div>
    </div>

    <!-- List View -->
    <div class="flex-1 overflow-y-auto">
      <div v-if="loadingFiles && files.length === 0" class="flex items-center justify-center h-full opacity-20 italic">Loading items...</div>
      <div v-else-if="files.length === 0" class="flex flex-col items-center justify-center h-full gap-2 opacity-20">
         <svg viewBox="0 0 24 24" width="48" height="48" fill="none" stroke="currentColor" stroke-width="1.5"><rect x="3" y="3" width="18" height="18" rx="2" ry="2"/><circle cx="8.5" cy="8.5" r="1.5"/><polyline points="21 15 16 10 5 21"/></svg>
         <div class="text-[14px]">Empty display gallery</div>
      </div>
      
      <div v-else class="flex flex-col">
        <!-- List Header -->
        <div class="flex items-center px-4 py-2 text-[10px] uppercase tracking-widest font-bold text-white/20 border-b border-white/5 sticky top-0 bg-[#0c0c0e]/95 backdrop-blur-md z-10">
           <span class="flex-1">Filename</span>
           <span class="w-32 text-right">Captured At</span>
           <span class="w-20 text-right">Size</span>
        </div>

        <div 
          v-for="f in files" 
          :key="f.path"
          @click="openMedia(f.path, f.name)"
          class="group flex items-center px-4 py-2.5 cursor-pointer hover:bg-white/5 border-b border-white/3 transition-colors"
        >
          <div class="flex-1 min-w-0 flex items-center gap-3">
             <div class="w-8 h-8 rounded-lg bg-white/5 flex items-center justify-center shrink-0 group-hover:bg-blue-500/20 group-hover:text-blue-400 transition-colors" v-html="fileIcon(f)"></div>
             <span class="font-medium truncate group-hover:text-blue-400 transition-colors">{{ f.name }}</span>
          </div>
          <span class="w-32 text-right text-[11px] text-white/30">{{ formatDateTime(f.modified) }}</span>
          <span class="w-20 text-right text-[11px] text-white/20">{{ fileSizeHuman(f.size) }}</span>
        </div>
      </div>
    </div>

    <!-- Status -->
    <div class="h-6 flex items-center px-3 border-t border-white/5 bg-white/1 text-[10px] text-white/20 font-mono">
       {{ files.length }} items in gallery
       <span class="ml-auto opacity-50">{{ s.display }}</span>
    </div>
  </div>
</template>

<style scoped>
/* Scrollbar */
::-webkit-scrollbar { width: 4px; }
::-webkit-scrollbar-thumb { background: rgba(255,255,255,0.1); border-radius: 10px; }
::-webkit-scrollbar-thumb:hover { background: rgba(255,255,255,0.2); }
</style>

<style scoped>
.fade-enter-active {
  transition: opacity 250ms var(--ease-out), transform 300ms var(--ease-out);
}
.fade-leave-active {
  transition: opacity 200ms var(--ease-out), transform 200ms var(--ease-out);
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
  transform: scale(0.96) translateY(20px);
}
</style>
