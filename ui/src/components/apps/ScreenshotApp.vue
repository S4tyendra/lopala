<script setup lang="ts">
import { ref, watch, onMounted, computed, nextTick } from 'vue'
import {
  globalScreenshotState,
  broadcastScreenshotState,
  bumpScreenshotVersion,
} from '../../composables/useScreenshot'
import type { FileEntry } from '../../types'
import { spawnWindow } from '../../composables/useWindows'
import { loadFiles } from '../../composables/useFiles'

const props = defineProps<{ winId: string }>()

const s = globalScreenshotState

const displays = ref<{name: string, description: string}[]>([])
const loadingDisplays = ref(false)

const files = ref<FileEntry[]>([])
const loadingFiles = ref(false)

const scrollContainer = ref<HTMLElement | null>(null)
let isSettingScroll = false

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
    const res = await fetch(`/api/files?path=${encodeURIComponent('/tmp/lopala/screenshots/' + display)}&_=${Date.now()}`)
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

const openImage = (path: string) => {
  s.value.openedImage = path
  bumpScreenshotVersion()
  broadcastScreenshotState()
}

const closeImage = () => {
  s.value.openedImage = null
  bumpScreenshotVersion()
  broadcastScreenshotState()
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
      openImage(path)
    } else {
      console.error('Take screenshot failed:', await res.text())
    }
  } catch (err) {
    console.error('Take screenshot error:', err)
  }
}

// Sync hooks
watch(() => s.value.display, (d) => {
  if (d) fetchFiles(d)
})

watch(() => s.value.version, () => {
  if (s.value.display) {
    fetchFiles(s.value.display)
  }
})

watch(() => s.value.scrollTop, (val) => {
  if (isSettingScroll || !scrollContainer.value) return
  if (Math.abs(scrollContainer.value.scrollTop - val) > 5) {
    isSettingScroll = true
    scrollContainer.value.scrollTop = val
    setTimeout(() => { isSettingScroll = false }, 50)
  }
})

const onScroll = (e: Event) => {
  if (isSettingScroll) return
  const target = e.target as HTMLElement
  s.value.scrollTop = target.scrollTop
  broadcastScreenshotState()
}

onMounted(() => {
  fetchDisplays()
  if (s.value.display) {
    fetchFiles(s.value.display)
  }
})

// UI formatting
const formatTime = (ts: number) => {
  return new Date(ts * 1000).toLocaleString(undefined, {
    month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit', second: '2-digit'
  })
}

// Image viewer background close
const onBgClick = (e: MouseEvent) => {
  if ((e.target as HTMLElement).classList.contains('image-bg')) {
    closeImage()
  }
}

const openFileManager = () => {
  if (!s.value.display) return
  const path = `/tmp/lopala/screenshots/${s.value.display}`
  spawnWindow('files', { title: 'Finder' })
  // loadFiles handles broadcasting this new path to all peers automatically
  loadFiles(path)
}
</script>

<template>
  <div class="flex flex-col h-full bg-[#1e1e1e] text-white overflow-hidden text-[14px]">
    
    <!-- Top Bar: Display Selector & Take Button -->
    <div class="flex-none flex items-center justify-between p-3 border-b border-white/10" style="background: rgba(255,255,255,0.03)">
      <div class="flex items-center gap-2">
        <span class="text-white/50 text-[13px] uppercase tracking-wider font-semibold">Display:</span>
        <select 
          :value="s.display" 
          @change="(e) => selectDisplay((e.target as HTMLSelectElement).value)"
          class="bg-white/10 hover:bg-white/15 px-3 py-1.5 rounded-md outline-none border border-white/5 focus:border-blue-500/50 transition-all duration-200 var(--ease-out) cursor-pointer font-medium text-[13px]"
        >
          <option v-for="d in displays" :key="d.name" :value="d.name" class="bg-black text-white">
            {{ d.name }} - {{ d.description }}
          </option>
        </select>
        <span v-if="loadingDisplays" class="text-white/50 ml-2">...</span>
      </div>

      <div class="flex items-center gap-3">
        <button 
          @click="openFileManager"
          :disabled="!s.display"
          title="Open in File Manager"
          class="flex items-center gap-1.5 px-3 py-1.5 bg-white/10 hover:bg-white/15 disabled:opacity-30 disabled:cursor-not-allowed rounded-md transition-all duration-200 var(--ease-out) active:scale-95 text-[13px] font-medium border border-white/5"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path>
          </svg>
          <span class="hidden sm:inline">Files</span>
        </button>

        <button 
          @click="takeScreenshot"
          :disabled="!s.display"
          class="flex items-center gap-2 px-4 py-1.5 bg-blue-600 hover:bg-blue-500 disabled:opacity-30 disabled:cursor-not-allowed rounded-md transition-all duration-200 var(--ease-out) active:scale-95 font-medium shadow-[0_4px_12px_rgba(37,99,235,0.3)]"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <line x1="12" y1="5" x2="12" y2="19"></line>
            <line x1="5" y1="12" x2="19" y2="12"></line>
          </svg>
          <span>Take Screenshot</span>
        </button>
      </div>
    </div>

    <!-- Content Area: Image Grid -->
    <div class="flex-1 overflow-y-auto p-4" ref="scrollContainer" @scroll="onScroll">
      <div v-if="loadingFiles && files.length === 0" class="text-white/40 text-center py-10">
        Loading...
      </div>
      <div v-else-if="files.length === 0" class="text-white/40 text-center py-10">
        No screenshots found for this display
      </div>
      
      <div v-else class="grid grid-cols-[repeat(auto-fill,minmax(200px,1fr))] gap-4">
        <div 
          v-for="f in files" 
          :key="f.path"
          @click="openImage(f.path)"
          class="group relative aspect-video bg-black/40 rounded-xl overflow-hidden border border-white/5 hover:border-white/20 transition-all duration-300 var(--ease-out) cursor-pointer shadow-lg active:scale-[0.98]"
        >
          <img 
            :src="`/api/files/download?path=${encodeURIComponent(f.path)}&_=${Date.now()}`" 
            class="w-full h-full object-cover transition-transform duration-500 var(--ease-out) group-hover:scale-105"
            loading="lazy"
          />
          <div class="absolute inset-0 bg-gradient-to-t from-black/90 via-black/20 to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-300 var(--ease-out)">
            <div class="absolute bottom-3 left-3 right-3 flex justify-between items-center text-[11px]">
              <span class="text-white font-bold truncate drop-shadow-lg">{{ f.name }}</span>
              <span class="text-white/60 font-medium">{{ formatTime(f.modified) }}</span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Image Viewer Overlay -->
    <transition name="fade">
      <div 
        v-if="s.openedImage"
        class="absolute inset-0 z-50 flex items-center justify-center p-6 bg-black/90 backdrop-blur-sm image-bg"
        @click="onBgClick"
      >
        <div class="relative max-w-full max-h-full">
          <button 
            @click="closeImage"
            class="absolute -top-4 -right-4 w-8 h-8 rounded-full bg-white/20 hover:bg-white/40 text-white flex items-center justify-center backdrop-blur-md shadow-lg transition-colors z-10"
          >
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <line x1="18" y1="6" x2="6" y2="18"></line>
              <line x1="6" y1="6" x2="18" y2="18"></line>
            </svg>
          </button>
          
          <img 
            :src="`/api/files/download?path=${encodeURIComponent(s.openedImage)}&_=${Date.now()}`" 
            class="max-w-full max-h-[calc(100vh-100px)] object-contain rounded-md shadow-2xl border border-white/10"
          />
          
          <div class="mt-4 text-center text-white/50 text-[13px]">
            {{ s.openedImage.split('/').pop() }}
          </div>
        </div>
      </div>
    </transition>
  </div>
</template>

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
