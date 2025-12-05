<script setup lang="ts">
import { windows } from '../composables/useWs'
import { spawnWindow } from '../composables/useWindows'

const APPS = [
  { id: 'files',    label: 'Finder',   bg: '#1a6ef5', path: '<path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/>' },
  { id: 'terminal', label: 'Terminal', bg: '#1a1a1a', path: '<polyline points="4 17 10 11 4 5"/><line x1="12" y1="19" x2="20" y2="19"/>' },
  { id: 'messages', label: 'Messages', bg: '#28a745', path: '<path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/>' },
  { id: 'canvas',   label: 'Canvas',   bg: '#7c3aed', path: '<path d="M12 21a9 9 0 0 1-9-9c0-5 4-9 9-9 5 0 9 4 9 9 0 2-1 3-3 3h-1c-1 0-2 1-2 2v1c0 1-1 2-2 2zm-5-9A1.5 1.5 0 1 1 5.5 12 1.5 1.5 0 0 1 7 12zm9 0a1.5 1.5 0 1 1-1.5-1.5A1.5 1.5 0 0 1 16 12zm-4-4a1.5 1.5 0 1 1-1.5-1.5A1.5 1.5 0 0 1 12 8z"/>' },
  { id: 'screenshot', label: 'Screenshot', bg: '#eab308', path: '<rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect><circle cx="8.5" cy="8.5" r="1.5"></circle><polyline points="21 15 16 10 5 21"></polyline>' },
  { id: 'screenview', label: 'Screen View', bg: '#0e7490', path: '<rect x="2" y="3" width="20" height="14" rx="2" ry="2"></rect><line x1="8" y1="21" x2="16" y2="21"></line><line x1="12" y1="17" x2="12" y2="21"></line><circle cx="12" cy="10" r="3"></circle>' },
  { id: 'taskmanager', label: 'Task Manager', bg: '#dc2626', path: '<rect x="3" y="3" width="7" height="7" rx="1"/><rect x="14" y="3" width="7" height="7" rx="1"/><rect x="3" y="14" width="7" height="7" rx="1"/><rect x="14" y="14" width="7" height="7" rx="1"/>' },
  { id: 'editor', label: 'Code Editor', bg: '#0078d4', path: '<polyline points="16 18 22 12 16 6"/><polyline points="8 6 2 12 8 18"/><line x1="14" y1="4" x2="10" y2="20"/>' },
]

const hasWindow = (app: string) => Object.values(windows.value).some(w => w.app === app)

const open = (id: string) => {
  if (id === 'messages') spawnWindow('messages', { title: 'Messages', channel: 'global' })
  else if (id === 'canvas') {
    const canvasId = Math.random().toString(36).substring(7)
    spawnWindow('canvas', { title: 'Canvas', canvasId })
  } else if (id === 'terminal') {
    spawnWindow('terminal', { title: 'Terminal' })
  } else if (id === 'editor') {
    spawnWindow('editor', { title: 'Code Editor' })
  } else if (id === 'taskmanager') {
    spawnWindow('taskmanager', { title: 'Task Manager' })
  } else {
    spawnWindow(id, { title: id[0].toUpperCase() + id.slice(1) })
  }
}
</script>

<template>
  <!-- Dock sits above everything except the menubar cursor layer -->
  <div
    class="fixed bottom-3 left-0 right-0 flex justify-center pointer-events-none"
    style="z-index: 2147483630"
  >
    <div
      class="pointer-events-auto flex items-end gap-2 px-2 py-2 rounded-3xl border"
      style="
        background: rgba(18,18,22,0.52);
        backdrop-filter: blur(40px) saturate(160%);
        -webkit-backdrop-filter: blur(40px) saturate(160%);
        border-color: rgba(255,255,255,0.13);
        box-shadow: 0 12px 48px rgba(0,0,0,0.45), inset 0 1px 0 rgba(255,255,255,0.18);
      "
    >
      <button
        v-for="app in APPS" :key="app.id"
        @click="open(app.id)"
        :title="app.label"
        class="relative w-12 h-12 rounded-[14px] flex items-center justify-center cursor-pointer transition-[transform,margin] duration-200 origin-bottom select-none"
        style="box-shadow: 0 4px 12px rgba(0,0,0,0.35), inset 0 1px 0 rgba(255,255,255,0.15)"
        :style="{ background: app.bg }"
        @mouseenter="(e) => { (e.currentTarget as HTMLElement).style.transform='scale(1.3) translateY(-8px)'; (e.currentTarget as HTMLElement).style.margin='0 8px' }"
        @mouseleave="(e) => { (e.currentTarget as HTMLElement).style.transform=''; (e.currentTarget as HTMLElement).style.margin='' }"
      >
        <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"
          class="drop-shadow-sm" v-html="app.path"></svg>
        <!-- Running dot -->
        <div v-if="hasWindow(app.id)"
          class="absolute w-1 h-1 rounded-full bg-white/70"
          style="bottom:-5px;left:50%;transform:translateX(-50%)"></div>
      </button>
    </div>
  </div>
</template>
