import { ref } from 'vue'
import type { FileEntry } from '../types'
import { wsSend } from './useWs'

export interface FileState {
  path: string
  entries: FileEntry[]
  loading: boolean
  selected: FileEntry | null
  preview: string | null      // '__no_preview__' | '__embed__:path' | text content
  viewMode: 'grid' | 'list'
}

// Per-window file state
const states = ref<Record<string, FileState>>({})
export const fileStates = states

function defaultState(path = '/home'): FileState {
  return { path, entries: [], loading: false, selected: null, preview: null, viewMode: 'grid' }
}

export function initFileState(winId: string, path = '/home') {
  if (!states.value[winId]) states.value[winId] = defaultState(path)
  loadFiles(winId, path)
}

export function cleanupFileState(winId: string) {
  delete states.value[winId]
}

export async function loadFiles(winId: string, path: string, broadcast = true) {
  if (!states.value[winId]) states.value[winId] = defaultState(path)
  const s = states.value[winId]
  s.loading = true
  s.selected = null
  s.preview = null
  try {
    const res = await fetch(`/api/files?path=${encodeURIComponent(path)}`)
    if (!res.ok) throw new Error(await res.text())
    s.entries = await res.json()
    s.path = path
    // Sync navigation to others
    if (broadcast) wsSend({ type: 'FileBrowse', winId, path })
  } catch (err) {
    console.error('loadFiles:', err)
  } finally {
    s.loading = false
  }
}

export async function openEntry(winId: string, entry: FileEntry) {
  const s = states.value[winId]
  if (!s) return
  if (entry.is_dir) { loadFiles(winId, entry.path); return }
  s.selected = entry
  s.preview = null

  const m = entry.mime
  if (/^(image\/|video\/|audio\/|application\/pdf)/.test(m)) {
    s.preview = `__embed__:${entry.path}`
    return
  }
  if (/^(text\/|application\/(json|xml|javascript))/.test(m)) {
    try {
      const res = await fetch(`/api/files/read?path=${encodeURIComponent(entry.path)}`)
      s.preview = res.ok ? await res.text() : `Error: ${await res.text()}`
    } catch (e) { s.preview = String(e) }
    return
  }
  s.preview = '__no_preview__'
}

export function fileIcon(entry: FileEntry): string {
  if (entry.is_dir) return '📁'
  const m = entry.mime
  if (m.startsWith('image/')) return '🖼️'
  if (m.startsWith('video/')) return '🎬'
  if (m.startsWith('audio/')) return '🎵'
  if (m === 'application/pdf') return '📄'
  if (m.includes('zip') || m.includes('tar') || m.includes('gzip')) return '🗜️'
  if (m.startsWith('text/')) return '📝'
  return '📎'
}

export function fileSizeHuman(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1048576) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / 1048576).toFixed(1)} MB`
}
