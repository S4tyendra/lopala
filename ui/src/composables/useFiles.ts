import { ref } from 'vue'
import type { FileEntry } from '../types'
import { wsSend, myId } from './useWs'

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
  version: number
}

export interface FileStateSync {
  window_id: string
  path: string
  selected: string[]
  scroll_top: number
  renaming: { path: string; name: string } | null
  clipboard_op: string | null
  clipboard_paths: string[]
  preview_path: string | null
  version: number
  sender: string
}

export const fileStates = ref<Record<string, FileState>>({})

export function getFileState(winId: string) {
  if (!fileStates.value[winId]) {
    fileStates.value[winId] = {
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
      version: 0,
    }
  }
  return fileStates.value[winId]
}

let isApplyingRemoteSync = false

export function bumpVersion(winId: string) {
  getFileState(winId).version++
}

export function initFileState(winId: string) {
  const s = getFileState(winId)
  loadFiles(winId, s.path, true)
}

export function broadcastFileState(winId: string) {
  if (isApplyingRemoteSync) return
  const s = getFileState(winId)
  const sync: FileStateSync = {
    window_id: winId,
    path: s.path,
    selected: Array.from(s.selected),
    scroll_top: s.scrollTop,
    renaming: s.renaming ? { path: s.renaming.path, name: s.renaming.name } : null,
    clipboard_op: s.clipboard?.op ?? null,
    clipboard_paths: s.clipboard?.paths ?? [],
    preview_path: s.preview?.path ?? null,
    version: s.version,
    sender: myId.value,
  }
  wsSend({ type: 'FileSync', state: sync })
}

export async function applyRemoteFileState(sync: FileStateSync) {
  // Ignore echo of our own events
  if (sync.sender === myId.value) return

  isApplyingRemoteSync = true
  const s = getFileState(sync.window_id)

  // Path changed OR version increased (mutation) — fetch new listing first
  if (s.path !== sync.path || sync.version > s.version) {
    await _fetchAndSetEntries(sync.window_id, sync.path)
  }

  s.version = sync.version

  s.selected = new Set(sync.selected)
  s.scrollTop = sync.scroll_top
  s.renaming = sync.renaming ? { ...sync.renaming } : null

  s.clipboard = sync.clipboard_op
    ? { op: sync.clipboard_op as 'copy' | 'cut', paths: sync.clipboard_paths }
    : null

  if (sync.preview_path) {
    if (s.preview?.path !== sync.preview_path) {
      const entry = s.entries.find(e => e.path === sync.preview_path)
      if (entry) await openEntry(sync.window_id, entry, true)
    }
  } else {
    s.preview = null
  }

  isApplyingRemoteSync = false
}

// Internal: fetch entries and set path — does NOT touch selection/scroll/etc
async function _fetchAndSetEntries(winId: string, path: string) {
  const s = getFileState(winId)
  s.path = path
  s.loading = true
  try {
    const res = await fetch(`/api/files?path=${encodeURIComponent(path)}&_=${Date.now()}`)
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
export async function loadFiles(winId: string, path: string, localOnly = false) {
  const s = getFileState(winId)
  s.selected = new Set()
  s.preview = null
  s.contextMenu = null
  s.scrollTop = 0
  s.renaming = null

  await _fetchAndSetEntries(winId, path)

  // Broadcast AFTER entries are loaded so peers get the full new path
  if (!localOnly) broadcastFileState(winId)
}

export async function openEntry(winId: string, entry: FileEntry, localOnly = false) {
  if (entry.is_dir) {
    await loadFiles(winId, entry.path)   // always broadcasts
    return
  }

  const s = getFileState(winId)
  const m = entry.mime
  let content = ''

  if (/^(image\/|video\/|audio\/|application\/pdf)/.test(m)) {
    content = '__embed__'
  } else if (/^(text\/|application\/(json|xml|javascript|typescript))/.test(m)) {
    try {
      const res = await fetch(`/api/files/read?path=${encodeURIComponent(entry.path)}&_=${Date.now()}`)
      content = res.ok ? await res.text() : `Error: ${await res.text()}`
    } catch (e) { content = String(e) }
  } else {
    content = '__no_preview__'
  }

  s.preview = { path: entry.path, content }
  if (!localOnly) broadcastFileState(winId)
}

export async function renameFile(winId: string, oldPath: string, newName: string) {
  try {
    const res = await fetch('/api/files/rename', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ path: oldPath, name: newName }),
    })
    if (!res.ok) throw new Error(await res.text())
    bumpVersion(winId)
    await loadFiles(winId, getFileState(winId).path)
  } catch (e) { alert(`Rename failed: ${e}`) }
}

export async function deleteFiles(winId: string, paths: string[]) {
  await Promise.all(paths.map(path =>
    fetch('/api/files/delete', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ path }),
    }).catch(() => {})
  ))
  bumpVersion(winId)
  await loadFiles(winId, getFileState(winId).path)
}

export async function copyFiles(winId: string, paths: string[], destDir: string) {
  await Promise.all(paths.map(path => {
    const name = path.split('/').pop()!
    return fetch('/api/files/copy', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ from: path, to: `${destDir}/${name}` }),
    }).catch(() => {})
  }))
  bumpVersion(winId)
  await loadFiles(winId, getFileState(winId).path)
}

export async function moveFiles(winId: string, paths: string[], destDir: string) {
  await Promise.all(paths.map(path => {
    const name = path.split('/').pop()!
    return fetch('/api/files/move', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ from: path, to: `${destDir}/${name}` }),
    }).catch(() => {})
  }))
  bumpVersion(winId)
  await loadFiles(winId, getFileState(winId).path)
}

export function fileIcon(entry: FileEntry): string {
  if (entry.is_dir) return `<svg viewBox="0 0 24 24" width="100%" height="100%" fill="none" stroke="currentColor" stroke-width="2"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2v11z"/></svg>`
  const m = entry.mime
  if (m.startsWith('image/')) return `<svg viewBox="0 0 24 24" width="100%" height="100%" fill="none" stroke="currentColor" stroke-width="2"><rect x="3" y="3" width="18" height="18" rx="2" ry="2"/><circle cx="8.5" cy="8.5" r="1.5"/><polyline points="21 15 16 10 5 21"/></svg>`
  if (m.startsWith('video/')) return `<svg viewBox="0 0 24 24" width="100%" height="100%" fill="none" stroke="currentColor" stroke-width="2"><rect x="2" y="2" width="20" height="20" rx="2.18" ry="2.18"/><line x1="7" y1="2" x2="7" y2="22"/><line x1="17" y1="2" x2="17" y2="22"/><line x1="2" y1="12" x2="22" y2="12"/><line x1="2" y1="7" x2="7" y2="7"/><line x1="2" y1="17" x2="7" y2="17"/><line x1="17" y1="17" x2="22" y2="17"/><line x1="17" y1="7" x2="22" y2="7"/></svg>`
  if (m.startsWith('audio/')) return `<svg viewBox="0 0 24 24" width="100%" height="100%" fill="none" stroke="currentColor" stroke-width="2"><path d="M9 18V5l12-2v13"/><circle cx="6" cy="18" r="3"/><circle cx="18" cy="16" r="3"/></svg>`
  if (m === 'application/pdf') return `<svg viewBox="0 0 24 24" width="100%" height="100%" fill="none" stroke="currentColor" stroke-width="2"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="16" y1="13" x2="8" y2="13"/><line x1="16" y1="17" x2="8" y2="17"/><polyline points="10 9 9 9 8 9"/></svg>`
  if (m.includes('zip') || m.includes('tar') || m.includes('gzip')) return `<svg viewBox="0 0 24 24" width="100%" height="100%" fill="none" stroke="currentColor" stroke-width="2"><rect x="4" y="4" width="16" height="16" rx="2"/><line x1="12" y1="4" x2="12" y2="20"/><line x1="12" y1="8" x2="15" y2="8"/><line x1="12" y1="12" x2="15" y2="12"/><line x1="12" y1="16" x2="15" y2="16"/></svg>`
  if (m.startsWith('text/') || m.includes('json') || m.includes('xml')) return `<svg viewBox="0 0 24 24" width="100%" height="100%" fill="none" stroke="currentColor" stroke-width="2"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="16" y1="13" x2="8" y2="13"/><line x1="16" y1="17" x2="8" y2="17"/><line x1="10" y1="9" x2="8" y2="9"/></svg>`
  if (m.includes('executable') || m.includes('elf')) return `<svg viewBox="0 0 24 24" width="100%" height="100%" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"/></svg>`
  return `<svg viewBox="0 0 24 24" width="100%" height="100%" fill="none" stroke="currentColor" stroke-width="2"><path d="M13 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z"/><polyline points="13 2 13 9 20 9"/></svg>`
}

export function fileSizeHuman(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1048576) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / 1048576).toFixed(1)} MB`
}

export function formatDate(ts: number): string {
  return new Date(ts * 1000).toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' })
}
