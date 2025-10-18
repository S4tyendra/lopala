<script setup lang="ts">
import { onMounted, ref, watch } from 'vue'
import { users, myId, myColor, wsSend } from '../../composables/useWs'
import {
  registerCanvas, sizeCanvas, drawStroke, clearCanvas,
  onCanvasPointerDown, onCanvasPointerMove, onCanvasPointerUp,
  brushSizes, disposeCanvas,
} from '../../composables/useCanvas'

const props = defineProps<{ winId: string; canvasId: string; winW: number; winH: number }>()

const canvasEl = ref<HTMLCanvasElement | null>(null)
const TOOLBAR_H = 44

onMounted(() => {
  if (!canvasEl.value) return
  const cw = canvasEl.value.parentElement!.clientWidth
  const ch = canvasEl.value.parentElement!.clientHeight - TOOLBAR_H
  canvasEl.value.width = cw
  canvasEl.value.height = ch
  registerCanvas(props.canvasId, canvasEl.value)
})

// Refit if window size changes
watch([() => props.winW, () => props.winH], () => {
  if (!canvasEl.value) return
  const cw = canvasEl.value.parentElement!.clientWidth
  const ch = canvasEl.value.parentElement!.clientHeight - TOOLBAR_H
  sizeCanvas(props.canvasId, cw, ch)
})

const doClear = () => {
  clearCanvas(props.canvasId)
  wsSend({ type: 'CanvasClear', canvas_id: props.canvasId })
}

const otherUsers = () => Object.values(users.value).filter(u => u.id !== myId.value)
</script>

<template>
  <div class="absolute inset-0 flex flex-col" style="background:#1e1e1e">
    <!-- Toolbar -->
    <div class="flex items-center gap-3 px-3 shrink-0 border-b" style="height:44px;background:rgba(30,30,30,0.95);border-color:rgba(255,255,255,0.06)">
      <!-- My color -->
      <div class="flex items-center gap-2">
        <div class="w-5 h-5 rounded-full border-2 border-white/20 shadow-md" :style="{ background: myColor }"></div>
        <span class="text-[11px]" style="color:rgba(255,255,255,0.35)">You</span>
      </div>

      <!-- Brush size -->
      <input type="range" min="1" max="30"
        :value="brushSizes[canvasId] ?? 4"
        @input="(e) => brushSizes[canvasId] = Number((e.target as HTMLInputElement).value)"
        class="w-24 h-1 accent-[#60a5fa]" />
      <span class="text-[11px] w-4 text-center" style="color:rgba(255,255,255,0.4)">{{ brushSizes[canvasId] ?? 4 }}</span>

      <!-- Clear -->
      <button @click="doClear"
        class="ml-1 px-3 py-1 rounded-lg text-[12px] font-medium transition-[background] duration-150 hover:brightness-125"
        style="background:rgba(255,255,255,0.07);color:rgba(255,255,255,0.6);border:1px solid rgba(255,255,255,0.1)">
        Clear
      </button>

      <!-- Other users -->
      <div class="ml-auto flex items-center gap-3">
        <div v-for="u in otherUsers()" :key="u.id" class="flex items-center gap-1.5">
          <div class="w-3.5 h-3.5 rounded-full" :style="{ background: u.color }"></div>
          <span class="text-[10px]" style="color:rgba(255,255,255,0.3)">{{ u.name }}</span>
        </div>
      </div>
    </div>

    <!-- Canvas -->
    <canvas ref="canvasEl"
      class="flex-1 block cursor-crosshair touch-none outline-none"
      style="width:100%;display:block"
      @pointerdown="(e) => onCanvasPointerDown(e, canvasId)"
      @pointermove="(e) => onCanvasPointerMove(e, canvasId)"
      @pointerup="() => onCanvasPointerUp(canvasId)"
      @pointerleave="() => onCanvasPointerUp(canvasId)"
    />
  </div>
</template>
