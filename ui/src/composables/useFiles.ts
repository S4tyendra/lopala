import { ref } from 'vue'
import type { FileEntry } from '../types'
import { wsSend, windows } from './useWs'

export interface FileState {
  path: string
  entries: FileEntry[]
  loading: boolean
  selected: Set<string>       // selected paths
  clipboard: { op: 'copy' | 'cut'; paths: string[] } | null
  preview: { entry: FileEntry; content: string } | null  // '__no_preview__' | '__embed__' | text
  viewMode: 'grid' | 'list'
  contextMenu: { x: number; y: number; entry: FileEntry | null } | null
}

const states = ref<Record<string, FileState>>({})
export const fileStates = states

// Global current path — ALL file windows share same navigation
export const globalFilePath = ref('/home')

function defaultState(): FileState {
  return {
    path: globalFilePath.value,
    entries: [],
    loading: false,
    selected: new Set(),
    clipboard: null,
    preview: null,
    viewMode: 'grid',
    contextMenu: null,
  }
}

export function initFileState(winId: string) {
  if (!states.value[winId]) states.value[winId] = defaultState()
  loadFiles(globalFilePath.value, true)
}

export function cleanupFileState(winId: string) {
  delete states.value[winId]
}

// Navigate ALL file windows to the same path
export async function loadFiles(path: string, localOnly = false) {
  globalFilePath.value = path

  // Navigate every open file window
  const winIds = Object.entries(windows.value)
    .filter(([, w]) => w.app === 'files')
    .map(([id]) => id)

  for (const id of winIds) {
    if (!states.value[id]) states.value[id] = defaultState()
    const s = states.value[id]
    s.path = path
    s.loading = true
    s.selected = new Set()
    s.preview = null
    s.contextMenu = null
  }

  try {
    const res = await fetch(`/api/files?path=${encodeURIComponent(path)}`)
    if (!res.ok) throw new Error(await res.text())
    const entries: FileEntry[] = await res.json()

    for (const id of winIds) {
      if (states.value[id]) {
        states.value[id].entries = entries
        states.value[id].loading = false
      }
    }

    if (!localOnly) wsSend({ type: 'FileBrowse', path })
  } catch (err) {
    console.error('loadFiles:', err)
    for (const id of winIds) {
      if (states.value[id]) states.value[id].loading = false
    }
  }
}

// Open entry: dir = navigate, file = preview
export async function openEntry(entry: FileEntry) {
  if (entry.is_dir) { loadFiles(entry.path); return }

  // Find all file windows and set preview
  const winIds = Object.entries(windows.value)
    .filter(([, w]) => w.app === 'files')
    .map(([id]) => id)

  const m = entry.mime
  let content = ''

  if (/^(image\/|video\/|audio\/|application\/pdf)/.test(m)) {
    content = '__embed__'
  } else if (/^(text\/|application\/(json|xml|javascript|typescript))/.test(m)) {
    try {
      const res = await fetch(`/api/files/read?path=${encodeURIComponent(entry.path)}`)
      content = res.ok ? await res.text() : `Error: ${await res.text()}`
    } catch (e) { content = String(e) }
  } else {
    content = '__no_preview__'
  }

  for (const id of winIds) {
    if (states.value[id]) states.value[id].preview = { entry, content }
  }
}

// ── File operations ───────────────────────────────────────────────────────────

export async function renameFile(oldPath: string, newName: string) {
  const dir = oldPath.split('/').slice(0, -1).join('/') || '/'
  const newPath = `${dir}/${newName}`
  try {
    const res = await fetch('/api/files/rename', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ old_path: oldPath, new_path: newPath }),
    })
    if (!res.ok) throw new Error(await res.text())
    await loadFiles(globalFilePath.value, false)
  } catch (e) { alert(`Rename failed: ${e}`) }
}

export async function deleteFiles(paths: string[]) {
  for (const path of paths) {
    try {
      await fetch('/api/files/delete', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ path }),
      })
    } catch {}
  }
  await loadFiles(globalFilePath.value, false)
}

export async function copyFiles(paths: string[], destDir: string) {
  for (const path of paths) {
    const name = path.split('/').pop()!
    try {
      await fetch('/api/files/copy', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ src: path, dst: `${destDir}/${name}` }),
      })
    } catch {}
  }
  await loadFiles(globalFilePath.value, false)
}

export async function moveFiles(paths: string[], destDir: string) {
  for (const path of paths) {
    const name = path.split('/').pop()!
    try {
      await fetch('/api/files/move', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ src: path, dst: `${destDir}/${name}` }),
      })
    } catch {}
  }
  await loadFiles(globalFilePath.value, false)
}

export function fileIcon(entry: FileEntry): string {
  if (entry.is_dir) return '📁'
  const m = entry.mime
  if (m.startsWith('image/')) return '🖼️'
  if (m.startsWith('video/')) return '🎬'
  if (m.startsWith('audio/')) return '🎵'
  if (m === 'application/pdf') return '📄'
  if (m.includes('zip') || m.includes('tar') || m.includes('gzip')) return '🗜️'
  if (m.startsWith('text/') || m.includes('json') || m.includes('xml')) return '📝'
  if (m.includes('executable') || m.includes('elf')) return '⚙️'
  return '📎'
}

export function fileSizeHuman(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1048576) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / 1048576).toFixed(1)} MB`
}

export function formatDate(ts: number): string {
  return new Date(ts * 1000).toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' })
}
