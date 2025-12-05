<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from 'vue'
import { wsSend } from '../../composables/useWs'

defineProps<{ winId: string }>()

// ─── Display ──────────────────────────────────────────────────────────────────
const displays = ref<{ name: string; description: string }[]>([])
const selectedDisplay = ref<string | null>(null)
const isLive = ref(false)
const isReady = ref(false)

const fetchDisplays = async () => {
  try {
    const res = await fetch('/api/displays')
    if (res.ok) {
      displays.value = await res.json()
      if (!selectedDisplay.value && displays.value.length > 0)
        selectedDisplay.value = displays.value[0].name
    }
  } catch {}
}

// ─── WebGL / Three.js state (imperative, not reactive) ───────────────────────
const canvasEl = ref<HTMLCanvasElement | null>(null)
let THREE: any = null
let renderer: any = null
let uniforms: any = null
let loader: any = null
let roDisconnect: (() => void) | null = null
let running = false

const MORPH_MS = 180  // longer than 100ms frame interval → transitions always overlap

const VERT = /* glsl */`
  varying vec2 vUv;
  void main() { vUv = uv; gl_Position = vec4(position.xy, 0.0, 1.0); }
`

const FRAG = /* glsl */`
  uniform sampler2D tA;
  uniform sampler2D tB;
  uniform float progress;
  varying vec2 vUv;

  // Smooth step — no overshoot, no oscillation
  float sineEase(float t) {
    return 0.5 - 0.5 * cos(t * 3.14159265);
  }

  void main() {
    vec2 uv = vUv;
    float p = sineEase(progress); // smooth 0→1

    // Gradient-based micro-shift:
    // Sample the difference between frames as a motion proxy.
    // Only pixels that actually changed get displaced — text/UI
    // stays pin-sharp, moving regions get a subtle motion smear.
    const float D = 0.002;  // tiny sample offset for gradient
    vec4 a0 = texture2D(tA, uv);
    vec4 b0 = texture2D(tB, uv);

    // Per-channel luma difference → magnitude of local motion
    float diffLum = dot(abs(b0.rgb - a0.rgb), vec3(0.299, 0.587, 0.114));

    // Spatial gradient of frame B drives warp direction
    // (approximates optical flow at almost zero cost)
    float gx = texture2D(tB, uv + vec2(D, 0.0)).r - texture2D(tB, uv - vec2(D, 0.0)).r;
    float gy = texture2D(tB, uv + vec2(0.0, D)).r - texture2D(tB, uv - vec2(0.0, D)).r;
    vec2 grad = vec2(gx, gy);

    // Scale: only high-diff regions get any shift; warp is tiny overall
    float W = diffLum * 0.018;
    vec2 shiftA = grad * W * p;
    vec2 shiftB = grad * W * (p - 1.0);

    vec4 cA = texture2D(tA, clamp(uv + shiftA, vec2(0.0), vec2(1.0)));
    vec4 cB = texture2D(tB, clamp(uv + shiftB, vec2(0.0), vec2(1.0)));

    gl_FragColor = mix(cA, cB, p);
  }
`

// ─── Frame queue ──────────────────────────────────────────────────────────────
const MAX_QUEUE = 4
const texQueue: any[] = []
let morphing = false
let morphStart = 0

function preloadTex(url: string): Promise<any> {
  return new Promise((resolve, reject) => {
    loader.load(url,
      (t: any) => { t.minFilter = THREE.LinearFilter; t.magFilter = THREE.LinearFilter; resolve(t) },
      undefined,
      reject
    )
  })
}

function onMorphDone() {
  uniforms.tA.value = uniforms.tB.value
  uniforms.progress.value = 0
  morphing = false
  pumpQueue()
}

function pumpQueue() {
  if (morphing || texQueue.length === 0) return
  const next = texQueue.shift()
  uniforms.tB.value = next
  morphStart = performance.now()
  morphing = true
}

// ─── Three.js initialization ─────────────────────────────────────────────────
function loadThree(): Promise<void> {
  if ((window as any).THREE) return Promise.resolve()
  return new Promise((resolve, reject) => {
    const s = document.createElement('script')
    s.src = '/three.min.js'
    s.onload = () => resolve()
    s.onerror = () => reject(new Error('Failed to load three.min.js'))
    document.head.appendChild(s)
  })
}

function initGL() {
  THREE = (window as any).THREE
  const cnv = canvasEl.value!

  renderer = new THREE.WebGLRenderer({ canvas: cnv, antialias: false })
  renderer.setPixelRatio(1)

  const scene = new THREE.Scene()
  const camera = new THREE.OrthographicCamera(-1, 1, 1, -1, 0, 1)

  uniforms = {
    tA:       { value: null },
    tB:       { value: null },
    progress: { value: 0.0 },
  }

  scene.add(new THREE.Mesh(
    new THREE.PlaneGeometry(2, 2),
    new THREE.ShaderMaterial({ uniforms, vertexShader: VERT, fragmentShader: FRAG })
  ))

  loader = new THREE.TextureLoader()
  loader.crossOrigin = 'anonymous'

  // Responsive canvas size
  const ro = new ResizeObserver(() => {
    renderer.setSize(cnv.clientWidth, cnv.clientHeight, false)
  })
  ro.observe(cnv)
  renderer.setSize(cnv.clientWidth, cnv.clientHeight, false)
  roDisconnect = () => ro.disconnect()

  // Render loop
  running = true
  const loop = () => {
    if (!running) return
    requestAnimationFrame(loop)

    if (morphing) {
      const raw = Math.min((performance.now() - morphStart) / MORPH_MS, 1)
      uniforms.progress.value = raw  // shader does its own sine ease internally
      if (raw >= 1) onMorphDone()
    }

    renderer.render(scene, camera)
  }
  loop()
}

// ─── WS listener (dedicated socket) ──────────────────────────────────────────
let ws: WebSocket | null = null

const handleMsg = (e: MessageEvent) => {
  try {
    const msg = JSON.parse(e.data)
    if (msg.type !== 'ScreenFrame' || msg.display !== selectedDisplay.value || !isLive.value) return

    // Server sends base64-encoded JPEG directly — create a blob URL
    const binary = atob(msg.data)
    const bytes = new Uint8Array(binary.length)
    for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i)
    const blob = new Blob([bytes], { type: 'image/jpeg' })
    const url = URL.createObjectURL(blob)

    // Preload as texture — fire & forget
    preloadTex(url).then(tex => {
      URL.revokeObjectURL(url) // free blob memory
      if (!isLive.value) { tex.dispose(); return }

      // Bootstrap first frame: tA = tB = first tex, no morph needed
      if (!uniforms.tA.value) {
        uniforms.tA.value = tex
        uniforms.tB.value = tex
        isReady.value = true
        return
      }

      // Drop oldest if queue is full (slow connection / fast capture)
      if (texQueue.length >= MAX_QUEUE) {
        const dropped = texQueue.shift()
        dropped?.dispose()
      }
      texQueue.push(tex)
      pumpQueue()
    }).catch(() => { URL.revokeObjectURL(url) })
  } catch {}
}

const connectWs = () => {
  const proto = location.protocol === 'https:' ? 'wss' : 'ws'
  ws = new WebSocket(`${proto}://${location.host}/_ws`)
  ws.addEventListener('message', handleMsg)
  ws.addEventListener('close', () => {
    ws = null
    if (isLive.value) setTimeout(connectWs, 1000)
  })
}

// ─── Stream lifecycle ─────────────────────────────────────────────────────────
const flush = () => {
  morphing = false
  texQueue.forEach(t => t?.dispose())
  texQueue.length = 0
  isReady.value = false
  if (uniforms) {
    uniforms.tA.value = null
    uniforms.tB.value = null
    uniforms.progress.value = 0
  }
}

const startStream = () => {
  if (!selectedDisplay.value) return
  isLive.value = true
  wsSend({ type: 'StartStream', display: selectedDisplay.value })
}

const stopStream = () => {
  if (!selectedDisplay.value) return
  isLive.value = false
  wsSend({ type: 'StopStream', display: selectedDisplay.value })
  flush()
}

const toggleLive = () => isLive.value ? stopStream() : startStream()

watch(selectedDisplay, (next, prev) => {
  if (prev && isLive.value) wsSend({ type: 'StopStream', display: prev })
  flush()
  if (next && isLive.value) wsSend({ type: 'StartStream', display: next })
})

// ─── Lifecycle ────────────────────────────────────────────────────────────────
onMounted(async () => {
  connectWs()
  fetchDisplays()
  await loadThree()
  initGL()
})

onUnmounted(() => {
  running = false
  stopStream()
  if (ws) { ws.removeEventListener('message', handleMsg); ws.close(); ws = null }
  roDisconnect?.()
  if (renderer) { renderer.dispose(); renderer = null }
})
</script>

<template>
  <div class="flex flex-col h-full overflow-hidden"
    style="background:#0a0a0b; color:#c8c8d0; font-family:'IBM Plex Mono',monospace">

    <!-- Header -->
    <div class="flex-none flex items-center justify-between px-3 py-2 border-b"
      style="border-color:#1e1e24; background:#111114">

      <select :value="selectedDisplay"
        @change="e => (selectedDisplay = (e.target as HTMLSelectElement).value)"
        class="rounded px-2 py-1 outline-none cursor-pointer text-[12px]"
        style="background:rgba(255,255,255,0.05); border:1px solid #1e1e24; color:#c8c8d0; font-family:inherit">
        <option v-for="d in displays" :key="d.name" :value="d.name" style="background:#111114">
          {{ d.name }} — {{ d.description }}
        </option>
      </select>

      <button @click="toggleLive" :disabled="!selectedDisplay"
        class="flex items-center gap-2 px-3 py-1 text-[10px] uppercase tracking-widest transition-all duration-150 disabled:opacity-25"
        style="font-family:inherit; border:1px solid; background:transparent; cursor:pointer"
        :style="isLive
          ? 'border-color:rgba(239,68,68,0.5); color:#f87171'
          : 'border-color:rgba(74,222,128,0.3); color:#4ade80'">
        <span v-if="isLive" class="relative flex h-1.5 w-1.5">
          <span class="animate-ping absolute inline-flex h-full w-full rounded-full opacity-75" style="background:#f87171"></span>
          <span class="relative inline-flex rounded-full h-1.5 w-1.5" style="background:#ef4444"></span>
        </span>
        {{ isLive ? '■ stop' : '▶ live' }}
      </button>
    </div>

    <!-- WebGL Canvas -->
    <div class="flex-1 relative overflow-hidden" style="background:#000">
      <canvas ref="canvasEl" class="absolute inset-0 w-full h-full" />

      <!-- Idle / Loading overlay -->
      <Transition name="fade">
        <div v-if="!isLive || !isReady"
          class="absolute inset-0 flex flex-col items-center justify-center gap-4"
          style="background:rgba(0,0,0,0.88)">
          <div v-if="isLive && !isReady"
            class="w-7 h-7 rounded-full border-2 animate-spin"
            style="border-color:#ffb300; border-top-color:transparent" />
          <svg v-else xmlns="http://www.w3.org/2000/svg" width="46" height="46"
            viewBox="0 0 24 24" fill="none" stroke="#2a2a38" stroke-width="0.75">
            <rect x="2" y="3" width="20" height="14" rx="2" ry="2"/>
            <line x1="8" y1="21" x2="16" y2="21"/>
            <line x1="12" y1="17" x2="12" y2="21"/>
          </svg>
          <span class="text-[10px] tracking-[0.2em] uppercase" style="color:#3a3a50">
            {{ isLive ? 'waiting for first frame…' : 'press ▶ live to start' }}
          </span>
        </div>
      </Transition>

      <!-- LIVE badge -->
      <div v-if="isLive && isReady"
        class="absolute top-2.5 left-3 flex items-center gap-1.5 px-2 py-0.5 text-[9px] font-bold uppercase tracking-[0.18em]"
        style="background:rgba(239,68,68,0.8); color:#fff; backdrop-filter:blur(4px)">
        <span class="relative flex h-1.5 w-1.5">
          <span class="animate-ping absolute inline-flex h-full w-full rounded-full bg-red-200 opacity-75"></span>
          <span class="relative inline-flex rounded-full h-1.5 w-1.5 bg-white"></span>
        </span>
        LIVE
      </div>
    </div>

  </div>
</template>

<style scoped>
.fade-enter-active, .fade-leave-active { transition: opacity 0.3s ease; }
.fade-enter-from, .fade-leave-to { opacity: 0; }
</style>
