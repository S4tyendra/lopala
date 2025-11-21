import { ref } from 'vue'
import { wsSend, myId } from './useWs'

export interface ScreenshotState {
  display: string | null
  openedImage: string | null
  scrollTop: number
  version: number
}

export interface ScreenshotStateSync {
  display: string | null
  opened_image: string | null
  scroll_top: number
  version: number
  sender: string
}

export const globalScreenshotState = ref<ScreenshotState>({
  display: null,
  openedImage: null,
  scrollTop: 0,
  version: 0,
})

let isApplyingRemoteSync = false

export function bumpScreenshotVersion() {
  globalScreenshotState.value.version++
}

export function broadcastScreenshotState() {
  if (isApplyingRemoteSync) return
  const s = globalScreenshotState.value
  const sync: ScreenshotStateSync = {
    display: s.display,
    opened_image: s.openedImage,
    scroll_top: s.scrollTop,
    version: s.version,
    sender: myId.value,
  }
  wsSend({ type: 'ScreenshotSync', state: sync })
}

export function applyRemoteScreenshotState(sync: ScreenshotStateSync) {
  if (sync.sender === myId.value) return
  isApplyingRemoteSync = true
  const s = globalScreenshotState.value
  
  s.display = sync.display
  s.openedImage = sync.opened_image
  s.scrollTop = sync.scroll_top
  s.version = sync.version
  
  isApplyingRemoteSync = false
}
