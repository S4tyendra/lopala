import { ref, computed, nextTick } from 'vue'
import type { AppWindow } from '../types'
import { windows, currentWorkspace, wsSend } from './useWs'
import { LOGICAL_W, LOGICAL_H } from '../types'

let zTop = 1000

export function nextZ() { return ++zTop }

// ─── Scale helpers for cross-resolution sync ─────────────────────────────────
// Positions are stored in logical 1920×1080 units.
// Callers pass logical coords; render converts to screen coords.
export function logicalToScreen(lx: number, ly: number, lw: number, lh: number) {
  const scaleX = window.innerWidth / LOGICAL_W
  const scaleY = window.innerHeight / LOGICAL_H
  return {
    x: Math.round(lx * scaleX),
    y: Math.round(ly * scaleY),
    w: Math.round(lw * scaleX),
    h: Math.round(lh * scaleY),
  }
}

export function screenToLogical(sx: number, sy: number, sw: number, sh: number) {
  const scaleX = LOGICAL_W / window.innerWidth
  const scaleY = LOGICAL_H / window.innerHeight
  return {
    x: Math.round(sx * scaleX),
    y: Math.round(sy * scaleY),
    w: Math.round(sw * scaleX),
    h: Math.round(sh * scaleY),
  }
}

// Clamp a screen-space window so it's never fully off screen
function clampScreen(x: number, y: number, w: number, h: number) {
  const minVisible = 60
  return {
    x: Math.max(-(w - minVisible), Math.min(x, window.innerWidth - minVisible)),
    y: Math.max(28, Math.min(y, window.innerHeight - 60)),
    w: Math.max(240, Math.min(w, window.innerWidth)),
    h: Math.max(150, Math.min(h, window.innerHeight - 28 - 70)),
  }
}

// Get screen-space rect for a window stored in logical coords
export function winScreenRect(win: AppWindow) {
  const { x, y, w, h } = logicalToScreen(win.x, win.y, win.w, win.h)
  return clampScreen(x, y, w, h)
}

export function broadcastWin(win: AppWindow) {
  wsSend({ type: 'UpdateWindow', window: win })
}

export function focusWindow(id: string) {
  const win = windows.value[id]
  if (!win) return
  win.z = nextZ()
  broadcastWin(win)
}

export function closeWindow(id: string) {
  wsSend({ type: 'CloseWindow', id })
}

export function toggleMaximize(win: AppWindow) {
  if (win.maximized) {
    // Restore
    win.maximized = false
    win.x = win._px ?? win.x; win.y = win._py ?? win.y
    win.w = win._pw ?? win.w; win.h = win._ph ?? win.h
  } else {
    // Save current logical, set to full screen in logical coords
    win._px = win.x; win._py = win.y; win._pw = win.w; win._ph = win.h
    win.maximized = true
    win.x = 0; win.y = Math.round(28 * LOGICAL_H / window.innerHeight)
    win.w = LOGICAL_W; win.h = Math.round((window.innerHeight - 28 - 70) * LOGICAL_H / window.innerHeight)
  }
  broadcastWin(win)
}

export function spawnWindow(app: string, extra: Partial<AppWindow> = {}): string {
  const count = Object.values(windows.value).filter(w => w.workspace === currentWorkspace.value).length
  const id = Math.random().toString(36).substring(7)

  // Default logical sizes
  const logW = app === 'messages' ? 440 : app === 'canvas' ? 640 : 700
  const logH = app === 'messages' ? 520 : 460

  // Stagger in logical space
  const offsetX = Math.round((80 + (count * 28) % 200) * LOGICAL_W / window.innerWidth)
  const offsetY = Math.round((56 + (count * 28) % 150) * LOGICAL_H / window.innerHeight)

  const win: AppWindow = {
    id, app, workspace: currentWorkspace.value,
    x: offsetX, y: offsetY,
    w: Math.round(logW * LOGICAL_W / window.innerWidth),
    h: Math.round(logH * LOGICAL_H / window.innerHeight),
    z: nextZ(), minimized: false, maximized: false, title: '',
    ...extra,
  }

  wsSend({ type: 'SpawnWindow', window: win })
  return id
}

export const visibleWindows = computed(() =>
  Object.values(windows.value)
    .filter(w => w.workspace === currentWorkspace.value && !w.minimized)
    .sort((a, b) => a.z - b.z)
)

// ─── Drag / Resize (screen-space deltas, stored as logical) ──────────────────
interface DragState { active: boolean; winId: string; startX: number; startY: number; origLX: number; origLY: number }
interface ResizeState { active: boolean; winId: string; startX: number; startY: number; origLW: number; origLH: number; edge: string }

const drag = ref<DragState>({ active: false, winId: '', startX: 0, startY: 0, origLX: 0, origLY: 0 })
const resize = ref<ResizeState>({ active: false, winId: '', startX: 0, startY: 0, origLW: 0, origLH: 0, edge: '' })

export const isDragging = computed(() => drag.value.active)
export const draggingWinId = computed(() => drag.value.winId)

export function startDrag(e: MouseEvent, win: AppWindow) {
  if (win.maximized) return
  focusWindow(win.id)
  drag.value = { active: true, winId: win.id, startX: e.clientX, startY: e.clientY, origLX: win.x, origLY: win.y }
}

export function startResize(e: MouseEvent, win: AppWindow, edge: string) {
  if (win.maximized) return
  e.preventDefault(); e.stopPropagation()
  focusWindow(win.id)
  resize.value = { active: true, winId: win.id, startX: e.clientX, startY: e.clientY, origLW: win.w, origLH: win.h, edge }
}

export function onDragMove(e: MouseEvent) {
  const scaleX = LOGICAL_W / window.innerWidth
  const scaleY = LOGICAL_H / window.innerHeight

  if (drag.value.active) {
    const win = windows.value[drag.value.winId]
    if (!win) return
    const dxL = Math.round((e.clientX - drag.value.startX) * scaleX)
    const dyL = Math.round((e.clientY - drag.value.startY) * scaleY)
    win.x = drag.value.origLX + dxL
    win.y = Math.max(Math.round(28 * scaleY), drag.value.origLY + dyL)
    broadcastWin(win)
  }

  if (resize.value.active) {
    const win = windows.value[resize.value.winId]
    if (!win) return
    const dxL = Math.round((e.clientX - resize.value.startX) * scaleX)
    const dyL = Math.round((e.clientY - resize.value.startY) * scaleY)
    if (resize.value.edge.includes('e')) win.w = Math.max(Math.round(240 * scaleX), resize.value.origLW + dxL)
    if (resize.value.edge.includes('s')) win.h = Math.max(Math.round(150 * scaleY), resize.value.origLH + dyL)
    broadcastWin(win)
  }
}

export function onDragEnd(): { resizedWinId: string | null } {
  const resizedWinId = resize.value.active ? resize.value.winId : null
  drag.value.active = false
  resize.value.active = false
  return { resizedWinId }
}
