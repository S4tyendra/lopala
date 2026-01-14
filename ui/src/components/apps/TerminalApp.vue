<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import { initTerminal, getTerminal, writeToTerminal } from '../../composables/useTerminals'

const props = defineProps<{ winId: string }>()

const showCopied = ref(false)
let copiedTimer: ReturnType<typeof setTimeout> | null = null

function flashCopied() {
  showCopied.value = true
  if (copiedTimer) clearTimeout(copiedTimer)
  copiedTimer = setTimeout(() => showCopied.value = false, 1500)
}

// ── Copy selection (Ctrl+Shift+C also bound here) ─────────────────────────────
function copySelection() {
  const t = getTerminal(props.winId)
  if (!t) return
  const sel = t.term.getSelection()
  if (sel) {
    navigator.clipboard.writeText(sel)
    t.term.clearSelection()
    flashCopied()
  }
}

// ── Copy full terminal buffer ─────────────────────────────────────────────────
function copyFullBuffer() {
  const t = getTerminal(props.winId)
  if (!t) return
  // Xterm.js doesn't expose the full scrollback easily via API,
  // but we can select all then grab it
  t.term.selectAll()
  const text = t.term.getSelection()
  t.term.clearSelection()
  if (text) {
    navigator.clipboard.writeText(text)
    flashCopied()
  }
}

// ── Copy last command output ──────────────────────────────────────────────────
function copyLastCommand() {
  const t = getTerminal(props.winId)
  if (!t) return
  // Best-effort: grab the visible active buffer content
  // xterm.js doesn't separate commands, so copy last ~50 lines
  const buf = t.term.buffer.active
  const lines: string[] = []
  const end = buf.cursorY + buf.baseY
  const start = Math.max(0, end - 50)
  for (let i = start; i <= end; i++) {
    const line = buf.getLine(i)
    if (line) lines.push(line.translateToString(true))
  }
  // Trim trailing empty lines
  while (lines.length && !lines[lines.length - 1].trim()) lines.pop()
  const text = lines.join('\n')
  if (text) {
    navigator.clipboard.writeText(text)
    flashCopied()
  }
}

// ── Keyboard intercept ────────────────────────────────────────────────────────
function onKey(e: KeyboardEvent) {
  // Ctrl+Shift+C -> copy selection (override browser default)
  if (e.ctrlKey && e.shiftKey && e.key === 'C') {
    e.preventDefault()
    e.stopPropagation()
    copySelection()
  }
}

onMounted(() => {
  initTerminal(props.winId)
  // Attach after a tick to ensure xterm is mounted
  setTimeout(() => {
    const el = document.getElementById(`term-${props.winId}`)
    el?.addEventListener('keydown', onKey, true)
  }, 100)
})

onUnmounted(() => {
  const el = document.getElementById(`term-${props.winId}`)
  el?.removeEventListener('keydown', onKey, true)
})
</script>

<template>
  <div class="term-wrap">
    <div :id="`term-${winId}`" class="term-canvas" />

    <teleport :to="`#actions-${winId}`">
      <div class="flex items-center gap-1.5 px-2">
        <button @click="copySelection"
          class="pro-btn flex items-center justify-center w-7 h-7 rounded-md transition-all active:scale-90 opacity-40 hover:opacity-100 hover:bg-white/10"
          title="Copy selection [Ctrl+Shift+C]">
          <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2.5">
            <path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2" />
            <rect x="8" y="2" width="8" height="4" rx="1" ry="1" />
          </svg>
        </button>
        <button @click="copyLastCommand"
          class="pro-btn flex items-center justify-center w-7 h-7 rounded-md transition-all active:scale-90 opacity-40 hover:opacity-100 hover:bg-white/10"
          title="Copy last output">
          <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2.5"
            stroke-linecap="round" stroke-linejoin="round">
            <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
            <polyline points="14 2 14 8 20 8" />
            <line x1="16" y1="13" x2="8" y2="13" />
            <line x1="16" y1="17" x2="8" y2="17" />
            <polyline points="10 9 9 9 8 9" />
          </svg>
        </button>
        <button @click="copyFullBuffer"
          class="pro-btn flex items-center justify-center w-7 h-7 rounded-md transition-all active:scale-90 opacity-40 hover:opacity-100 hover:bg-white/10"
          title="Copy all text">
          <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2.5"
            stroke-linecap="round" stroke-linejoin="round">
            <rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
            <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
          </svg>
        </button>
        <Transition name="fade">
          <span v-if="showCopied"
            class="text-[9px] font-bold uppercase tracking-widest text-[#34d399] bg-[#34d399]/10 px-2 py-0.5 rounded-full border border-[#34d399]/20">Copied</span>
        </Transition>
      </div>
    </teleport>
  </div>
</template>

<style scoped>
.term-wrap {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  background: #0c0c0c;
}

.term-canvas {
  flex: 1;
  padding: 2px;
  min-height: 0;
}

.pro-btn {
  cursor: pointer;
  color: white;
}


.fade-enter-active {
  transition: opacity 250ms var(--ease-out), transform 300ms var(--ease-out);
}

.fade-leave-active {
  transition: opacity 200ms var(--ease-out);
}

.fade-enter-from {
  opacity: 0;
  transform: translateY(4px) scale(0.95);
}

.fade-leave-to {
  opacity: 0;
}
</style>
