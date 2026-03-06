// Shared type definitions for the entire app

export interface AppWindow {
  id: string
  app: string
  workspace: number
  x: number
  y: number
  w: number
  h: number
  z: number
  minimized: boolean
  maximized: boolean
  title: string
  local?: boolean
  channel?: string
  canvasId?: string
  // store pre-max state inline
  _px?: number; _py?: number; _pw?: number; _ph?: number
  args?: any // For passing data on spawn, e.g. which file to open
  initialPath?: string
  initialSelect?: string
}

export interface User {
  id: string
  name: string
  x: number
  y: number
  workspace: number
  color: string
  latency_ms?: number
}

export interface ChatMessage {
  id: string
  channel: string
  user_name: string
  content: string
  timestamp: number
}

export interface Channel {
  id: string
  name: string   // emoji only, e.g. '#', '🔥'
  created_by: string
}

export interface FileEntry {
  name: string
  path: string
  is_dir: boolean
  size: number
  modified: number
  mime: string
}

export interface CanvasStroke {
  canvas_id: string
  user_id: string
  color: string
  size: number
  points: [number, number][]
}

// Reference logical resolution — positions are stored in these units
// and scaled to actual viewport on render
export const LOGICAL_W = 1920
export const LOGICAL_H = 1080
