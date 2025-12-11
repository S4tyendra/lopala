<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch, nextTick, shallowRef } from 'vue'
import { ws, wsSend, myId, myName, myColor, windows } from '../../composables/useWs'
import { EditorState, StateField, StateEffect, type ChangeSpec, type Extension } from '@codemirror/state'
import { EditorView, keymap, ViewUpdate, Decoration, type DecorationSet, WidgetType } from '@codemirror/view'
import { basicSetup } from 'codemirror'
import { oneDark } from '@codemirror/theme-one-dark'
import { javascript } from '@codemirror/lang-javascript'
import { html } from '@codemirror/lang-html'
import { css } from '@codemirror/lang-css'
import { json } from '@codemirror/lang-json'
import { python } from '@codemirror/lang-python'
import { rust } from '@codemirror/lang-rust'
import { markdown } from '@codemirror/lang-markdown'
import type { FileEntry } from '../../types'

const props = defineProps<{ winId: string }>()

// ─── State ────────────────────────────────────────────────────────────────────
interface EditorTab {
  path: string
  name: string
  content: string
  dirty: boolean
  version: number
}

const tabs = ref<EditorTab[]>([])
const activeTab = ref<string | null>(null)

const treePath = ref('/')
const treeEntries = ref<FileEntry[]>([])
const treeLoading = ref(false)
const saving = ref(false)
const showSidebar = ref(true)

// CodeMirror view (imperative, not reactive)
let view: EditorView | null = null
const editorEl = ref<HTMLElement | null>(null)
let suppressRemote = false // prevents echo-back loops

// Document versions for conflict resolution
const docVersions = new Map<string, number>()

// ─── Remote Cursors Extension ─────────────────────────────────────────────────
class RemoteCursorWidget extends WidgetType {
  constructor(public color: string, public name: string) { super() }
  toDOM() {
    const el = document.createElement('span')
    el.className = 'cm-remote-cursor'
    el.style.borderLeft = `2px solid ${this.color}`
    const lbl = document.createElement('span')
    lbl.className = 'cm-remote-cursor-label'
    lbl.style.background = this.color
    lbl.textContent = this.name
    el.appendChild(lbl)
    return el
  }
}

const remoteCursorsMap = new Map<string, { pos: number, name: string, color: string }>()
const setRemoteCursorsEffect = StateEffect.define<any>()

const remoteCursorField = StateField.define<DecorationSet>({
  create() { return Decoration.none },
  update(deco, tr) {
    deco = deco.map(tr.changes)
    for (const e of tr.effects) {
      if (e.is(setRemoteCursorsEffect)) {
        const arr = (e.value as any[])
          .filter(c => c.pos <= tr.state.doc.length)
          .sort((a,b) => a.pos - b.pos)
          .map(c => Decoration.widget({ widget: new RemoteCursorWidget(c.color, c.name), side: 1 }).range(c.pos))
        return Decoration.set(arr, true)
      }
    }
    return deco
  },
  provide: f => EditorView.decorations.from(f)
})

// ─── Language detection ───────────────────────────────────────────────────────
function langForFile(name: string): Extension[] {
  const ext = name.split('.').pop()?.toLowerCase() ?? ''
  switch (ext) {
    case 'js': case 'jsx': case 'mjs': return [javascript()]
    case 'ts': case 'tsx': return [javascript({ typescript: true, jsx: true })]
    case 'vue': case 'html': case 'htm': case 'astro': return [html()]
    case 'css': case 'scss': return [css()]
    case 'json': case 'jsonc': return [json()]
    case 'py': return [python()]
    case 'rs': return [rust()]
    case 'md': case 'mdx': return [markdown()]
    default: return []
  }
}

// ─── File tree ────────────────────────────────────────────────────────────────
async function loadTree(path?: string) {
  if (path) treePath.value = path
  treeLoading.value = true
  try {
    const res = await fetch(`/api/files?path=${encodeURIComponent(treePath.value)}`)
    if (res.ok) treeEntries.value = await res.json()
  } catch {}
  treeLoading.value = false
}

function treeGoUp() {
  const p = treePath.value
  const parent = p === '/' ? '/' : p.split('/').slice(0, -1).join('/') || '/'
  loadTree(parent)
}

function treeClick(entry: FileEntry) {
  if (entry.is_dir) {
    loadTree(entry.path)
  } else {
    // Add to args!
    const win = windows.value[props.winId]
    if (!win) return
    if (!win.args) win.args = { mode: 'dir', files: [] }
    if (!win.args.files) win.args.files = []
    
    if (!win.args.files.includes(entry.path)) {
      win.args.files.push(entry.path)
    }
    win.args.activeFile = entry.path
    wsSend({ type: 'UpdateWindow', window: win })
  }
}

function fileIcon(entry: FileEntry) {
  if (entry.is_dir) return '📁'
  const ext = entry.name.split('.').pop()?.toLowerCase()
  const map: Record<string, string> = {
    ts: '🔷', js: '🟨', vue: '💚', rs: '🦀', py: '🐍', html: '🌐',
    css: '🎨', json: '📋', md: '📝', toml: '⚙️', yaml: '⚙️', yml: '⚙️',
    lock: '🔒', gitignore: '👁️',
  }
  return map[ext ?? ''] ?? '📄'
}

// ─── Sync Args logic (Global Shared Tabs) ──────────────────────────────────────
let currentMode = ''
let currentDirPath = ''

watch(() => windows.value[props.winId]?.args, (newArgs) => {
  if (!newArgs) return
  
  if (newArgs.mode !== currentMode) {
    currentMode = newArgs.mode
    showSidebar.value = newArgs.mode === 'dir'
  }
  if (newArgs.mode === 'dir' && newArgs.dirPath && newArgs.dirPath !== currentDirPath) {
    currentDirPath = newArgs.dirPath
    loadTree(newArgs.dirPath)
  }

  const requiredFiles: string[] = newArgs.files || []
  const activeReq: string | undefined = newArgs.activeFile

  // 1. Remove closed tabs
  for (let i = tabs.value.length - 1; i >= 0; i--) {
    if (!requiredFiles.includes(tabs.value[i].path)) {
      if (tabs.value[i].path === activeTab.value) destroyEditor()
      tabs.value.splice(i, 1)
    }
  }

  // 2. Add new files
  for (const path of requiredFiles) {
    if (!tabs.value.find(t => t.path === path)) {
      openFileLocal(path, path.split('/').pop() || '')
    }
  }

  // 3. Set active tab
  if (activeReq && activeTab.value !== activeReq) {
    activeTab.value = activeReq
    const tabToMount = tabs.value.find(t => t.path === activeReq)
    if (tabToMount) mountEditor(tabToMount)
    // If not found yet, `openFileLocal` will mount it once fetched
  } else if (!activeReq) {
    activeTab.value = null
    destroyEditor()
  }
}, { deep: true, immediate: true })

async function openFileLocal(path: string, name: string) {
  try {
    const res = await fetch(`/api/files/read?path=${encodeURIComponent(path)}`)
    if (!res.ok) return
    const content = await res.text()
    const version = Date.now()
    const tab: EditorTab = { path, name, content, dirty: false, version }
    tabs.value.push(tab)
    docVersions.set(path, version)
    
    const win = windows.value[props.winId]
    if (win?.args?.activeFile === path) {
      activeTab.value = path
      await nextTick()
      mountEditor(tab)
    }
  } catch {}
}

function closeTab(path: string) {
  const win = windows.value[props.winId]
  if (!win || !win.args || !win.args.files) return
  
  const tab = tabs.value.find(t => t.path === path)
  if (tab?.dirty && !confirm(`Close ${tab.name} without saving?`)) return
  
  win.args.files = win.args.files.filter((p: string) => p !== path)
  if (win.args.activeFile === path) {
    win.args.activeFile = win.args.files.length ? win.args.files[win.args.files.length - 1] : undefined
  }
  wsSend({ type: 'UpdateWindow', window: win })
}

function setGlobalActiveTab(path: string) {
  const win = windows.value[props.winId]
  if (!win || !win.args) return
  win.args.activeFile = path
  wsSend({ type: 'UpdateWindow', window: win })
}

async function saveTab(path?: string) {
  const p = path ?? activeTab.value
  if (!p) return
  const tab = tabs.value.find(t => t.path === p)
  if (!tab) return
  saving.value = true
  try {
    const res = await fetch('/api/files/write', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ path: tab.path, content: tab.content }),
    })
    if (res.ok) tab.dirty = false
  } catch {}
  saving.value = false
}

// ─── CodeMirror ───────────────────────────────────────────────────────────────
function mountEditor(tab: EditorTab) {
  destroyEditor()
  if (!editorEl.value) return

  const extensions: Extension[] = [
    basicSetup,
    oneDark,
    ...langForFile(tab.name),
    EditorView.updateListener.of((update: ViewUpdate) => {
      if (update.docChanged && !suppressRemote) {
        const content = update.state.doc.toString()
        tab.content = content
        tab.dirty = true

        update.changes.iterChanges((fromA, toA, fromB, toB, inserted) => {
          const version = ++tab.version
          docVersions.set(tab.path, version)
          wsSend({
            type: 'EditorOp',
            op: {
              file_path: tab.path,
              user_id: myId.value,
              op: toA > fromA ? (inserted.length ? 'replace' : 'delete') : 'insert',
              pos: fromA,
              del_len: toA - fromA,
              text: inserted.toString(),
              version,
            }
          })
        })
      }
      
      if (update.selectionSet || update.docChanged) {
        if (!suppressRemote && Date.now() - lastCursorEmit > 50) {
          lastCursorEmit = Date.now()
          const head = update.state.selection.main.head
          const anchor = update.state.selection.main.anchor
          wsSend({
            type: 'EditorCursor',
            cursor: {
              file_path: tab.path,
              user_id: myId.value,
              user_name: myName.value,
              user_color: myColor.value,
              pos: head,
              anchor
            }
          })
        }
      }
    }),
    remoteCursorField,
    // Ctrl/Cmd+S to save
    keymap.of([{
      key: 'Mod-s',
      run: () => { saveTab(); return true },
    }]),
    EditorView.theme({
      '&': { height: '100%', fontSize: '13px' },
      '.cm-scroller': { overflow: 'auto', fontFamily: '"JetBrains Mono", monospace' },
      '.cm-content': { caretColor: '#00ffcc' },
    }),
  ]

  view = new EditorView({
    state: EditorState.create({ doc: tab.content, extensions }),
    parent: editorEl.value,
  })
}

function destroyEditor() {
  if (view) { view.destroy(); view = null }
}

// ─── Remote editor ops ───────────────────────────────────────────────────────
function onWsMsg(e: MessageEvent) {
  try {
    const msg = JSON.parse(e.data)
    if (msg.type === 'EditorOp' && msg.op.user_id !== myId.value) {
      const op = msg.op
      const tab = tabs.value.find(t => t.path === op.file_path)
      if (!tab || !view || activeTab.value !== op.file_path) return

      // Check version ordering — skip stale ops
      const localV = docVersions.get(op.file_path) ?? 0
      if (op.version <= localV) return
      docVersions.set(op.file_path, op.version)

      suppressRemote = true
      try {
        const changes: ChangeSpec[] = []
        if (op.del_len > 0) {
          changes.push({ from: op.pos, to: op.pos + op.del_len, insert: op.text || undefined })
        } else if (op.text) {
          changes.push({ from: op.pos, insert: op.text })
        }
        if (changes.length) {
          view.dispatch({ changes })
          tab.content = view.state.doc.toString()
          tab.version = op.version
        }
      } finally {
        suppressRemote = false
      }
    } else if (msg.type === 'EditorCursor' && msg.cursor.user_id !== myId.value) {
      const c = msg.cursor
      if (c.file_path === activeTab.value && view) {
        remoteCursorsMap.set(c.user_id, { pos: c.pos, name: c.user_name, color: c.user_color })
        view.dispatch({ effects: setRemoteCursorsEffect.of(Array.from(remoteCursorsMap.values())) })
      }
    }
  } catch {}
}

let lastCursorEmit = 0

onMounted(() => {
  ws.value?.addEventListener('message', onWsMsg)
})
onUnmounted(() => {
  destroyEditor()
  ws.value?.removeEventListener('message', onWsMsg)
})
watch(ws, (n, o) => {
  o?.removeEventListener('message', onWsMsg)
  n?.addEventListener('message', onWsMsg)
})
</script>

<template>
  <div class="editor-root">
    <!-- ── File Tree ──────────────────────────────────────────────── -->
    <div v-if="showSidebar" class="file-tree">
      <div class="tree-header">
        <button @click="treeGoUp" class="tree-up" title="Go up">←</button>
        <span class="tree-path" :title="treePath">{{ treePath.split('/').pop() || '/' }}</span>
      </div>
      <div class="tree-list">
        <div v-if="treeLoading" class="tree-loading">Loading…</div>
        <div v-for="entry in treeEntries" :key="entry.path"
          @click="treeClick(entry)"
          class="tree-item"
          :class="{ active: activeTab === entry.path }">
          <span class="tree-icon">{{ fileIcon(entry) }}</span>
          <span class="tree-name">{{ entry.name }}</span>
        </div>
      </div>
    </div>

    <!-- ── Editor Panel ───────────────────────────────────────────── -->
    <div class="editor-panel">
      <!-- Tabs -->
      <div v-if="tabs.length" class="tab-bar">
        <div v-for="tab in tabs" :key="tab.path"
          @click="setGlobalActiveTab(tab.path)"
          class="tab" :class="{ active: activeTab === tab.path }">
          <span class="tab-name">{{ tab.dirty ? '● ' : '' }}{{ tab.name }}</span>
          <button @click.stop="closeTab(tab.path)" class="tab-close">✕</button>
        </div>
        <div class="tab-actions">
          <button v-if="saving" class="save-indicator">Saving…</button>
          <button v-else @click="saveTab()" class="save-btn" title="Ctrl+S">💾</button>
        </div>
      </div>

      <!-- CodeMirror mount point -->
      <div ref="editorEl" class="cm-mount" />

      <!-- Empty state -->
      <div v-if="!tabs.length && showSidebar" class="empty-state">
        <span class="empty-icon">✏️</span>
        <span class="empty-label">Open a file from the tree</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.editor-root {
  display: flex; height: 100%; overflow: hidden;
  background: #0c0c0e; color: #c8c8d0;
  font-family: 'JetBrains Mono', monospace;
}

/* ── File Tree ────────────────────────────────────────────────────── */
.file-tree {
  width: 220px; flex-shrink: 0; display: flex; flex-direction: column;
  border-right: 1px solid rgba(255,255,255,0.06);
  background: rgba(0,0,0,0.3);
}
.tree-header {
  display: flex; align-items: center; gap: 6px;
  padding: 8px 10px; border-bottom: 1px solid rgba(255,255,255,0.05);
  font-size: 11px; font-weight: 600; color: rgba(255,255,255,0.5);
}
.tree-up {
  background: none; border: none; color: rgba(255,255,255,0.35);
  cursor: pointer; font-size: 13px; padding: 2px 6px; border-radius: 4px;
  transition: all 100ms;
}
.tree-up:hover { background: rgba(255,255,255,0.08); color: white; }
.tree-path { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; flex: 1; }
.tree-list { flex: 1; overflow-y: auto; padding: 4px 0; }
.tree-loading { padding: 12px; font-size: 10px; color: rgba(255,255,255,0.2); text-align: center; }
.tree-item {
  display: flex; align-items: center; gap: 6px;
  padding: 4px 12px; cursor: pointer; font-size: 11px;
  color: rgba(255,255,255,0.7); transition: background 80ms;
  white-space: nowrap; overflow: hidden;
}
.tree-item:hover { background: rgba(255,255,255,0.05); }
.tree-item.active { background: rgba(96,165,250,0.12); color: white; }
.tree-icon { font-size: 13px; flex-shrink: 0; }
.tree-name { overflow: hidden; text-overflow: ellipsis; }

/* ── Editor Panel ─────────────────────────────────────────────────── */
.editor-panel { flex: 1; display: flex; flex-direction: column; overflow: hidden; min-width: 0; }

.tab-bar {
  display: flex; align-items: center; flex-shrink: 0;
  border-bottom: 1px solid rgba(255,255,255,0.06);
  background: rgba(0,0,0,0.25); overflow-x: auto;
  scrollbar-width: none;
}
.tab-bar::-webkit-scrollbar { display: none; }
.tab {
  display: flex; align-items: center; gap: 6px;
  padding: 6px 12px; font-size: 11px; cursor: pointer;
  color: rgba(255,255,255,0.45); white-space: nowrap;
  border-right: 1px solid rgba(255,255,255,0.04);
  transition: all 100ms; flex-shrink: 0;
}
.tab:hover { background: rgba(255,255,255,0.04); color: rgba(255,255,255,0.7); }
.tab.active { background: rgba(96,165,250,0.1); color: white; border-bottom: 2px solid #60a5fa; }
.tab-name { max-width: 120px; overflow: hidden; text-overflow: ellipsis; }
.tab-close {
  background: none; border: none; color: rgba(255,255,255,0.2);
  font-size: 10px; cursor: pointer; padding: 0 2px; border-radius: 3px;
  transition: all 100ms;
}
.tab-close:hover { background: rgba(248,113,113,0.2); color: #f87171; }
.tab-actions { margin-left: auto; padding: 0 8px; flex-shrink: 0; }
.save-btn {
  background: none; border: none; cursor: pointer; font-size: 14px;
  opacity: 0.5; transition: opacity 100ms;
}
.save-btn:hover { opacity: 1; }
.save-indicator { font-size: 10px; color: #34d399; background: none; border: none; animation: pulse 1s infinite; }
@keyframes pulse { 0%,100% { opacity: 1 } 50% { opacity: 0.4 } }

.cm-mount { flex: 1; overflow: hidden; }
.cm-mount :deep(.cm-editor) { height: 100%; }

.empty-state {
  flex: 1; display: flex; flex-direction: column; align-items: center;
  justify-content: center; gap: 8px; color: rgba(255,255,255,0.15);
}
.empty-icon { font-size: 36px; }
.empty-label { font-size: 12px; }

/* ── Remote Cursor Styles ────────────────────────────────────────── */
:deep(.cm-remote-cursor) {
  position: absolute; display: inline-block; z-index: 10;
  pointer-events: none; margin-left: -1px;
}
:deep(.cm-remote-cursor-label) {
  position: absolute; top: -16px; left: -1px;
  color: #000; font-size: 9px; font-weight: 700;
  padding: 1px 4px; border-radius: 4px 4px 4px 0;
  white-space: nowrap; opacity: 0; transition: opacity 200ms;
}
:deep(.cm-remote-cursor:hover .cm-remote-cursor-label) {
  opacity: 1;
}
</style>
