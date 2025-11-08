import { ref } from 'vue'
import type { FileEntry } from '../types'
import { wsSend } from './useWs'

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

export const globalFileState = ref<FileState>({
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
})

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
    renaming: s.renaming ? { path: s.renaming.path, name: s.renaming.name } : null,
    clipboard_op: s.clipboard?.op ?? null,
    clipboard_paths: s.clipboard?.paths ?? [],
    preview_path: s.preview?.path ?? null,
  }
  wsSend({ type: 'FileSync', state: sync })
}

export async function applyRemoteFileState(sync: FileStateSync) {
  isApplyingRemoteSync = true
  const s = globalFileState.value

  // Path changed — fetch new listing first, THEN apply rest of state
  if (s.path !== sync.path) {
    await _fetchAndSetEntries(sync.path)
  }

  s.selected = new Set(sync.selected)
  s.scrollTop = sync.scroll_top
  s.renaming = sync.renaming ? { ...sync.renaming } : null

  s.clipboard = sync.clipboard_op
    ? { op: sync.clipboard_op as 'copy' | 'cut', paths: sync.clipboard_paths }
    : null

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

// Internal: fetch entries and set path — does NOT touch selection/scroll/etc
async function _fetchAndSetEntries(path: string) {
  const s = globalFileState.value
  s.path = path
  s.loading = true
  try {
    const res = await fetch(`/api/files?path=${encodeURIComponent(path)}`)
    if (!res.ok) throw new Error(await res.text())
    const entries: FileEntry[] = await res.json()
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

// Public: full navigation — resets selection, broadcasts to all peers
export async function loadFiles(path: string, localOnly = false) {
  const s = globalFileState.value
  s.selected = new Set()
  s.preview = null
  s.contextMenu = null
  s.scrollTop = 0
  s.renaming = null

  await _fetchAndSetEntries(path)

  // Broadcast AFTER entries are loaded so peers get the full new path
  if (!localOnly) broadcastFileState()
}

export async function openEntry(entry: FileEntry, localOnly = false) {
  if (entry.is_dir) {
    await loadFiles(entry.path)   // always broadcasts
    return
  }

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
  try {
    const res = await fetch('/api/files/rename', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ path: oldPath, name: newName }),
    })
    if (!res.ok) throw new Error(await res.text())
    // Reload current dir and broadcast immediately — fixes "stuck at old name" for peers
    await loadFiles(globalFileState.value.path)
  } catch (e) { alert(`Rename failed: ${e}`) }
}

export async function deleteFiles(paths: string[]) {
  await Promise.all(paths.map(path =>
    fetch('/api/files/delete', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ path }),
    }).catch(() => {})
  ))
  await loadFiles(globalFileState.value.path)
}

export async function copyFiles(paths: string[], destDir: string) {
  await Promise.all(paths.map(path => {
    const name = path.split('/').pop()!
    return fetch('/api/files/copy', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ from: path, to: `${destDir}/${name}` }),
    }).catch(() => {})
  }))
  await loadFiles(globalFileState.value.path)
}

export async function moveFiles(paths: string[], destDir: string) {
  await Promise.all(paths.map(path => {
    const name = path.split('/').pop()!
    return fetch('/api/files/move', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ from: path, to: `${destDir}/${name}` }),
    }).catch(() => {})
  }))
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
