<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { windows } from '../composables/useWs'
import { spawnWindow, focusWindow, broadcastWin } from '../composables/useWindows'

const APPS = [
  { id: 'files',    label: 'Finder',   bg: '#1a6ef5', path: '<path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/>' },
  { id: 'terminal', label: 'Terminal', bg: '#1a1a1a', path: '<polyline points="4 17 10 11 4 5"/><line x1="12" y1="19" x2="20" y2="19"/>' },
  { id: 'messages', label: 'Messages', bg: '#28a745', path: '<path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/>' },
  { id: 'canvas',   label: 'Canvas',   bg: '#7c3aed', path: '<path d="M12 21a9 9 0 0 1-9-9c0-5 4-9 9-9 5 0 9 4 9 9 0 2-1 3-3 3h-1c-1 0-2 1-2 2v1c0 1-1 2-2 2zm-5-9A1.5 1.5 0 1 1 5.5 12 1.5 1.5 0 0 1 7 12zm9 0a1.5 1.5 0 1 1-1.5-1.5A1.5 1.5 0 0 1 16 12zm-4-4a1.5 1.5 0 1 1-1.5-1.5A1.5 1.5 0 0 1 12 8z"/>' },
  { id: 'screenshot', label: 'Screenshot', bg: '#eab308', path: '<rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect><circle cx="8.5" cy="8.5" r="1.5"></circle><polyline points="21 15 16 10 5 21"></polyline>' },
  { id: 'screenview', label: 'Screen View', bg: '#0e7490', path: '<rect x="2" y="3" width="20" height="14" rx="2" ry="2"></rect><line x1="8" y1="21" x2="16" y2="21"></line><line x1="12" y1="17" x2="12" y2="21"></line><circle cx="12" cy="10" r="3"></circle>' },
  { id: 'taskmanager', label: 'Task Manager', bg: '#dc2626', path: '<rect x="3" y="3" width="7" height="7" rx="1"/><rect x="14" y="3" width="7" height="7" rx="1"/><rect x="3" y="14" width="7" height="7" rx="1"/><rect x="14" y="14" width="7" height="7" rx="1"/>' },
]

const hasWindow = (app: string) => Object.values(windows.value).some(w => w.app === app)

const open = (id: string) => {
  // Focus existing window of this app if it exists
  const existingId = Object.keys(windows.value).find(k => windows.value[k].app === id)
  if (existingId) {
    const win = windows.value[existingId]
    if (win.minimized) {
       win.minimized = false
       broadcastWin(win)
    }
    focusWindow(existingId)
    return
  }

  if (id === 'messages') spawnWindow('messages', { title: 'Messages', channel: 'global' })
  else if (id === 'canvas') {
    const canvasId = Math.random().toString(36).substring(7)
    spawnWindow('canvas', { title: 'Canvas', canvasId })
  } else if (id === 'terminal') {
    spawnWindow('terminal', { title: 'Terminal' })
  } else if (id === 'taskmanager') {
    spawnWindow('taskmanager', { title: 'Task Manager' })
  } else {
    spawnWindow(id, { title: id[0].toUpperCase() + id.slice(1) })
  }
}

// ── Magnification Logic ──────────────────────────────────────────────────────
const dockRef = ref<HTMLElement | null>(null)
const mouseX = ref(-1000)
const mouseY = ref(-1000)

const onMouseMove = (e: MouseEvent) => {
  mouseX.value = e.clientX
  mouseY.value = e.clientY
}
onMounted(() => window.addEventListener('mousemove', onMouseMove))
onUnmounted(() => window.removeEventListener('mousemove', onMouseMove))

function getScale(appId: string) {
  const el = document.getElementById(`dock-item-${appId}`)
  if (!el || !dockRef.value) return 1
  
  const rect = el.getBoundingClientRect()
  const centerX = rect.left + rect.width / 2
  const centerY = rect.top + rect.height / 2
  
  const dist = Math.sqrt((mouseX.value - centerX)**2 + (mouseY.value - centerY)**2)
  const maxDist = 140
  const maxScale = 1.6
  
  if (dist > maxDist) return 1
  const scale = 1 + (maxScale - 1) * (1 - dist / maxDist)
  return scale
}

const currentScales = ref<Record<string, number>>({})
let rafId: number
const updateScales = () => {
  APPS.forEach(a => currentScales.value[a.id] = getScale(a.id))
  rafId = requestAnimationFrame(updateScales)
}
onMounted(() => rafId = requestAnimationFrame(updateScales))
onUnmounted(() => cancelAnimationFrame(rafId))

</script>

<template>
  <div class="fixed bottom-3 left-0 right-0 flex justify-center pointer-events-none" style="z-index: 2147483630">
    <div ref="dockRef"
      class="pointer-events-auto flex items-end gap-1 px-3 py-2.5 rounded-[24px] border"
      style="background:rgba(18,18,22,0.4); backdrop-filter:blur(40px) saturate(180%); -webkit-backdrop-filter:blur(40px) saturate(180%); border-color:rgba(255,255,255,0.12); box-shadow:0 20px 50px rgba(0,0,0,0.5), inset 0 0.5px 0 rgba(255,255,255,0.15);">
      
      <button v-for="app in APPS" :key="app.id" :id="`dock-item-${app.id}`"
        @click="open(app.id)"
        class="group relative flex items-center justify-center cursor-pointer select-none active:scale-90 transition-[width,height,margin]"
        :style="{
          width: `${44 * (currentScales[app.id] || 1)}px`,
          height: `${44 * (currentScales[app.id] || 1)}px`,
          margin: `0 ${2 * (currentScales[app.id] || 1)}px`,
          marginBottom: `${10 * ((currentScales[app.id] || 1) - 1)}px`
        }">
        
        <div class="w-full h-full rounded-[12px] flex items-center justify-center transition-colors duration-200"
          :style="{ background: app.bg, boxShadow: '0 4px 12px rgba(0,0,0,0.3), inset 0 1px 0 rgba(255,255,255,0.1)' }">
          <svg xmlns="http://www.w3.org/2000/svg" width="50%" height="50%" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"
            class="drop-shadow-md" v-html="app.path"></svg>
        </div>

        <!-- Running indicator -->
        <div v-if="hasWindow(app.id)"
          class="absolute -bottom-1.5 w-1 h-1 rounded-full bg-white/90"
          style="left:50%; transform:translateX(-50%); box-shadow:0 0 8px rgba(255,255,255,0.8)">
        </div>

        <!-- Tooltip -->
        <div class="absolute -top-12 left-1/2 -translate-x-1/2 px-2.5 py-1.5 bg-black/80 backdrop-blur-md border border-white/10 rounded-lg text-[12px] font-bold text-white opacity-0 group-hover:opacity-100 transition-opacity duration-200 pointer-events-none whitespace-nowrap shadow-xl">
          {{ app.label }}
        </div>
      </button>
    </div>
  </div>
</template>

<style scoped>
button {
  transition: width 0.1s ease-out, height 0.1s ease-out, margin 0.1s ease-out;
  will-change: width, height, margin;
}
</style>
