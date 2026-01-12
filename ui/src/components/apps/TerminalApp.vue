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

    <!-- Copy toolbar -->
    <div class="copy-bar">
      <button @click="copySelection" class="copy-btn" title="Copy Selection (Ctrl+Shift+C)">📋 Sel</button>
      <button @click="copyLastCommand" class="copy-btn" title="Copy Last Output">📝 Last</button>
      <button @click="copyFullBuffer" class="copy-btn" title="Copy Full Buffer">📑 All</button>
      <Transition name="fade">
        <span v-if="showCopied" class="copied-badge">Copied!</span>
      </Transition>
    </div>
  </div>
</template>

<style scoped>
.term-wrap {
  position: absolute; inset: 0; display: flex; flex-direction: column;
  background: #0c0c0c;
}
.term-canvas { flex: 1; padding: 2px; min-height: 0; }

.copy-bar {
  display: flex; align-items: center; gap: 4px;
  padding: 3px 6px; flex-shrink: 0;
  background: rgba(0,0,0,0.4);
  border-top: 1px solid rgba(255,255,255,0.06);
}
.copy-btn {
  padding: 3px 10px; border-radius: 6px; font-size: 10px; font-weight: 500;
  background: rgba(255,255,255,0.04); border: 1px solid rgba(255,255,255,0.06);
  color: rgba(255,255,255,0.5); cursor: pointer; transition: all 150ms var(--ease-out);
  font-family: inherit; white-space: nowrap;
}
.copy-btn:hover { background: rgba(255,255,255,0.08); color: white; border-color: rgba(255,255,255,0.12); }
.copy-btn:active { transform: scale(0.96); }

.copied-badge {
  margin-left: auto; font-size: 10px; font-weight: 600; color: #34d399;
  padding: 2px 10px; border-radius: 6px;
  background: rgba(52,211,153,0.1); border: 1px solid rgba(52,211,153,0.2);
}

.fade-enter-active { transition: opacity 250ms var(--ease-out), transform 300ms var(--ease-out); }
.fade-leave-active { transition: opacity 200ms var(--ease-out); }
.fade-enter-from { opacity: 0; transform: translateY(4px) scale(0.95); }
.fade-leave-to { opacity: 0; }
</style>
