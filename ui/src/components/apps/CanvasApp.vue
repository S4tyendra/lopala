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
    <div class="flex items-center gap-4 px-4 shrink-0 border-b" style="height:44px;background:rgba(28,28,30,0.95);border-color:rgba(255,255,255,0.06);backdrop-filter:blur(24px)">
      <!-- My color -->
      <div class="flex items-center gap-2">
        <div class="w-5 h-5 rounded-full border-2 border-white/20 shadow-[0_2px_8px_rgba(0,0,0,0.4)]" :style="{ background: myColor }"></div>
        <span class="text-[11px] font-bold uppercase tracking-tight" style="color:rgba(255,255,255,0.4)">You</span>
      </div>

      <!-- Brush size -->
      <div class="flex items-center gap-3">
        <input type="range" min="1" max="30"
          :value="brushSizes[canvasId] ?? 4"
          @input="(e) => brushSizes[canvasId] = Number((e.target as HTMLInputElement).value)"
          class="w-24 h-1 cursor-pointer accent-[#60a5fa] hover:brightness-110 transition-all duration-200" />
        <span class="text-[10px] font-mono w-4 text-center" style="color:rgba(255,255,255,0.4)">{{ (brushSizes[canvasId] ?? 4).toString().padStart(2, '0') }}</span>
      </div>

      <!-- Clear -->
      <button @click="doClear"
        class="ml-2 px-3.5 py-1.5 rounded-lg text-[11px] font-bold uppercase tracking-wide transition-all duration-200 var(--ease-out) active:scale-95 hover:bg-white/10"
        style="background:rgba(255,255,255,0.05);color:rgba(255,255,255,0.8);border:1px solid rgba(255,255,255,0.1)">
        Clear
      </button>

      <!-- Other users -->
      <div class="ml-auto flex items-center gap-4">
        <div v-for="u in otherUsers()" :key="u.id" class="flex items-center gap-2 transition-all duration-300 animate-in fade-in slide-in-from-right-4">
          <div class="w-4 h-4 rounded-full border border-white/10 shadow-sm" :style="{ background: u.color }"></div>
          <span class="text-[10px] font-semibold" style="color:rgba(255,255,255,0.4)">{{ u.name }}</span>
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
