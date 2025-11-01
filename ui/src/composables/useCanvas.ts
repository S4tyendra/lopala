import { ref } from 'vue'
import type { CanvasStroke } from '../types'
import { wsSend, myId, myColor } from './useWs'

const contexts = new Map<string, CanvasRenderingContext2D>()
const drawing = new Map<string, boolean>()
const currentStroke = new Map<string, [number, number][]>()
const lastBroadcastPt = new Map<string, [number, number]>()

export const brushSizes = ref<Record<string, number>>({})

export function registerCanvas(canvasId: string, el: HTMLCanvasElement) {
  const ctx = el.getContext('2d')
  if (!ctx) return
  ctx.fillStyle = '#1e1e1e'
  ctx.fillRect(0, 0, el.width, el.height)
  contexts.set(canvasId, ctx)
}

export function sizeCanvas(canvasId: string, w: number, h: number) {
  const ctx = contexts.get(canvasId)
  if (!ctx) return
  const img = ctx.getImageData(0, 0, ctx.canvas.width, ctx.canvas.height)
  ctx.canvas.width = w
  ctx.canvas.height = h
  ctx.fillStyle = '#1e1e1e'
  ctx.fillRect(0, 0, w, h)
  try { ctx.putImageData(img, 0, 0) } catch {}
}

function drawLine(ctx: CanvasRenderingContext2D, from: [number,number], to: [number,number], color: string, size: number) {
  ctx.beginPath()
  ctx.moveTo(from[0], from[1])
  ctx.lineTo(to[0], to[1])
  ctx.strokeStyle = color
  ctx.lineWidth = size
  ctx.lineCap = 'round'
  ctx.lineJoin = 'round'
  ctx.stroke()
}

// Draw a remote live line (from CanvasLiveLine event)
export function drawRemoteLine(canvasId: string, from: [number,number], to: [number,number], color: string, size: number) {
  const ctx = contexts.get(canvasId)
  if (!ctx) return
  drawLine(ctx, from, to, color, size)
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
  const pt: [number, number] = [e.clientX - rect.left, e.clientY - rect.top]
  currentStroke.set(canvasId, [pt])
  lastBroadcastPt.set(canvasId, pt)
}

export function onCanvasPointerMove(e: PointerEvent, canvasId: string) {
  if (!drawing.get(canvasId)) return
  const rect = (e.target as HTMLCanvasElement).getBoundingClientRect()
  const pt: [number, number] = [e.clientX - rect.left, e.clientY - rect.top]
  const path = currentStroke.get(canvasId) ?? []
  const prev = path[path.length - 1] ?? pt
  path.push(pt)
  currentStroke.set(canvasId, path)

  const size = brushSizes.value[canvasId] ?? 4
  const color = myColor.value

  // Draw locally immediately
  const ctx = contexts.get(canvasId)
  if (ctx) drawLine(ctx, prev, pt, color, size)

  // Broadcast live line segment to other users
  const lastBP = lastBroadcastPt.get(canvasId) ?? prev
  const dx = pt[0] - lastBP[0], dy = pt[1] - lastBP[1]
  if (dx * dx + dy * dy > 16) { // min 4px movement before broadcasting
    lastBroadcastPt.set(canvasId, pt)
    wsSend({
      type: 'CanvasLiveLine',
      canvas_id: canvasId,
      user_id: myId.value,
      color,
      size,
      from: prev,
      to: pt,
    })
  }
}

export function onCanvasPointerUp(canvasId: string) {
  if (!drawing.get(canvasId)) return
  drawing.set(canvasId, false)
  const path = currentStroke.get(canvasId) ?? []
  currentStroke.delete(canvasId)
  lastBroadcastPt.delete(canvasId)
  if (path.length < 2) return

  // Send full stroke for server-side history persistence
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
  lastBroadcastPt.delete(canvasId)
}
