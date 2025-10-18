<script setup lang="ts">
import { windows } from '../composables/useWs'
import { spawnWindow } from '../composables/useWindows'

const APPS = [
  { id: 'files',    label: 'Finder',   bg: '#1a6ef5', emoji: '📁' },
  { id: 'terminal', label: 'Terminal', bg: '#1a1a1a', emoji: '⌨️' },
  { id: 'messages', label: 'Messages', bg: '#28a745', emoji: '💬' },
  { id: 'canvas',   label: 'Canvas',   bg: '#7c3aed', emoji: '🎨' },
]

const hasWindow = (app: string) => Object.values(windows.value).some(w => w.app === app)

const open = (id: string) => {
  if (id === 'messages') spawnWindow('messages', { title: 'Messages', channel: 'global' })
  else if (id === 'canvas') {
    const canvasId = Math.random().toString(36).substring(7)
    spawnWindow('canvas', { title: 'Canvas', canvasId })
  } else if (id === 'terminal') {
    spawnWindow('terminal', { title: 'Terminal' })
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
        <span class="text-2xl leading-none" style="font-family:'SamsungOneUI','SamsungColorEmoji','Noto Color Emoji',emoji">
          {{ app.emoji }}
        </span>
        <!-- Running dot -->
        <div v-if="hasWindow(app.id)"
          class="absolute w-1 h-1 rounded-full bg-white/70"
          style="bottom:-5px;left:50%;transform:translateX(-50%)"></div>
      </button>
    </div>
  </div>
</template>
