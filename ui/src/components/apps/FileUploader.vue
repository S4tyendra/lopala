<script setup lang="ts">
/**
 * FileUploader.vue
 *
 * High-performance, resumable chunked file uploader.
 *
 * Strategy:
 *  1. POST /api/files/upload/init  → receives session_id + total_parts
 *  2. Split file into 10 MiB chunks
 *  3. Hash each chunk with WebCrypto SHA-256 (hardware-accelerated, non-blocking)
 *  4. Upload 3 chunks concurrently; retry failed chunks up to 3×
 *  5. POST /api/files/upload/chunk?session_id=…&part=…&hash=…&process=true on finish
 */
import { ref, computed } from 'vue'

const props = defineProps<{
  destDir: string // current directory path in file manager
}>()

const emit = defineEmits<{
  (e: 'done'): void
  (e: 'close'): void
}>()

// ── Constants ─────────────────────────────────────────────────────────────────
const CHUNK_SIZE = 10 * 1024 * 1024 // 10 MiB (must match server)
const CONCURRENCY = 3
const MAX_RETRIES = 3

// ── Per-file upload state ─────────────────────────────────────────────────────
interface ChunkState {
  index: number
  status: 'pending' | 'hashing' | 'uploading' | 'done' | 'error'
  retries: number
  progress: number // bytes uploaded for this chunk
}

interface FileUpload {
  id: string
  file: File
  destPath: string
  sessionId: string | null
  totalParts: number
  chunks: ChunkState[]
  status: 'queued' | 'init' | 'uploading' | 'done' | 'error'
  errorMsg: string | null
}

const uploads = ref<FileUpload[]>([])
const isDragging = ref(false)

// ── Computed progress ─────────────────────────────────────────────────────────
const overallProgress = computed(() => {
  if (!uploads.value.length) return 0
  const total = uploads.value.reduce((acc, u) => acc + u.file.size, 0)
  if (total === 0) return 0

  const done = uploads.value.reduce((acc, u) => {
    if (u.status === 'done') return acc + u.file.size
    if (u.status === 'error') return acc
    // sum per-chunk progress
    return acc + u.chunks.reduce((a, c) => {
      if (c.status === 'done') return a + chunkByteSize(u.file.size, u.totalParts, c.index)
      return a + c.progress
    }, 0)
  }, 0)

  return Math.round((done / total) * 100)
})

const allDone = computed(() => uploads.value.length > 0 && uploads.value.every(u => u.status === 'done' || u.status === 'error'))

function chunkByteSize(fileSize: number, totalParts: number, index: number) {
  const last = totalParts - 1
  return index === last ? fileSize - (last * CHUNK_SIZE) : CHUNK_SIZE
}

// ── SHA-256 via WebCrypto (hardware-accelerated, does NOT block UI thread) ───
async function sha256Hex(buffer: ArrayBuffer): Promise<string> {
  const hashBuf = await crypto.subtle.digest('SHA-256', buffer)
  return Array.from(new Uint8Array(hashBuf))
    .map(b => b.toString(16).padStart(2, '0'))
    .join('')
}

// ── File ingestion ────────────────────────────────────────────────────────────
function onFileInput(e: Event) {
  const files = (e.target as HTMLInputElement).files
  if (files) addFiles(Array.from(files))
}

function onDrop(e: DragEvent) {
  isDragging.value = false
  const files = e.dataTransfer?.files
  if (files) addFiles(Array.from(files))
}

function addFiles(files: File[]) {
  for (const f of files) {
    const totalParts = Math.ceil(f.size / CHUNK_SIZE)
    const id = crypto.randomUUID()
    uploads.value.push({
      id,
      file: f,
      destPath: props.destDir.replace(/\/$/, '') + '/' + f.name,
      sessionId: null,
      totalParts,
      chunks: Array.from({ length: totalParts }, (_, i) => ({
        index: i,
        status: 'pending',
        retries: 0,
        progress: 0,
      })),
      status: 'queued',
      errorMsg: null,
    })
  }
  startAll()
}

// ── Orchestration ─────────────────────────────────────────────────────────────
async function startAll() {
  const queued = uploads.value.filter(u => u.status === 'queued')
  for (const u of queued) {
    uploadFile(u) // fire-and-forget per file
  }
}

async function uploadFile(u: FileUpload) {
  u.status = 'init'

  // 1. Init session
  try {
    const res = await fetch('/api/files/upload/init', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        dest_path: u.destPath,
        file_name: u.file.name,
        total_size: u.file.size,
      }),
    })
    if (!res.ok) throw new Error(await res.text())
    const data = await res.json()
    u.sessionId = data.session_id
    u.totalParts = data.total_parts
    // Re-initialise chunks in case totalParts changed
    u.chunks = Array.from({ length: u.totalParts }, (_, i) => ({
      index: i,
      status: 'pending',
      retries: 0,
      progress: 0,
    }))
  } catch (e: any) {
    u.status = 'error'
    u.errorMsg = e.message
    return
  }

  u.status = 'uploading'

  // 2. Upload chunks with CONCURRENCY=3, retries
  const queue = [...u.chunks]

  async function worker() {
    while (queue.length) {
      const chunk = queue.shift()!
      await uploadChunk(u, chunk)
    }
  }

  const workers = Array.from({ length: CONCURRENCY }, () => worker())
  await Promise.all(workers)

  const anyFailed = u.chunks.some(c => c.status === 'error')
  if (anyFailed) {
    u.status = 'error'
    u.errorMsg = 'Some chunks permanently failed after retries'
    return
  }

  // 3. Complete
  try {
    const res = await fetch(
      `/api/files/upload/chunk?session_id=${u.sessionId}&part=0&hash=complete&process=true`,
      { method: 'POST', body: new Uint8Array(0) }
    )
    if (!res.ok) {
      const err = await res.json()
      throw new Error(err.error || 'completion failed')
    }
    u.status = 'done'
    emit('done')
  } catch (e: any) {
    u.status = 'error'
    u.errorMsg = e.message
  }
}

async function uploadChunk(u: FileUpload, chunk: ChunkState) {
  chunk.status = 'hashing'
  const start = chunk.index * CHUNK_SIZE
  const slice = u.file.slice(start, start + CHUNK_SIZE)
  const buf = await slice.arrayBuffer()
  const hash = await sha256Hex(buf)

  chunk.status = 'uploading'

  for (let attempt = 0; attempt <= MAX_RETRIES; attempt++) {
    if (attempt > 0) {
      chunk.retries = attempt
      // Exponential back-off: 500ms, 1s, 2s
      await new Promise(r => setTimeout(r, 500 * Math.pow(2, attempt - 1)))
    }
    try {
      await uploadWithProgress(u.sessionId!, chunk, buf, hash)
      chunk.status = 'done'
      chunk.progress = buf.byteLength
      return
    } catch (err) {
      if (attempt === MAX_RETRIES) {
        chunk.status = 'error'
        return
      }
    }
  }
}

function uploadWithProgress(
  sessionId: string,
  chunk: ChunkState,
  buf: ArrayBuffer,
  hash: string,
): Promise<void> {
  return new Promise((resolve, reject) => {
    const xhr = new XMLHttpRequest()
    xhr.open(
      'POST',
      `/api/files/upload/chunk?session_id=${sessionId}&part=${chunk.index}&hash=${hash}`
    )
    xhr.upload.onprogress = (e) => {
      if (e.lengthComputable) chunk.progress = e.loaded
    }
    xhr.onload = () => {
      if (xhr.status >= 200 && xhr.status < 300) resolve()
      else reject(new Error(`HTTP ${xhr.status}: ${xhr.responseText}`))
    }
    xhr.onerror = () => reject(new Error('Network error'))
    xhr.send(buf)
  })
}

// ── Helpers ───────────────────────────────────────────────────────────────────
function fileProgress(u: FileUpload): number {
  if (u.status === 'done') return 100
  if (u.status === 'queued' || u.status === 'init') return 0
  const totalBytes = u.file.size
  if (totalBytes === 0) return 100
  const done = u.chunks.reduce((acc, c) => {
    if (c.status === 'done') return acc + chunkByteSize(u.file.size, u.totalParts, c.index)
    return acc + c.progress
  }, 0)
  return Math.min(99, Math.round((done / totalBytes) * 100))
}

function fmtSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 ** 2) return `${(bytes / 1024).toFixed(1)} KB`
  if (bytes < 1024 ** 3) return `${(bytes / 1024 ** 2).toFixed(1)} MB`
  return `${(bytes / 1024 ** 3).toFixed(2)} GB`
}

function statusIcon(u: FileUpload) {
  if (u.status === 'done') return '✓'
  if (u.status === 'error') return '✕'
  if (u.status === 'queued') return '·'
  return null
}

function statusClass(u: FileUpload) {
  if (u.status === 'done') return 'done'
  if (u.status === 'error') return 'err'
  return 'active'
}

function retryFile(u: FileUpload) {
  u.status = 'queued'
  u.errorMsg = null
  u.chunks.forEach(c => {
    if (c.status === 'error') { c.status = 'pending'; c.retries = 0; c.progress = 0 }
  })
  uploadFile(u)
}
</script>

<template>
  <div class="uploader" @click.stop>
    <!-- Header -->
    <div class="up-header">
      <span class="up-title">Upload Files</span>
      <span class="up-sub">→ {{ props.destDir }}</span>
      <button class="up-close" @click="emit('close')">✕</button>
    </div>

    <!-- Drop zone -->
    <label
      class="dropzone"
      :class="{ dragging: isDragging }"
      @dragover.prevent="isDragging = true"
      @dragleave.prevent="isDragging = false"
      @drop.prevent="onDrop"
    >
      <input type="file" multiple @change="onFileInput" class="hidden-inp" />
      <div class="dz-icon">{{ isDragging ? '📂' : '⬆️' }}</div>
      <div class="dz-label">Drop files here or <span class="dz-link">browse</span></div>
      <div class="dz-sub">Any file size · Chunked · Resumable</div>
    </label>

    <!-- Overall progress bar (only while uploading) -->
    <div v-if="uploads.length && !allDone" class="overall-bar-wrap">
      <div class="overall-bar-track">
        <div class="overall-bar-fill" :style="{ width: overallProgress + '%' }" />
      </div>
      <span class="overall-pct">{{ overallProgress }}%</span>
    </div>

    <!-- File list -->
    <div v-if="uploads.length" class="file-list">
      <div
        v-for="u in uploads"
        :key="u.id"
        class="file-row"
        :class="statusClass(u)"
      >
        <!-- Left: icon + name -->
        <div class="fr-left">
          <span class="fr-icon">
            <span v-if="statusIcon(u)" class="status-icon">{{ statusIcon(u) }}</span>
            <span v-else class="spinner" />
          </span>
          <div class="fr-info">
            <span class="fr-name">{{ u.file.name }}</span>
            <span class="fr-meta">{{ fmtSize(u.file.size) }} · {{ u.totalParts }} part{{ u.totalParts !== 1 ? 's' : '' }}</span>
            <span v-if="u.errorMsg" class="fr-err">{{ u.errorMsg }}</span>
          </div>
        </div>

        <!-- Right: progress -->
        <div class="fr-right">
          <template v-if="u.status === 'done'">
            <span class="pct-done">Done</span>
          </template>
          <template v-else-if="u.status === 'error'">
            <button class="retry-btn" @click.stop="retryFile(u)">Retry</button>
          </template>
          <template v-else>
            <div class="progress-wrap">
              <div class="progress-track">
                <div class="progress-fill" :style="{ width: fileProgress(u) + '%' }" />
              </div>
              <span class="pct-label">{{ fileProgress(u) }}%</span>
            </div>
          </template>
        </div>

        <!-- Chunk mini-indicators -->
        <div v-if="u.totalParts > 1 && u.status === 'uploading'" class="chunk-strip">
          <div
            v-for="c in u.chunks"
            :key="c.index"
            class="chunk-dot"
            :class="c.status"
            :title="`Part ${c.index + 1}: ${c.status}${c.retries ? ' (retry ' + c.retries + ')' : ''}`"
          />
        </div>
      </div>
    </div>

    <!-- Done state -->
    <div v-if="allDone" class="done-bar">
      <span>{{ uploads.filter(u => u.status === 'done').length }}/{{ uploads.length }} uploaded</span>
      <button class="btn-done" @click="emit('close')">Close</button>
    </div>
  </div>
</template>

<style scoped>
/* ── Shell ─────────────────────────────────────────────────────────────────── */
.uploader {
  display: flex;
  flex-direction: column;
  gap: 0;
  background: rgba(18, 18, 24, 0.96);
  border: 1px solid rgba(255,255,255,0.08);
  border-radius: 16px;
  backdrop-filter: blur(32px) saturate(160%);
  box-shadow: 0 32px 80px rgba(0,0,0,0.6), 0 0 0 0.5px rgba(255,255,255,0.04);
  overflow: hidden;
  width: 480px;
  max-height: 70vh;
  display: flex;
  flex-direction: column;
}

/* ── Header ────────────────────────────────────────────────────────────────── */
.up-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 12px 16px 10px;
  border-bottom: 1px solid rgba(255,255,255,0.06);
  background: rgba(0,0,0,0.2);
}
.up-title { font-size: 13px; font-weight: 600; color: rgba(255,255,255,0.9); }
.up-sub { font-size: 11px; color: rgba(255,255,255,0.3); flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.up-close { margin-left: auto; background: none; border: none; color: rgba(255,255,255,0.3); cursor: pointer; font-size: 13px; padding: 2px 6px; border-radius: 6px; transition: all 120ms; }
.up-close:hover { background: rgba(255,255,255,0.08); color: white; }

/* ── Drop zone ──────────────────────────────────────────────────────────────── */
.dropzone {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 6px;
  margin: 14px;
  padding: 24px;
  border: 1.5px dashed rgba(255,255,255,0.12);
  border-radius: 12px;
  cursor: pointer;
  transition: all 200ms;
  background: rgba(255,255,255,0.02);
}
.dropzone:hover, .dropzone.dragging {
  border-color: rgba(96,165,250,0.6);
  background: rgba(96,165,250,0.06);
}
.hidden-inp { display: none; }
.dz-icon { font-size: 28px; line-height: 1; transition: transform 200ms; }
.dropzone.dragging .dz-icon { transform: scale(1.15); }
.dz-label { font-size: 13px; color: rgba(255,255,255,0.7); }
.dz-link { color: #60a5fa; font-weight: 500; }
.dz-sub { font-size: 11px; color: rgba(255,255,255,0.25); }

/* ── Overall progress ───────────────────────────────────────────────────────── */
.overall-bar-wrap {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 0 14px 10px;
}
.overall-bar-track {
  flex: 1;
  height: 3px;
  border-radius: 99px;
  background: rgba(255,255,255,0.07);
  overflow: hidden;
}
.overall-bar-fill {
  height: 100%;
  border-radius: 99px;
  background: linear-gradient(90deg, #60a5fa, #a78bfa);
  transition: width 300ms ease;
  box-shadow: 0 0 8px rgba(96,165,250,0.5);
}
.overall-pct { font-size: 11px; color: rgba(255,255,255,0.35); min-width: 34px; text-align: right; }

/* ── File list ──────────────────────────────────────────────────────────────── */
.file-list {
  flex: 1;
  overflow-y: auto;
  padding: 0 14px 14px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.file-row {
  border-radius: 10px;
  background: rgba(255,255,255,0.04);
  border: 1px solid rgba(255,255,255,0.06);
  overflow: hidden;
  transition: border-color 200ms;
}
.file-row.done { border-color: rgba(52,211,153,0.25); }
.file-row.err  { border-color: rgba(248,113,113,0.25); }

.fr-left {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  padding: 10px 12px 6px;
}
.fr-icon { font-size: 18px; width: 22px; text-align: center; flex-shrink: 0; padding-top: 2px; }
.status-icon { font-size: 14px; }
.file-row.done .status-icon { color: #34d399; }
.file-row.err  .status-icon { color: #f87171; }

.fr-info { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 2px; }
.fr-name { font-size: 12px; font-weight: 500; color: rgba(255,255,255,0.9); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.fr-meta { font-size: 10px; color: rgba(255,255,255,0.3); }
.fr-err  { font-size: 10px; color: #f87171; }

.fr-right { display: flex; align-items: center; padding: 0 12px 8px 44px; }

/* progress */
.progress-wrap { display: flex; align-items: center; gap: 8px; flex: 1; }
.progress-track {
  flex: 1;
  height: 4px;
  border-radius: 99px;
  background: rgba(255,255,255,0.07);
  overflow: hidden;
}
.progress-fill {
  height: 100%;
  border-radius: 99px;
  background: linear-gradient(90deg, #60a5fa, #a78bfa);
  transition: width 200ms linear;
  box-shadow: 0 0 6px rgba(96,165,250,0.4);
  animation: glow-pulse 2s ease-in-out infinite;
}
@keyframes glow-pulse {
  0%, 100% { box-shadow: 0 0 4px rgba(96,165,250,0.3); }
  50%       { box-shadow: 0 0 12px rgba(167,139,250,0.5); }
}
.pct-label { font-size: 10px; color: rgba(255,255,255,0.35); min-width: 28px; text-align: right; }
.pct-done { font-size: 11px; color: #34d399; font-weight: 500; }

.retry-btn {
  padding: 3px 10px;
  border-radius: 6px;
  font-size: 11px;
  font-weight: 500;
  color: #fb923c;
  background: rgba(251,146,60,0.12);
  border: 1px solid rgba(251,146,60,0.25);
  cursor: pointer;
  transition: all 120ms;
}
.retry-btn:hover { background: rgba(251,146,60,0.22); color: white; }

/* Chunk dots */
.chunk-strip {
  display: flex;
  flex-wrap: wrap;
  gap: 3px;
  padding: 0 12px 8px 44px;
}
.chunk-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: rgba(255,255,255,0.12);
  transition: background 200ms;
}
.chunk-dot.done     { background: #34d399; }
.chunk-dot.uploading{ background: #60a5fa; animation: blink 1s ease-in-out infinite; }
.chunk-dot.hashing  { background: #facc15; animation: blink 0.7s ease-in-out infinite; }
.chunk-dot.error    { background: #f87171; }
@keyframes blink { 0%,100% { opacity:1 } 50% { opacity:0.35 } }

/* Spinner */
.spinner {
  display: inline-block;
  width: 14px; height: 14px;
  border: 2px solid rgba(255,255,255,0.12);
  border-top-color: #60a5fa;
  border-radius: 50%;
  animation: spin 700ms linear infinite;
}
@keyframes spin { to { transform: rotate(360deg); } }

/* Done bar */
.done-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 14px;
  border-top: 1px solid rgba(255,255,255,0.06);
  background: rgba(0,0,0,0.15);
  font-size: 12px;
  color: rgba(255,255,255,0.4);
}
.btn-done {
  padding: 4px 14px;
  border-radius: 8px;
  background: rgba(52,211,153,0.15);
  border: 1px solid rgba(52,211,153,0.3);
  color: #34d399;
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  transition: all 120ms;
}
.btn-done:hover { background: rgba(52,211,153,0.25); color: white; }
</style>
