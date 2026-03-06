<script setup lang="ts">
import { computed, ref, onMounted } from 'vue'
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

// One-shot entrance animation — only on initial mount, not on z-index focus changes
const justMounted = ref(false)
onMounted(() => {
  justMounted.value = true
  setTimeout(() => { justMounted.value = false }, 300)
})
</script>

<template>
  <div
    class="absolute flex flex-col overflow-hidden glass rounded-[14px] shadow-[0_22px_70px_rgba(0,0,0,0.6)] border border-white/10 ease-out transition-[left,top,width,height,opacity,background,border-radius] duration-250"
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
    style="will-change: transform, left, top, width, height;"
    :class="{ 'win-enter-anim': justMounted }"
    @mousedown="isMin ? restoreMin() : focusWindow(win.id)"
  >
    <!-- Window Drag handle / Title bar -->
    <div
      v-if="!isMin"
      class="flex-none flex items-center h-[34px] px-3 select-none cursor-default group/title border-b border-white/5"
      @mousedown="startDrag($event, win)"
      @dblclick="toggleMaximize(win)"
    >
      <!-- Traffic Lights -->
      <div class="flex items-center gap-2 w-[54px]">
        <button @click.stop="closeWindow(win.id)"
          class="w-3 h-3 rounded-full bg-[#ff5f56] border border-black/10 flex items-center justify-center group/btn active:scale-90 transition-all">
          <span class="text-[8px] text-black/40 opacity-0 group-hover/title:opacity-100 font-bold">✕</span>
        </button>
        <button @click.stop="broadcastWin({ ...win, minimized: true })"
          class="w-3 h-3 rounded-full bg-[#ffbd2e] border border-black/10 flex items-center justify-center group/btn active:scale-90 transition-all">
          <span class="text-[8px] text-black/40 opacity-0 group-hover/title:opacity-100 font-bold">−</span>
        </button>
        <button @click.stop="toggleMaximize(win)"
          class="w-3 h-3 rounded-full bg-[#27c93f] border border-black/10 flex items-center justify-center group/btn active:scale-90 transition-all">
          <span class="text-[8px] text-black/40 opacity-0 group-hover/title:opacity-100 font-bold">+</span>
        </button>
      </div>

      <!-- Window Title -->
      <div class="flex-1 text-center truncate px-2">
        <span class="text-[12px] font-bold tracking-tight opacity-70 text-white">{{ win.title || win.app }}</span>
      </div>

      <!-- App Specific Actions (Professional Buttons) -->
      <div :id="`actions-${win.id}`" class="flex items-center gap-1.5 min-w-[54px] justify-end">
        <slot name="actions" />
      </div>
    </div>

    <!-- Minimized Title Bar -->
    <div v-else class="flex items-center h-full px-4">
      <span class="text-[12px] font-semibold truncate text-white/80">
        {{ `Resume ${win.title || win.app}` }}
      </span>
    </div>

    <!-- App content -->
    <div class="flex-1 overflow-hidden relative" :style="{ opacity: isMin ? 0 : 1, pointerEvents: isMin ? 'none' : 'auto' }">
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

<style scoped>
.win-enter-anim {
  animation: winEnter 250ms var(--ease-out) both;
}
@keyframes winEnter {
  from { opacity: 0; transform: scale(0.96) translateY(10px); }
  to   { opacity: 1; transform: scale(1) translateY(0); }
}
</style>
