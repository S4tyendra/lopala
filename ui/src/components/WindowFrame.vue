<script setup lang="ts">
import { computed } from 'vue'
import type { AppWindow } from '../types'
import {
  focusWindow, closeWindow, toggleMaximize, startDrag, startResize,
  winScreenRect, minimizedSlots, broadcastWin
} from '../composables/useWindows'

const props = defineProps<{ win: AppWindow }>()

const rect = computed(() => winScreenRect(props.win))
const slot = computed(() => minimizedSlots.value.get(props.win.id))
const isMin = computed(() => props.win.minimized)

const onTitlebarMousedown = (e: MouseEvent) => {
  if (e.offsetX < 80) return
  startDrag(e, props.win)
}

const restoreMin = () => {
  if (!isMin.value) return
  props.win.minimized = false
  broadcastWin(props.win)
  focusWindow(props.win.id)
}
</script>

<template>
  <div
    class="absolute flex flex-col overflow-hidden transition-[left,top,width,height,opacity,background,border-radius] duration-250 ease-[var(--ease-out)]"
    :style="isMin ? {
      left: '20px',
      top: `calc(100% - ${90 + (slot ?? 0) * 50}px)`,
      width: '200px',
      height: '40px',
      zIndex: 2000000 + (slot ?? 0),
      borderRadius: '8px',
      background: 'rgba(28,28,30,0.8)',
      backdropFilter: 'blur(40px) saturate(160%)',
      border: '1px solid rgba(255,255,255,0.2)',
      boxShadow: '0 8px 32px rgba(0,0,0,0.5)',
      cursor: 'pointer',
      opacity: 0.9,
    } : {
      left: rect.x + 'px',
      top:  rect.y + 'px',
      width:  rect.w + 'px',
      height: rect.h + 'px',
      zIndex: win.z,
      borderRadius: win.maximized ? '0' : '12px',
      background: 'rgba(28,28,30,0.7)',
      backdropFilter: 'blur(40px) saturate(160%)',
      border: '1px solid rgba(255,255,255,0.13)',
      boxShadow: '0 30px 80px rgba(0,0,0,0.6), inset 0 1px 0 rgba(255,255,255,0.18)',
    }"
    style="animation: winEnter 250ms var(--ease-out) both; will-change: transform, left, top, width, height;"
    @mousedown="isMin ? restoreMin() : focusWindow(win.id)"
  >
    <!-- Title Bar -->
    <div
      class="flex items-center h-9 px-3 shrink-0 relative cursor-default border-b"
      style="border-color:rgba(255,255,255,0.05)"
      @mousedown="!isMin && onTitlebarMousedown($event)"
    >
      <!-- Traffic lights -->
      <div v-show="!isMin" class="flex gap-2 z-10" @mousedown.stop>
        <button @click.stop="closeWindow(win.id)"
          class="w-3.5 h-3.5 rounded-full border flex items-center justify-center transition-[transform,filter] duration-150 var(--ease-out) active:scale-90 hover:brightness-110"
          style="background:#ff5f56;border-color:rgba(0,0,0,0.25)">
        </button>
        <button @click.stop="win.minimized = true; broadcastWin(win)"
          class="w-3.5 h-3.5 rounded-full border flex items-center justify-center transition-[transform,filter] duration-150 var(--ease-out) active:scale-90 hover:brightness-110"
          style="background:#ffbd2e;border-color:rgba(0,0,0,0.25)">
        </button>
        <button @click.stop="toggleMaximize(win)"
          class="w-3.5 h-3.5 rounded-full border flex items-center justify-center transition-[transform,filter] duration-150 var(--ease-out) active:scale-90 hover:brightness-110"
          style="background:#27c93f;border-color:rgba(0,0,0,0.25)">
        </button>
      </div>

      <!-- Centered title -->
      <div class="absolute inset-0 flex items-center shadow-none text-white overflow-hidden pointer-events-none"
           :class="isMin ? 'justify-start px-4' : 'justify-center px-20'">
        <span class="text-[12px] font-semibold truncate" style="color:rgba(255,255,255,0.8);letter-spacing:0.2px">
           {{ isMin ? `Resume ${win.title || win.app}` : win.title }}
        </span>
      </div>
    </div>

    <!-- App content -->
    <div class="flex-1 overflow-hidden relative pointer-events-auto" :style="{ opacity: isMin ? 0 : 1, pointerEvents: isMin ? 'none' : 'auto' }">
      <slot />
    </div>

    <!-- Resize handles (invisible, 4px hit area) -->
    <div v-show="!isMin" class="absolute right-0 bottom-0 w-4 h-4 cursor-se-resize z-50"
      @mousedown.stop="startResize($event, win, 'se')" />
    <div v-show="!isMin" class="absolute right-0 top-9 bottom-4 w-1 cursor-e-resize z-50"
      @mousedown.stop="startResize($event, win, 'e')" />
    <div v-show="!isMin" class="absolute bottom-0 left-4 right-4 h-1 cursor-s-resize z-50"
      @mousedown.stop="startResize($event, win, 's')" />
  </div>
</template>
