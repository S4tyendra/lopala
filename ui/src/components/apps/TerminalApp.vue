<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import { initTerminal, getTerminal } from '../../composables/useTerminals'

const props = defineProps<{ winId: string }>()

const showCopied = ref(false)
let copiedTimer: ReturnType<typeof setTimeout> | null = null

function flashCopied() {
  showCopied.value = true
  if (copiedTimer) clearTimeout(copiedTimer)
  copiedTimer = setTimeout(() => showCopied.value = false, 1500)
}

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

function copyFullBuffer() {
  const t = getTerminal(props.winId)
  if (!t) return
  t.term.selectAll()
  const text = t.term.getSelection()
  t.term.clearSelection()
  if (text) {
    navigator.clipboard.writeText(text)
    flashCopied()
  }
}

function copyLastOutput() {
  const t = getTerminal(props.winId)
  if (!t) return
  const buf = t.term.buffer.active
  const lines: string[] = []
  const end = buf.cursorY + buf.baseY
  const start = Math.max(0, end - 50)
  for (let i = start; i <= end; i++) {
    const line = buf.getLine(i)
    if (line) lines.push(line.translateToString(true))
  }
  while (lines.length && !lines[lines.length - 1].trim()) lines.pop()
  const text = lines.join('\n')
  if (text) {
    navigator.clipboard.writeText(text)
    flashCopied()
  }
}

function onKey(e: KeyboardEvent) {
  if (e.ctrlKey && e.shiftKey && e.key === 'C') {
    e.preventDefault()
    e.stopPropagation()
    copySelection()
  }
}

onMounted(() => {
  initTerminal(props.winId)
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
  <div class="term-root">
    <!-- Toolbar -->
    <div class="term-toolbar">
      <div class="term-toolbar-left">
        <span class="term-badge">Terminal</span>
      </div>
      <div class="term-toolbar-right">
        <button @click="copySelection" class="term-btn" title="Copy selection [Ctrl+Shift+C]">
          <svg viewBox="0 0 24 24" width="13" height="13" fill="none" stroke="currentColor" stroke-width="2.5"
            stroke-linecap="round" stroke-linejoin="round">
            <rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
            <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
          </svg>
          <span>Copy Selection</span>
        </button>

        <button @click="copyFullBuffer" class="term-btn" title="Copy entire buffer">
          <svg viewBox="0 0 24 24" width="13" height="13" fill="none" stroke="currentColor" stroke-width="2.5"
            stroke-linecap="round" stroke-linejoin="round">
            <path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2" />
            <rect x="8" y="2" width="8" height="4" rx="1" ry="1" />
          </svg>
          <span>Copy All</span>
        </button>
        <Transition name="copied-fade">
          <span v-if="showCopied" class="term-copied">✓</span>
        </Transition>
      </div>
    </div>

    <!-- xterm mount point -->
    <div :id="`term-${winId}`" class="term-canvas" />
  </div>
</template>

<style scoped>
.term-root {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  background: #0c0c0c;
}

/* ── Toolbar ────────────────────────────────────────────────────── */
.term-toolbar {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 10px;
  height: 30px;
  background: rgba(255, 255, 255, 0.03);
  border-bottom: 1px solid rgba(255, 255, 255, 0.07);
}

.term-toolbar-left {
  display: flex;
  align-items: center;
  gap: 6px;
}

.term-badge {
  font-size: 10px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.12em;
  color: rgba(255, 255, 255, 0.2);
}

.term-toolbar-right {
  display: flex;
  align-items: center;
  gap: 4px;
}

.term-btn {
  display: flex;
  align-items: center;
  gap: 5px;
  padding: 3px 8px;
  border: none;
  border-radius: 5px;
  background: rgba(255, 255, 255, 0.06);
  color: rgba(255, 255, 255, 0.55);
  cursor: pointer;
  font-size: 11px;
  font-weight: 500;
  transition: background 150ms, color 150ms, transform 100ms;
  white-space: nowrap;
}

.term-btn:hover {
  background: rgba(255, 255, 255, 0.12);
  color: rgba(255, 255, 255, 0.9);
}

.term-btn:active {
  transform: scale(0.94);
  background: rgba(255, 255, 255, 0.08);
}

.term-copied {
  font-size: 10px;
  font-weight: 700;
  color: #34d399;
  background: rgba(52, 211, 153, 0.12);
  border: 1px solid rgba(52, 211, 153, 0.25);
  padding: 2px 8px;
  border-radius: 999px;
  letter-spacing: 0.04em;
}

/* ── Terminal canvas ─────────────────────────────────────────────── */
.term-canvas {
  flex: 1;
  min-height: 0;
  padding: 2px;
}

/* ── Copied badge transition ─────────────────────────────────────── */
.copied-fade-enter-active {
  transition: opacity 200ms, transform 200ms;
}

.copied-fade-leave-active {
  transition: opacity 180ms;
}

.copied-fade-enter-from {
  opacity: 0;
  transform: translateY(4px) scale(0.92);
}

.copied-fade-leave-to {
  opacity: 0;
}
</style>
