import { ref } from 'vue'
import type { CanvasStroke } from '../types'
import { wsSend, myId, myColor } from './useWs'

// Per-canvas rendering context
const contexts = new Map<string, CanvasRenderingContext2D>()
const drawing = new Map<string, boolean>()
const currentStroke = new Map<string, [number, number][]>()

export const brushSizes = ref<Record<string, number>>({})

export function registerCanvas(canvasId: string, el: HTMLCanvasElement) {
  const ctx = el.getContext('2d')
  if (!ctx) return
  // White-ish dark background
  ctx.fillStyle = '#1e1e1e'
  ctx.fillRect(0, 0, el.width, el.height)
  contexts.set(canvasId, ctx)
}

export function sizeCanvas(canvasId: string, w: number, h: number) {
  const ctx = contexts.get(canvasId)
  if (!ctx) return
  // Save current pixels
  const img = ctx.getImageData(0, 0, ctx.canvas.width, ctx.canvas.height)
  ctx.canvas.width = w
  ctx.canvas.height = h
  ctx.fillStyle = '#1e1e1e'
  ctx.fillRect(0, 0, w, h)
  try { ctx.putImageData(img, 0, 0) } catch {}
}

export function drawStroke(stroke: CanvasStroke) {
  const ctx = contexts.get(stroke.canvas_id)
  if (!ctx || stroke.points.length < 2) return
  ctx.beginPath()
  ctx.moveTo(stroke.points[0][0], stroke.points[0][1])
  for (let i = 1; i < stroke.points.length; i++) ctx.lineTo(stroke.points[i][0], stroke.points[i][1])
  ctx.strokeStyle = stroke.color
  ctx.lineWidth = stroke.size
  ctx.lineCap = 'round'
  ctx.lineJoin = 'round'
  ctx.stroke()
}

export function clearCanvas(canvasId: string) {
  const ctx = contexts.get(canvasId)
  if (!ctx) return
  ctx.fillStyle = '#1e1e1e'
  ctx.clearRect(0, 0, ctx.canvas.width, ctx.canvas.height)
  ctx.fillRect(0, 0, ctx.canvas.width, ctx.canvas.height)
}

export function onCanvasPointerDown(e: PointerEvent, canvasId: string) {
  ;(e.target as HTMLElement).setPointerCapture(e.pointerId)
  drawing.set(canvasId, true)
  const rect = (e.target as HTMLCanvasElement).getBoundingClientRect()
  currentStroke.set(canvasId, [[e.clientX - rect.left, e.clientY - rect.top]])
}

export function onCanvasPointerMove(e: PointerEvent, canvasId: string) {
  if (!drawing.get(canvasId)) return
  const rect = (e.target as HTMLCanvasElement).getBoundingClientRect()
  const pt: [number, number] = [e.clientX - rect.left, e.clientY - rect.top]
  const path = currentStroke.get(canvasId) ?? []
  path.push(pt)
  currentStroke.set(canvasId, path)

  // Draw locally for zero latency
  const ctx = contexts.get(canvasId)
  if (ctx && path.length >= 2) {
    ctx.beginPath()
    ctx.moveTo(path[path.length - 2][0], path[path.length - 2][1])
    ctx.lineTo(pt[0], pt[1])
    ctx.strokeStyle = myColor.value
    ctx.lineWidth = brushSizes.value[canvasId] ?? 4
    ctx.lineCap = 'round'
    ctx.lineJoin = 'round'
    ctx.stroke()
  }
}

export function onCanvasPointerUp(canvasId: string) {
  if (!drawing.get(canvasId)) return
  drawing.set(canvasId, false)
  const path = currentStroke.get(canvasId) ?? []
  currentStroke.delete(canvasId)
  if (path.length < 2) return

  const stroke: CanvasStroke = {
    canvas_id: canvasId,
    user_id: myId.value,
    color: myColor.value,
    size: brushSizes.value[canvasId] ?? 4,
    points: path,
  }
  wsSend({ type: 'CanvasDraw', stroke })
}

export function disposeCanvas(canvasId: string) {
  contexts.delete(canvasId)
  drawing.delete(canvasId)
  currentStroke.delete(canvasId)
}
