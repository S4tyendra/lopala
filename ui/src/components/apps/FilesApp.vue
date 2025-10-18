<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { fileStates, initFileState, loadFiles, openEntry, fileIcon, fileSizeHuman } from '../../composables/useFiles'

const props = defineProps<{ winId: string }>()
const s = () => fileStates.value[props.winId]

onMounted(() => {
  if (!fileStates.value[props.winId]) initFileState(props.winId, '/home')
})

const goUp = () => {
  const path = s()?.path ?? '/home'
  const parent = path === '/' ? '/' : path.split('/').slice(0, -1).join('/') || '/'
  loadFiles(props.winId, parent)
}

const setViewMode = (mode: 'grid' | 'list') => {
  if (s()) s()!.viewMode = mode
}

const mimeToEmbed = (path: string) => `/api/files/download?path=${encodeURIComponent(path)}`
</script>

<template>
  <div class="absolute inset-0 flex" style="background:rgba(20,20,20,0.6)">
    <!-- Sidebar -->
    <div class="w-40 flex flex-col shrink-0 border-r" style="background:rgba(15,15,15,0.5);border-color:rgba(255,255,255,0.06)">
      <p class="text-[10px] uppercase tracking-widest px-3 pt-3 pb-1 font-semibold" style="color:rgba(255,255,255,0.25)">Places</p>
      <button v-for="fav in [{n:'Home',p:'/home'},{n:'Root',p:'/'},{n:'Tmp',p:'/tmp'},{n:'Etc',p:'/etc'}]"
        :key="fav.p" @click="loadFiles(winId, fav.p)"
        :style="s()?.path === fav.p ? 'background:rgba(10,132,255,0.2);color:#60a5fa' : 'color:rgba(255,255,255,0.6)'"
        class="flex items-center gap-2 text-[12px] px-3 py-1.5 mx-2 rounded-lg transition-[background] duration-150 hover:brightness-125">
        <span>📁</span>{{ fav.n }}
      </button>

      <div class="mt-auto p-3 border-t" style="border-color:rgba(255,255,255,0.05)">
        <p class="text-[10px] break-all leading-relaxed" style="color:rgba(255,255,255,0.3);font-family:monospace">{{ s()?.path }}</p>
      </div>
    </div>

    <!-- Main -->
    <div class="flex-1 flex flex-col overflow-hidden">
      <!-- Toolbar -->
      <div class="h-9 flex items-center gap-2 px-3 shrink-0 border-b" style="background:rgba(0,0,0,0.15);border-color:rgba(255,255,255,0.05)">
        <button @click="goUp" class="text-[11px] px-2 py-1 rounded-lg hover:brightness-125 transition" style="color:rgba(255,255,255,0.5)">← Back</button>
        <span class="flex-1 text-[11px] truncate font-mono" style="color:rgba(255,255,255,0.35)">{{ s()?.path }}</span>
        <button @click="setViewMode('grid')" :style="s()?.viewMode==='grid'?'color:white;background:rgba(255,255,255,0.12)':'color:rgba(255,255,255,0.35)'" class="px-2 py-1 rounded text-[12px] transition-[background] duration-150">⊞</button>
        <button @click="setViewMode('list')" :style="s()?.viewMode==='list'?'color:white;background:rgba(255,255,255,0.12)':'color:rgba(255,255,255,0.35)'" class="px-2 py-1 rounded text-[12px] transition-[background] duration-150">☰</button>
      </div>

      <!-- Split: files + preview -->
      <div class="flex-1 flex overflow-hidden">
        <!-- Files -->
        <div :class="['overflow-y-auto', s()?.selected ? 'w-1/2' : 'flex-1']">
          <div v-if="s()?.loading" class="flex items-center justify-center py-16 text-[13px]" style="color:rgba(255,255,255,0.25)">Loading…</div>

          <!-- Grid -->
          <div v-else-if="s()?.viewMode==='grid'" class="p-3 grid grid-cols-4 gap-2 content-start">
            <button v-for="entry in s()?.entries" :key="entry.path"
              @click="openEntry(winId, entry)"
              @dblclick="entry.is_dir && loadFiles(winId, entry.path)"
              :style="s()?.selected?.path===entry.path?'background:rgba(10,132,255,0.25);outline:1px solid rgba(10,132,255,0.5)':''"
              class="flex flex-col items-center gap-1.5 p-2 rounded-xl cursor-pointer text-center transition-[background] duration-150 hover:brightness-125"
              style="color:rgba(255,255,255,0.8)">
              <span class="text-2xl leading-none">{{ fileIcon(entry) }}</span>
              <span class="text-[10px] w-full truncate">{{ entry.name }}</span>
            </button>
          </div>

          <!-- List -->
          <div v-else class="p-2">
            <button v-for="entry in s()?.entries" :key="entry.path"
              @click="openEntry(winId, entry)"
              @dblclick="entry.is_dir && loadFiles(winId, entry.path)"
              :style="s()?.selected?.path===entry.path?'background:rgba(10,132,255,0.2)':''"
              class="w-full flex items-center gap-3 px-3 py-1.5 rounded-lg cursor-pointer text-left transition-[background] duration-150 hover:brightness-125"
              style="color:rgba(255,255,255,0.8)">
              <span class="text-base leading-none shrink-0">{{ fileIcon(entry) }}</span>
              <span class="flex-1 text-[12px] truncate">{{ entry.name }}</span>
              <span class="text-[10px] shrink-0" style="color:rgba(255,255,255,0.3)">{{ entry.is_dir ? '—' : fileSizeHuman(entry.size) }}</span>
            </button>
          </div>
        </div>

        <!-- Preview -->
        <div v-if="s()?.selected" class="w-1/2 flex flex-col overflow-hidden border-l" style="border-color:rgba(255,255,255,0.05)">
          <div class="h-8 flex items-center justify-between px-3 shrink-0 border-b" style="border-color:rgba(255,255,255,0.05)">
            <span class="text-[11px] truncate" style="color:rgba(255,255,255,0.4)">{{ s()?.selected?.name }}</span>
            <a :href="mimeToEmbed(s()?.selected?.path ?? '')" download
              class="text-[10px] shrink-0 ml-2 transition-colors duration-150 hover:brightness-125" style="color:#60a5fa">⬇ Download</a>
          </div>
          <div class="flex-1 overflow-auto">
            <div v-if="s()?.preview==='__no_preview__'" class="flex flex-col items-center justify-center h-full gap-3" style="color:rgba(255,255,255,0.2)">
              <span class="text-4xl">{{ s()?.selected ? fileIcon(s()!.selected!) : '' }}</span>
              <span class="text-[12px]">No preview available</span>
            </div>
            <div v-else-if="s()?.preview?.startsWith('__embed__:')" class="h-full">
              <!-- Image -->
              <img v-if="/^image\//.test(s()?.selected?.mime??'')"
                :src="mimeToEmbed(s()!.preview!.slice(10))" class="max-w-full max-h-full object-contain m-auto block p-2" />
              <!-- Video -->
              <video v-else-if="/^video\//.test(s()?.selected?.mime??'')"
                :src="mimeToEmbed(s()!.preview!.slice(10))" controls class="w-full h-full object-contain" />
              <!-- Audio -->
              <audio v-else-if="/^audio\//.test(s()?.selected?.mime??'')"
                :src="mimeToEmbed(s()!.preview!.slice(10))" controls class="w-full mt-8 px-4" />
              <!-- PDF / other via object -->
              <object v-else :data="mimeToEmbed(s()!.preview!.slice(10))" class="w-full h-full" />
            </div>
            <pre v-else-if="s()?.preview" class="text-[11px] p-3 leading-relaxed overflow-auto h-full" style="font-family:monospace;color:rgba(100,255,140,0.8);white-space:pre-wrap;word-break:break-all">{{ s()?.preview }}</pre>
            <div v-else class="flex items-center justify-center h-full text-[12px]" style="color:rgba(255,255,255,0.15)">Select a file to preview</div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
