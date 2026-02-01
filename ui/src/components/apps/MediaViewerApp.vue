<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue'
import { wsSend, windows } from '../../composables/useWs'

const props = defineProps<{ winId: string }>()

interface MediaTab {
  path: string
  name: string
  type: 'image' | 'video' | 'pdf' | 'other'
}

const win = computed(() => windows.value[props.winId])
const tabs = computed<MediaTab[]>(() => win.value?.args?.media || [])
const activeIndex = computed(() => win.value?.args?.activeIndex ?? 0)
const activeTab = computed(() => tabs.value[activeIndex.value])

const setActive = (index: number) => {
  wsSend({
    type: 'UpdateWindow',
    window: {
      ...win.value,
      args: { ...win.value.args, activeIndex: index }
    }
  })
}

const closeTab = (index: number) => {
  const newMedia = tabs.value.filter((_, i) => i !== index)
  if (newMedia.length === 0) {
    wsSend({ type: 'CloseWindow', id: props.winId })
    return
  }
  let newActive = activeIndex.value
  if (newActive >= newMedia.length) newActive = newMedia.length - 1
  wsSend({
    type: 'UpdateWindow',
    window: {
      ...win.value,
      args: { ...win.value.args, media: newMedia, activeIndex: newActive }
    }
  })
}

const mediaUrl = (path: string) => `/api/files/download?path=${encodeURIComponent(path)}`

const getMediaType = (path: string): MediaTab['type'] => {
  const ext = path.split('.').pop()?.toLowerCase()
  if (['png', 'jpg', 'jpeg', 'gif', 'webp', 'svg'].includes(ext || '')) return 'image'
  if (['mp4', 'webm', 'ogg', 'mov'].includes(ext || '')) return 'video'
  if (ext === 'pdf') return 'pdf'
  return 'other'
}

// Ensure first tab type is set if missing (for legacy or direct spawns)
onMounted(() => {
  if (tabs.value.length > 0 && !tabs.value[0].type) {
    const updated = tabs.value.map(t => ({ ...t, type: t.type || getMediaType(t.path) }))
    wsSend({
      type: 'UpdateWindow',
      window: { ...win.value, args: { ...win.value.args, media: updated } }
    })
  }
})
</script>

<template>
  <div class="media-root bg-[#0a0a0c] flex flex-col h-full overflow-hidden">
    <!-- Tabs Header -->
    <div class="tabs-header flex items-center bg-black/40 border-b border-white/5 overflow-x-auto scrollbar-none">
      <div v-for="(tab, i) in tabs" :key="tab.path"
        @click="setActive(i)"
        class="tab flex items-center gap-2 px-4 py-2 cursor-pointer border-r border-white/5 transition-all duration-200"
        :class="{ 'active bg-white/5 text-white': activeIndex === i, 'text-white/40 hover:bg-white/10': activeIndex !== i }">
        
        <span class="tab-icon w-4 h-4 flex items-center justify-center shrink-0">
           <svg v-if="tab.type === 'image'" viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2.5"><rect x="3" y="3" width="18" height="18" rx="2" ry="2"/><circle cx="8.5" cy="8.5" r="1.5"/><polyline points="21 15 16 10 5 21"/></svg>
           <svg v-else-if="tab.type === 'video'" viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2.5"><polygon points="23 7 16 12 23 17 23 7"/><rect x="1" y="5" width="15" height="14" rx="2" ry="2"/></svg>
           <svg v-else viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M13 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z"/><polyline points="13 2 13 9 20 9"/></svg>
        </span>
        
        <span class="tab-name text-[11px] font-medium truncate max-w-[120px]">{{ tab.name }}</span>
        
        <button @click.stop="closeTab(i)" class="close-btn p-1 rounded-md hover:bg-red-500/20 hover:text-red-400 transition-colors">
          <svg viewBox="0 0 24 24" width="10" height="10" fill="none" stroke="currentColor" stroke-width="3"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
        </button>
      </div>
    </div>

    <!-- Media Content -->
    <div class="media-body flex-1 relative overflow-hidden flex items-center justify-center p-4">
      <div v-if="activeTab" class="media-container w-full h-full flex items-center justify-center">
        <img v-if="activeTab.type === 'image'" 
          :src="mediaUrl(activeTab.path)" 
          class="max-w-full max-h-full object-contain rounded shadow-2xl transition-all duration-300 animate-in fade-in zoom-in-95" 
        />
        
        <video v-else-if="activeTab.type === 'video'" 
          :src="mediaUrl(activeTab.path)" 
          controls 
          autoplay
          class="max-w-full max-h-full rounded shadow-2xl outline-none" 
        />

        <embed v-else-if="activeTab.type === 'pdf'" 
          :src="mediaUrl(activeTab.path)" 
          type="application/pdf"
          class="w-full h-full rounded" 
        />
        
        <div v-else class="text-white/20 flex flex-col items-center gap-4">
           <svg viewBox="0 0 24 24" width="48" height="48" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z"/><polyline points="14 2 14 8 20 8"/></svg>
           <div class="text-[14px]">No preview available for this file type</div>
           <a :href="mediaUrl(activeTab.path)" download class="mt-2 px-6 py-2 bg-white/10 hover:bg-white/15 rounded-full text-[12px] font-bold transition-all active:scale-95">Download File</a>
        </div>
      </div>
      
      <div v-else class="text-white/10 text-[13px] italic">No active media</div>
    </div>
    
    <!-- Footer Info -->
    <div class="footer px-4 py-2 bg-black/20 border-t border-white/5 flex items-center justify-between text-[10px] text-white/30 font-mono">
      <template v-if="activeTab">
        <span>{{ activeTab.path }}</span>
        <span>Tab {{ activeIndex + 1 }} of {{ tabs.length }}</span>
      </template>
    </div>
  </div>
</template>

<style scoped>
.media-root { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; }
.tab.active { position: relative; }
.tab.active::after {
  content: ''; position: absolute; bottom: 0; left: 0; right: 0; height: 1.5px;
  background: white; box-shadow: 0 0 8px rgba(255,255,255,0.4);
}
.scrollbar-none::-webkit-scrollbar { display: none; }
</style>
