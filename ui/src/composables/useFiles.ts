import { ref, computed } from 'vue'
import type { FileEntry } from '../types'
import { wsSend, windows } from './useWs'

export interface FileState {
  path: string
  entries: FileEntry[]
  loading: boolean
  selected: Set<string>
  clipboard: { op: 'copy' | 'cut'; paths: string[] } | null
  preview: { path: string; content: string } | null
  viewMode: 'grid' | 'list'
  contextMenu: { x: number; y: number; entry: FileEntry | null } | null
  scrollTop: number
  renaming: { path: string; name: string } | null
}

export interface FileStateSync {
  path: string
  selected: string[]
  scroll_top: number
  renaming: { path: string; name: string } | null
  clipboard_op: string | null
  clipboard_paths: string[]
  preview_path: string | null
}

const defaultState: FileState = {
  path: '/home',
  entries: [],
  loading: false,
  selected: new Set(),
  clipboard: null,
  preview: null,
  viewMode: 'grid',
  contextMenu: null,
  scrollTop: 0,
  renaming: null,
}

export const globalFileState = ref<FileState>({ ...defaultState })
let isApplyingRemoteSync = false

export function initFileState() {
  loadFiles(globalFileState.value.path, true)
}

export function broadcastFileState() {
  if (isApplyingRemoteSync) return
  const s = globalFileState.value
  const sync: FileStateSync = {
    path: s.path,
    selected: Array.from(s.selected),
    scroll_top: s.scrollTop,
    renaming: s.renaming,
    clipboard_op: s.clipboard?.op ?? null,
    clipboard_paths: s.clipboard?.paths ?? [],
    preview_path: s.preview?.path ?? null,
  }
  wsSend({ type: 'FileSync', state: sync })
}

export async function applyRemoteFileState(sync: FileStateSync) {
  isApplyingRemoteSync = true
  const s = globalFileState.value

  if (s.path !== sync.path) {
    await loadFiles(sync.path, true)
  }

  s.selected = new Set(sync.selected)
  s.scrollTop = sync.scroll_top
  s.renaming = sync.renaming

  if (sync.clipboard_op) {
    s.clipboard = { op: sync.clipboard_op as 'copy'|'cut', paths: sync.clipboard_paths }
  } else {
    s.clipboard = null
  }

  if (sync.preview_path) {
    if (s.preview?.path !== sync.preview_path) {
      const entry = s.entries.find(e => e.path === sync.preview_path)
      if (entry) await openEntry(entry, true)
    }
  } else {
    s.preview = null
  }

  isApplyingRemoteSync = false
}

export async function loadFiles(path: string, localOnly = false) {
  const s = globalFileState.value
  s.path = path
  s.loading = true
  s.selected = new Set()
  s.preview = null
  s.contextMenu = null
  s.scrollTop = 0
  s.renaming = null
  
  if (!localOnly) broadcastFileState()

  try {
    const res = await fetch(`/api/files?path=${encodeURIComponent(path)}`)
    if (!res.ok) throw new Error(await res.text())
    const entries: FileEntry[] = await res.json()
    // Sort directories first
    s.entries = entries.sort((a, b) => {
      if (a.is_dir !== b.is_dir) return a.is_dir ? -1 : 1
      return a.name.localeCompare(b.name)
    })
  } catch (err) {
    console.error('loadFiles:', err)
  } finally {
    s.loading = false
  }
}

export async function openEntry(entry: FileEntry, localOnly = false) {
  if (entry.is_dir) { loadFiles(entry.path); return }

  const s = globalFileState.value
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

  s.preview = { path: entry.path, content }
  if (!localOnly) broadcastFileState()
}

export async function renameFile(oldPath: string, newName: string) {
  const dir = oldPath.split('/').slice(0, -1).join('/') || '/'
  const newPath = `${dir}/${newName}`
  try {
    const res = await fetch('/api/files/rename', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ path: oldPath, name: newName }),
    })
    if (!res.ok) throw new Error(await res.text())
    await loadFiles(globalFileState.value.path)  // broadcasts
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
  await loadFiles(globalFileState.value.path)
}

export async function copyFiles(paths: string[], destDir: string) {
  for (const path of paths) {
    const name = path.split('/').pop()!
    try {
      await fetch('/api/files/copy', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ from: path, to: `${destDir}/${name}` }),
    })
    } catch {}
  }
  await loadFiles(globalFileState.value.path)
}

export async function moveFiles(paths: string[], destDir: string) {
  for (const path of paths) {
    const name = path.split('/').pop()!
    try {
      await fetch('/api/files/move', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ from: path, to: `${destDir}/${name}` }),
      })
    } catch {}
  }
  await loadFiles(globalFileState.value.path)
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
