<script setup lang="ts">
import type { AppWindow } from '../types'
import { focusWindow, closeWindow, toggleMaximize, startDrag, startResize, winScreenRect } from '../composables/useWindows'

const props = defineProps<{ win: AppWindow }>()

const rect = () => winScreenRect(props.win)

const onTitlebarMousedown = (e: MouseEvent) => {
  // Ignore if clicking traffic light area (leftmost 80px)
  if (e.offsetX < 80) return
  startDrag(e, props.win)
}
</script>

<template>
  <div
    class="absolute flex flex-col overflow-hidden"
    :style="{
      left: rect().x + 'px',
      top:  rect().y + 'px',
      width:  rect().w + 'px',
      height: rect().h + 'px',
      zIndex: win.z,
      borderRadius: win.maximized ? '0' : '12px',
      background: 'rgba(28,28,30,0.7)',
      backdropFilter: 'blur(40px) saturate(160%)',
      WebkitBackdropFilter: 'blur(40px) saturate(160%)',
      border: '1px solid rgba(255,255,255,0.13)',
      boxShadow: '0 30px 80px rgba(0,0,0,0.6), inset 0 1px 0 rgba(255,255,255,0.18)',
      willChange: 'transform',
    }"
    style="animation: winEnter 220ms cubic-bezier(0.23,1,0.32,1) both;"
    @mousedown="focusWindow(win.id)"
  >
    <!-- Title Bar -->
    <div
      class="flex items-center h-9 px-3 shrink-0 relative cursor-default border-b"
      style="border-color:rgba(255,255,255,0.05)"
      @mousedown="onTitlebarMousedown"
    >
      <!-- Traffic lights -->
      <div class="flex gap-2 z-10" @mousedown.stop>
        <button @click.stop="closeWindow(win.id)"
          class="w-3 h-3 rounded-full border flex items-center justify-center transition-[transform,filter] duration-100 active:scale-90 hover:brightness-120"
          style="background:#ff5f56;border-color:rgba(0,0,0,0.25)">
        </button>
        <button @click.stop="win.minimized = true"
          class="w-3 h-3 rounded-full border flex items-center justify-center transition-[transform,filter] duration-100 active:scale-90 hover:brightness-120"
          style="background:#ffbd2e;border-color:rgba(0,0,0,0.25)">
        </button>
        <button @click.stop="toggleMaximize(win)"
          class="w-3 h-3 rounded-full border flex items-center justify-center transition-[transform,filter] duration-100 active:scale-90 hover:brightness-120"
          style="background:#27c93f;border-color:rgba(0,0,0,0.25)">
        </button>
      </div>

      <!-- Centered title -->
      <div class="absolute inset-0 flex items-center justify-center pointer-events-none px-20">
        <span class="text-[12px] font-semibold truncate" style="color:rgba(255,255,255,0.8);letter-spacing:0.2px">{{ win.title }}</span>
      </div>
    </div>

    <!-- App content -->
    <div class="flex-1 overflow-hidden relative">
      <slot />
    </div>

    <!-- Resize handles (invisible, 4px hit area) -->
    <div class="absolute right-0 bottom-0 w-4 h-4 cursor-se-resize z-50"
      @mousedown.stop="startResize($event, win, 'se')" />
    <div class="absolute right-0 top-9 bottom-4 w-1 cursor-e-resize z-50"
      @mousedown.stop="startResize($event, win, 'e')" />
    <div class="absolute bottom-0 left-4 right-4 h-1 cursor-s-resize z-50"
      @mousedown.stop="startResize($event, win, 's')" />
  </div>
</template>
