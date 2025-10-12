<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted, nextTick, computed } from 'vue';
import { Terminal } from 'xterm';
import { FitAddon } from 'xterm-addon-fit';
import 'xterm/css/xterm.css';

// ─── Types ────────────────────────────────────────────────────────────────────
interface AppWindow {
  id: string; app: string; workspace: number;
  x: number; y: number; w: number; h: number; z: number;
  minimized: boolean; maximized: boolean; title: string;
  channel?: string; canvasId?: string;
  _px?: number; _py?: number; _pw?: number; _ph?: number;
}
interface User { id: string; name: string; x: number; y: number; workspace: number; color: string; }
interface ChatMessage { id: string; channel: string; user_name: string; content: string; timestamp: number; }
interface Channel { id: string; name: string; created_by: string; }
interface FileEntry { name: string; path: string; is_dir: boolean; size: number; modified: number; mime: string; }
interface CanvasStroke { canvas_id: string; user_id: string; color: string; size: number; points: [number,number][]; }

// ─── Identity ─────────────────────────────────────────────────────────────────
const myId = ref(Math.random().toString(36).substring(7));
const myName = ref('');
const myColor = ref('#0a84ff');
const showNamePrompt = ref(true);

// ─── Shared State ─────────────────────────────────────────────────────────────
const currentWorkspace = ref(0);
const workspaceCount = ref(4);
const users = ref<Record<string, User>>({});
const windows = ref<Record<string, AppWindow>>({});
const chats = ref<ChatMessage[]>([]);
const channels = ref<Channel[]>([]);

// ─── WebSocket ────────────────────────────────────────────────────────────────
const ws = ref<WebSocket | null>(null);

// ─── Terminal instances ───────────────────────────────────────────────────────
const terminals = new Map<string, { term: Terminal, fitAddon: FitAddon }>();

// ─── File Manager State ───────────────────────────────────────────────────────
const fileStates = ref<Record<string, { path: string; entries: FileEntry[]; loading: boolean; selected: FileEntry | null; preview: string | null; viewMode: 'grid' | 'list'; }>>({});

// ─── Canvas State ─────────────────────────────────────────────────────────────
const canvasContexts = new Map<string, CanvasRenderingContext2D>();
const canvasDrawing = new Map<string, boolean>();
const canvasCurrentPath = new Map<string, [number,number][]>();
const canvasBrushSize = ref<Record<string, number>>({});

// ─── Z-index counter ───────────────────────────────────────────────────────────
let zTop = 100;
const nextZ = () => ++zTop;

// ─── Chat inputs ──────────────────────────────────────────────────────────────
const chatInputs = ref<Record<string, string>>({});
const newChannelName = ref('');

// ─── Clock ───────────────────────────────────────────────────────────────────
const clock = ref('');
const updateClock = () => { clock.value = new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }); };

// ─── Active app name (menu bar) ───────────────────────────────────────────────
const activeAppName = ref('Finder');
const appDisplayNames: Record<string, string> = {
  terminal: 'Terminal', files: 'Finder', messages: 'Messages', canvas: 'Canvas',
};

// ─── Join ─────────────────────────────────────────────────────────────────────
const join = () => {
  if (!myName.value.trim()) myName.value = `user_${myId.value.substring(0, 4)}`;
  showNamePrompt.value = false;
  connectWs();
};

const connectWs = () => {
  const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
  ws.value = new WebSocket(`${protocol}//${location.host}/_ws`);

  ws.value.onopen = () => {
    ws.value!.send(JSON.stringify({ type: 'UserJoined', user: { id: myId.value, name: myName.value, x: 0, y: 0, workspace: currentWorkspace.value, color: '' } }));
  };

  ws.value.onmessage = (e) => {
    const msg = JSON.parse(e.data);
    handleServerEvent(msg);
  };
};

const handleServerEvent = (msg: any) => {
  switch (msg.type) {
    case 'SyncState': {
      const s = msg.state;
      users.value = s.users;
      windows.value = s.windows;
      chats.value = s.chats;
      channels.value = s.channels || [];
      workspaceCount.value = s.workspace_count;
      // Get my assigned color
      if (s.users[myId.value]) myColor.value = s.users[myId.value].color;
      nextTick(checkTerminals);
      nextTick(replayCanvases);
      break;
    }
    case 'UserJoined':
      users.value[msg.user.id] = msg.user;
      if (msg.user.id === myId.value) myColor.value = msg.user.color;
      break;
    case 'UserLeft':
      delete users.value[msg.id];
      break;
    case 'CursorMove':
      if (msg.id !== myId.value && users.value[msg.id]) {
        users.value[msg.id].x = msg.x;
        users.value[msg.id].y = msg.y;
        users.value[msg.id].workspace = msg.workspace;
      }
      break;
    case 'SpawnWindow':
      windows.value[msg.window.id] = msg.window;
      nextTick(checkTerminals);
      nextTick(replayCanvases);
      break;
    case 'UpdateWindow':
      if (!dragState.isDragging || msg.window.id !== dragState.win?.id) {
        windows.value[msg.window.id] = msg.window;
      }
      break;
    case 'CloseWindow':
      delete windows.value[msg.id];
      cleanupWindow(msg.id);
      break;
    case 'UpdateTitle':
      if (windows.value[msg.id]) windows.value[msg.id].title = msg.title;
      break;
    case 'PtyOut': {
      const t = terminals.get(msg.id);
      if (t) t.term.write(msg.data);
      break;
    }
    case 'HistoryData': {
      const t = terminals.get(msg.id);
      if (t) t.term.write(msg.data);
      break;
    }
    case 'ChatMsg':
      chats.value.push(msg.msg);
      break;
    case 'ChannelCreated':
      if (!channels.value.find(c => c.id === msg.channel.id)) channels.value.push(msg.channel);
      break;
    case 'SetWorkspaceCount':
      workspaceCount.value = msg.count;
      break;
    case 'CanvasDraw':
      drawStrokeOnCanvas(msg.stroke);
      break;
    case 'CanvasClear':
      clearCanvasById(msg.canvas_id);
      break;
  }
};

// ─── Terminal management ──────────────────────────────────────────────────────
const checkTerminals = () => {
  for (const id of terminals.keys()) {
    if (!windows.value[id]) {
      terminals.get(id)?.term.dispose();
      terminals.delete(id);
    }
  }
  for (const [id, win] of Object.entries(windows.value)) {
    if (win.app === 'terminal' && !terminals.has(id)) {
      const el = document.getElementById(`term-${id}`);
      if (!el) return;
      const term = new Terminal({
        cursorBlink: true, fontSize: 14,
        fontFamily: '"JetBrains Mono Nerd Font", "JetBrains Mono", monospace',
        theme: { background: '#0c0c0c', foreground: '#d4d4d4', cursor: '#00ffcc', selectionBackground: '#ffffff33' }
      });
      const fitAddon = new FitAddon();
      term.loadAddon(fitAddon);
      term.open(el);
      fitAddon.fit();
      terminals.set(id, { term, fitAddon });
      ws.value?.send(JSON.stringify({ type: 'RequestHistory', id }));
      term.onData(data => ws.value?.send(JSON.stringify({ type: 'PtyIn', id, data })));
    }
  }
};

watch(currentWorkspace, () => nextTick(() => { checkTerminals(); replayCanvases(); }));

// ─── Canvas management ────────────────────────────────────────────────────────
const replayCanvases = () => {
  // Canvas elements are rendered; replay strokes from SyncState would need state.canvas_strokes
  // But we push live strokes via WsEvent, so just ensure context is registered
};

const registerCanvas = (canvasId: string, el: HTMLCanvasElement) => {
  const ctx = el.getContext('2d');
  if (ctx) canvasContexts.set(canvasId, ctx);
};

const drawStrokeOnCanvas = (stroke: CanvasStroke) => {
  const ctx = canvasContexts.get(stroke.canvas_id);
  if (!ctx || stroke.points.length < 2) return;
  ctx.beginPath();
  ctx.moveTo(stroke.points[0][0], stroke.points[0][1]);
  for (let i = 1; i < stroke.points.length; i++) ctx.lineTo(stroke.points[i][0], stroke.points[i][1]);
  ctx.strokeStyle = stroke.color;
  ctx.lineWidth = stroke.size;
  ctx.lineCap = 'round';
  ctx.lineJoin = 'round';
  ctx.stroke();
};

const clearCanvasById = (canvasId: string) => {
  const ctx = canvasContexts.get(canvasId);
  if (ctx) ctx.clearRect(0, 0, ctx.canvas.width, ctx.canvas.height);
};

const onCanvasMousedown = (e: MouseEvent, canvasId: string, el: HTMLCanvasElement) => {
  if (!canvasContexts.has(canvasId)) registerCanvas(canvasId, el);
  canvasDrawing.set(canvasId, true);
  const rect = el.getBoundingClientRect();
  canvasCurrentPath.set(canvasId, [[e.clientX - rect.left, e.clientY - rect.top]]);
};

const onCanvasMousemove = (e: MouseEvent, canvasId: string, el: HTMLCanvasElement) => {
  if (!canvasDrawing.get(canvasId)) return;
  const rect = el.getBoundingClientRect();
  const pt: [number, number] = [e.clientX - rect.left, e.clientY - rect.top];
  const path = canvasCurrentPath.get(canvasId) || [];
  path.push(pt);
  canvasCurrentPath.set(canvasId, path);

  // Draw locally immediately
  const ctx = canvasContexts.get(canvasId);
  if (ctx && path.length >= 2) {
    ctx.beginPath();
    ctx.moveTo(path[path.length-2][0], path[path.length-2][1]);
    ctx.lineTo(pt[0], pt[1]);
    ctx.strokeStyle = myColor.value;
    ctx.lineWidth = canvasBrushSize.value[canvasId] || 4;
    ctx.lineCap = 'round'; ctx.lineJoin = 'round';
    ctx.stroke();
  }
};

const onCanvasMouseup = (canvasId: string) => {
  if (!canvasDrawing.get(canvasId)) return;
  canvasDrawing.set(canvasId, false);
  const path = canvasCurrentPath.get(canvasId) || [];
  if (path.length < 2) return;
  const stroke: CanvasStroke = {
    canvas_id: canvasId, user_id: myId.value, color: myColor.value,
    size: canvasBrushSize.value[canvasId] || 4, points: path,
  };
  ws.value?.send(JSON.stringify({ type: 'CanvasDraw', stroke }));
  canvasCurrentPath.delete(canvasId);
};

// ─── File Manager ─────────────────────────────────────────────────────────────
const initFileState = (winId: string, startPath: string) => {
  if (fileStates.value[winId]) return;
  fileStates.value[winId] = { path: startPath, entries: [], loading: false, selected: null, preview: null, viewMode: 'grid' };
  loadFiles(winId, startPath);
};

const loadFiles = async (winId: string, path: string) => {
  const state = fileStates.value[winId];
  if (!state) return;
  state.loading = true;
  state.selected = null;
  state.preview = null;
  try {
    const res = await fetch(`/api/files?path=${encodeURIComponent(path)}`);
    if (!res.ok) throw new Error(await res.text());
    state.entries = await res.json();
    state.path = path;
  } catch (err) {
    console.error('loadFiles error:', err);
  } finally {
    state.loading = false;
  }
};

const openFileEntry = async (winId: string, entry: FileEntry) => {
  const state = fileStates.value[winId];
  if (!state) return;
  if (entry.is_dir) {
    loadFiles(winId, entry.path);
    return;
  }
  state.selected = entry;
  state.preview = null;
  const previewableMime = /^(text\/|application\/json|application\/xml|image\/|video\/|application\/pdf)/;
  if (!previewableMime.test(entry.mime)) { state.preview = '__no_preview__'; return; }
  if (entry.mime.startsWith('text/') || entry.mime === 'application/json' || entry.mime === 'application/xml') {
    try {
      const res = await fetch(`/api/files/read?path=${encodeURIComponent(entry.path)}`);
      state.preview = res.ok ? await res.text() : `Error: ${await res.text()}`;
    } catch (e) { state.preview = String(e); }
  } else {
    // Let browser render it via object tag
    state.preview = `__embed__:${entry.path}`;
  }
};

const fileSizeHuman = (bytes: number) => {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024*1024) return `${(bytes/1024).toFixed(1)} KB`;
  return `${(bytes/(1024*1024)).toFixed(1)} MB`;
};

const fileIcon = (entry: FileEntry) => {
  if (entry.is_dir) return '📁';
  const m = entry.mime;
  if (m.startsWith('image/')) return '🖼️';
  if (m.startsWith('video/')) return '🎬';
  if (m.startsWith('audio/')) return '🎵';
  if (m === 'application/pdf') return '📄';
  if (m.includes('zip') || m.includes('tar') || m.includes('gzip')) return '🗜️';
  if (m.startsWith('text/')) return '📝';
  return '📎';
};

// ─── Window management ────────────────────────────────────────────────────────
const cleanupWindow = (id: string) => {
  terminals.get(id)?.term.dispose();
  terminals.delete(id);
  delete fileStates.value[id];
  canvasContexts.delete(id);
  canvasDrawing.delete(id);
};

const focusWindow = (id: string) => {
  const win = windows.value[id];
  if (!win) return;
  win.z = nextZ();
  activeAppName.value = appDisplayNames[win.app] || win.app;
  broadcastWin(win);
  nextTick(() => terminals.get(id)?.term.focus());
};

const closeWindow = (id: string) => ws.value?.send(JSON.stringify({ type: 'CloseWindow', id }));

const toggleMaximize = (win: AppWindow) => {
  if (win.maximized) {
    Object.assign(win, { maximized: false, x: win._px!, y: win._py!, w: win._pw!, h: win._ph! });
  } else {
    Object.assign(win, { _px: win.x, _py: win.y, _pw: win.w, _ph: win.h, maximized: true, x: 0, y: 28, w: window.innerWidth, h: window.innerHeight - 28 - 76 });
  }
  broadcastWin(win);
  nextTick(() => {
    const t = terminals.get(win.id);
    if (t) { t.fitAddon.fit(); sendResize(win.id, t); }
  });
};

const spawnWindow = (app: string, extra: Partial<AppWindow> = {}) => {
  const count = Object.values(windows.value).filter(w => w.workspace === currentWorkspace.value).length;
  const id = Math.random().toString(36).substring(7);
  const defaults: AppWindow = {
    id, app, workspace: currentWorkspace.value,
    x: 80 + (count * 28) % 200, y: 56 + (count * 28) % 150,
    w: app === 'messages' ? 480 : app === 'canvas' ? 660 : 700,
    h: app === 'messages' ? 520 : 460,
    z: nextZ(), minimized: false, maximized: false, title: '',
    ...extra,
  };
  ws.value?.send(JSON.stringify({ type: 'SpawnWindow', window: defaults }));
  return id;
};

const spawnFiles = () => spawnWindow('files', { title: 'Finder', w: 760, h: 480 });
const spawnTerminal = () => spawnWindow('terminal', { title: 'Terminal' });
const spawnMessages = (channel = 'global') => spawnWindow('messages', { title: `Messages — #${channel}`, channel, w: 480, h: 520 });
const spawnCanvas = () => { const canvasId = Math.random().toString(36).substring(7); spawnWindow('canvas', { title: 'Canvas', canvasId, w: 700, h: 500 }); };

// ─── Broadcast ────────────────────────────────────────────────────────────────
const broadcastWin = (win: AppWindow) => ws.value?.send(JSON.stringify({ type: 'UpdateWindow', window: win }));

const sendResize = (id: string, t: { term: Terminal }) => {
  ws.value?.send(JSON.stringify({ type: 'PtyResize', id, rows: t.term.rows, cols: t.term.cols }));
};

// ─── Drag & Resize ────────────────────────────────────────────────────────────
let dragState = { isDragging: false, win: null as AppWindow | null, startX: 0, startY: 0, ox: 0, oy: 0 };
let resizeState = { isResizing: false, win: null as AppWindow | null, startX: 0, startY: 0, sw: 0, sh: 0, edge: '' };

const startDrag = (e: MouseEvent, win: AppWindow) => {
  if (win.maximized) return;
  dragState = { isDragging: true, win, startX: e.clientX, startY: e.clientY, ox: win.x, oy: win.y };
  focusWindow(win.id);
};

const startResize = (e: MouseEvent, win: AppWindow, edge: string) => {
  if (win.maximized) return;
  e.preventDefault(); e.stopPropagation();
  resizeState = { isResizing: true, win, startX: e.clientX, startY: e.clientY, sw: win.w, sh: win.h, edge };
  focusWindow(win.id);
};

let lastCursor = 0;
const onMouseMove = (e: MouseEvent) => {
  // Throttled cursor emit
  const now = Date.now();
  if (now - lastCursor > 40) {
    lastCursor = now;
    if (!showNamePrompt.value && ws.value?.readyState === WebSocket.OPEN) {
      ws.value.send(JSON.stringify({ type: 'CursorMove', id: myId.value, x: e.clientX, y: e.clientY, workspace: currentWorkspace.value }));
    }
  }

  if (dragState.isDragging && dragState.win) {
    dragState.win.x = dragState.ox + (e.clientX - dragState.startX);
    dragState.win.y = Math.max(28, dragState.oy + (e.clientY - dragState.startY));
    broadcastWin(dragState.win);
  }

  if (resizeState.isResizing && resizeState.win) {
    const w = resizeState.win;
    if (resizeState.edge.includes('e')) w.w = Math.max(280, resizeState.sw + (e.clientX - resizeState.startX));
    if (resizeState.edge.includes('s')) w.h = Math.max(180, resizeState.sh + (e.clientY - resizeState.startY));
    const t = terminals.get(w.id);
    if (t) t.fitAddon.fit();
    broadcastWin(w);
  }
};

const onMouseUp = () => {
  if (resizeState.isResizing && resizeState.win) {
    const t = terminals.get(resizeState.win.id);
    if (t) { t.fitAddon.fit(); sendResize(resizeState.win.id, t); }
  }
  dragState.isDragging = false;
  resizeState.isResizing = false;
};

const onKeyDown = (e: KeyboardEvent) => {
  if ((e.metaKey || e.ctrlKey) && e.key === 't') { e.preventDefault(); spawnTerminal(); }
  if ((e.metaKey || e.ctrlKey) && e.key === 'f') { e.preventDefault(); spawnFiles(); }
  if ((e.metaKey || e.ctrlKey) && e.key === 'm') { e.preventDefault(); spawnMessages(); }
};

const sendChat = (channel: string) => {
  const content = chatInputs.value[channel]?.trim();
  if (!content) return;
  ws.value?.send(JSON.stringify({ type: 'SendChat', channel, content, user_name: myName.value }));
  chatInputs.value[channel] = '';
};

const createChannel = () => {
  const name = newChannelName.value.trim();
  if (!name) return;
  ws.value?.send(JSON.stringify({ type: 'CreateChannel', name, created_by: myName.value }));
  newChannelName.value = '';
};

// ─── Auto-init windows when state changes ────────────────────────────────────
watch(windows, (newWins) => {
  nextTick(() => {
    for (const [id, win] of Object.entries(newWins)) {
      if (win.workspace !== currentWorkspace.value) continue;
      if (win.app === 'terminal') checkTerminals();
      if (win.app === 'files' && !fileStates.value[id]) {
        initFileState(id, '/home');
      }
      if (win.app === 'canvas' && win.canvasId) {
        const el = document.getElementById(`canvas-${win.canvasId}`) as HTMLCanvasElement;
        if (el && !canvasContexts.has(win.canvasId)) {
          el.width = win.w;
          el.height = win.h - 80;
          registerCanvas(win.canvasId, el);
        }
      }
    }
  });
}, { deep: true });

// ─── Lifecycle ────────────────────────────────────────────────────────────────
onMounted(() => {
  updateClock();
  setInterval(updateClock, 10000);
  window.addEventListener('mousemove', onMouseMove);
  window.addEventListener('mouseup', onMouseUp);
  window.addEventListener('keydown', onKeyDown);
  window.addEventListener('resize', () => {
    Object.values(windows.value).filter(w => w.maximized).forEach(w => {
      w.w = window.innerWidth; w.h = window.innerHeight - 28 - 76;
    });
    terminals.forEach((t, id) => { t.fitAddon.fit(); sendResize(id, t); });
  });
});


onUnmounted(() => {
  window.removeEventListener('mousemove', onMouseMove);
  window.removeEventListener('mouseup', onMouseUp);
  window.removeEventListener('keydown', onKeyDown);
  ws.value?.close();
});

const visibleWindows = computed(() =>
  Object.values(windows.value).filter(w => w.workspace === currentWorkspace.value && !w.minimized)
    .sort((a, b) => a.z - b.z)
);
</script>

<template>
  <!-- Name Prompt Modal -->
  <div v-if="showNamePrompt" class="fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center z-99999">
    <div class="bg-[rgba(30,30,30,0.95)] border border-white/15 rounded-2xl p-8 w-96 shadow-2xl" style="animation: winEnter 250ms cubic-bezier(0.23,1,0.32,1) both;">
      <h2 class="text-white font-semibold text-lg mb-1">Welcome</h2>
      <p class="text-[#a1a1a6] text-sm mb-6">Enter a display name to join the shared workspace.</p>
      <input v-model="myName" @keyup.enter="join" placeholder="Your name" autofocus
        class="w-full bg-black/30 border border-white/10 rounded-xl px-4 py-3 text-white text-sm outline-none mb-4 focus:border-[#0a84ff] transition-[border-color] duration-150" />
      <button @click="join"
        class="w-full bg-[#0a84ff] hover:bg-[#006ee6] active:scale-[0.97] text-white font-medium py-3 rounded-xl transition-[background-color,transform] duration-150">
        Join Session
      </button>
    </div>
  </div>

  <div class="h-screen w-screen overflow-hidden relative select-none" style="background: url('https://images.unsplash.com/photo-1618005182384-a83a8bd57fbe?q=80&w=2564&auto=format&fit=crop') center/cover fixed;">
    <!-- ─── Menu Bar ─────────────────────────────────────────────────────── -->
    <div class="fixed top-0 left-0 right-0 h-7 z-9999 flex justify-between items-center px-4 text-[13px] font-medium tracking-[0.3px] text-white border-b border-white/10"
         style="background: rgba(30,30,30,0.4); backdrop-filter: blur(20px) saturate(120%); -webkit-backdrop-filter: blur(20px) saturate(120%);">
      <div class="flex items-center gap-4 h-full">
        <span class="flex items-center h-full px-1.5 rounded hover:bg-white/10 cursor-default transition-[background] duration-100">
          <svg viewBox="0 0 24 24" class="w-3.5 h-3.5 fill-current"><use href="#icon-apple"/></svg>
        </span>
        <span class="font-semibold cursor-default">{{ activeAppName }}</span>
        <span v-for="item in ['File','Edit','View','Window','Help']" :key="item" class="cursor-default px-1.5 h-full flex items-center rounded hover:bg-white/10 transition-[background] duration-100">{{ item }}</span>
      </div>
      <div class="flex items-center gap-4 h-full">
        <!-- Workspace dots -->
        <div class="flex gap-1.5 items-center">
          <div v-for="i in workspaceCount" :key="i"
               @click="currentWorkspace = i - 1"
               :class="['w-2 h-2 rounded-full cursor-pointer transition-[background-color,transform] duration-200', currentWorkspace === i-1 ? 'bg-white scale-110' : 'bg-white/30 hover:bg-white/60 hover:scale-125']"></div>
          <button v-if="workspaceCount < 10" @click="() => { workspaceCount++; ws?.send(JSON.stringify({type:'SetWorkspaceCount',count:workspaceCount})) }" class="text-white/40 hover:text-white text-xs ml-1 transition-colors duration-150">+</button>
        </div>
        <span class="cursor-default px-1.5 h-full flex items-center hover:bg-white/10 rounded transition-[background] duration-100">{{ clock }}</span>
        <span class="text-xs text-white/60 cursor-default">{{ myName }}</span>
      </div>
    </div>

    <!-- ─── Workspace Area ──────────────────────────────────────────────────── -->
    <div class="absolute inset-0 top-7 bottom-20 overflow-hidden pointer-events-none">
      <!-- Remote cursors -->
      <div v-for="u in Object.values(users).filter(u => u.id !== myId && u.workspace === currentWorkspace)"
           :key="u.id"
           class="absolute pointer-events-none transition-[left,top] duration-75"
           :style="{ left: u.x + 'px', top: u.y + 'px' }">
        <svg class="w-4 h-4 drop-shadow-lg" style="transform: rotate(-15deg) translate(-2px,-2px);" viewBox="0 0 24 24">
          <path :fill="u.color" d="M4,0 L4,20 L8,16 L12,24 L14,23 L10,15 L16,15 Z"/>
        </svg>
        <div class="px-2 py-0.5 text-white text-[11px] font-medium rounded ml-3 whitespace-nowrap shadow-lg" :style="{ background: u.color }">{{ u.name }}</div>
      </div>

      <!-- Windows -->
      <div v-for="win in visibleWindows" :key="win.id"
           class="absolute flex flex-col overflow-hidden pointer-events-auto"
           :style="{
             left: win.x+'px', top: win.y+'px', width: win.w+'px', height: win.h+'px', zIndex: win.z,
             background: 'rgba(30,30,30,0.75)',
             backdropFilter: 'blur(40px) saturate(150%)',
             WebkitBackdropFilter: 'blur(40px) saturate(150%)',
             borderRadius: win.maximized ? '0' : '12px',
             border: '1px solid rgba(255,255,255,0.15)',
             boxShadow: '0 25px 60px rgba(0,0,0,0.5), inset 0 1px 1px rgba(255,255,255,0.2)',
           }"
           style="will-change: transform; animation: winEnter 250ms cubic-bezier(0.23,1,0.32,1) both;"
           @mousedown="focusWindow(win.id)">

        <!-- Title Bar -->
        <div class="h-[38px] flex items-center px-3.5 border-b border-white/5 relative shrink-0 cursor-default"
             @mousedown="startDrag($event, win)">
          <div class="flex gap-2 z-10" @mousedown.stop>
            <button @click="closeWindow(win.id)"
              class="w-3 h-3 rounded-full bg-[#ff5f56] border border-[#e0443e] flex items-center justify-center active:scale-90 transition-transform duration-150">
              <svg class="w-2 h-2 opacity-0 hover:opacity-100 transition-opacity duration-150" viewBox="0 0 24 24" fill="none" stroke="rgba(0,0,0,0.6)" stroke-width="2.5" stroke-linecap="round"><path d="M18 6L6 18M6 6l12 12"/></svg>
            </button>
            <button @click="win.minimized = true"
              class="w-3 h-3 rounded-full bg-[#ffbd2e] border border-[#dea123] flex items-center justify-center active:scale-90 transition-transform duration-150">
              <svg class="w-2 h-2 opacity-0 hover:opacity-100 transition-opacity duration-150" viewBox="0 0 24 24" fill="none" stroke="rgba(0,0,0,0.6)" stroke-width="2.5" stroke-linecap="round"><path d="M5 12h14"/></svg>
            </button>
            <button @click="toggleMaximize(win)"
              class="w-3 h-3 rounded-full bg-[#27c93f] border border-[#1aab29] flex items-center justify-center active:scale-90 transition-transform duration-150">
              <svg class="w-2 h-2 opacity-0 hover:opacity-100 transition-opacity duration-150" viewBox="0 0 24 24" fill="none" stroke="rgba(0,0,0,0.6)" stroke-width="1.5" stroke-linecap="round"><rect x="3" y="3" width="18" height="18" rx="1"/></svg>
            </button>
          </div>
          <div class="absolute inset-0 flex items-center justify-center pointer-events-none text-[13px] font-semibold text-white truncate px-20">{{ win.title }}</div>
        </div>

        <!-- Content -->
        <div class="flex-1 overflow-hidden relative bg-black/20">

          <!-- ── Terminal ── -->
          <div v-if="win.app === 'terminal'" :id="`term-${win.id}`" class="absolute inset-0 p-1"></div>

          <!-- ── File Manager ── -->
          <div v-if="win.app === 'files'" class="absolute inset-0 flex bg-black/10">
            <!-- Sidebar -->
            <div class="w-44 bg-black/20 border-r border-white/5 flex flex-col py-3 shrink-0">
              <p class="text-[11px] text-white/30 font-semibold uppercase tracking-wider px-4 mb-2">Favorites</p>
              <button v-for="fav in [{name:'Home', path: '/home'}, {name:'Root', path: '/'}, {name:'Tmp', path: '/tmp'}]" :key="fav.path"
                @click="loadFiles(win.id, fav.path)"
                class="flex items-center gap-2 text-[13px] px-4 py-1.5 rounded mx-2 transition-[background] duration-150 hover:bg-white/8 text-white/80">
                <svg viewBox="0 0 24 24" class="w-4 h-4 fill-[#0a84ff]"><use href="#icon-folder"/></svg>{{ fav.name }}
              </button>
              <div class="mt-4 pt-4 border-t border-white/5">
                <p class="text-[11px] text-white/30 font-semibold uppercase tracking-wider px-4 mb-2">Path</p>
                <div class="text-[11px] text-white/40 px-4 break-all leading-relaxed">{{ fileStates[win.id]?.path }}</div>
              </div>
            </div>
            <!-- File content area -->
            <div class="flex-1 flex flex-col overflow-hidden">
              <!-- Toolbar -->
              <div class="h-10 border-b border-white/5 flex items-center px-4 gap-3 shrink-0 bg-black/10">
                <button @click="() => { const s = fileStates[win.id]; if(s) { const parent = s.path.split('/').slice(0,-1).join('/') || '/'; loadFiles(win.id, parent); } }"
                  class="text-white/60 hover:text-white px-2 py-1 rounded hover:bg-white/10 text-xs transition-[background,color] duration-150">← Back</button>
                <div class="text-xs text-white/40 font-mono truncate flex-1">{{ fileStates[win.id]?.path }}</div>
                <div class="flex gap-1">
                  <button @click="() => fileStates[win.id] && (fileStates[win.id].viewMode = 'grid')" :class="['p-1.5 rounded text-xs transition-[background] duration-150', fileStates[win.id]?.viewMode === 'grid' ? 'bg-white/15 text-white' : 'text-white/40 hover:bg-white/8']">⊞</button>
                  <button @click="() => fileStates[win.id] && (fileStates[win.id].viewMode = 'list')" :class="['p-1.5 rounded text-xs transition-[background] duration-150', fileStates[win.id]?.viewMode === 'list' ? 'bg-white/15 text-white' : 'text-white/40 hover:bg-white/8']">☰</button>
                </div>
              </div>
              <!-- Files + Preview split -->
              <div class="flex-1 flex overflow-hidden">
                <div :class="['overflow-y-auto', fileStates[win.id]?.selected ? 'w-1/2' : 'flex-1']">
                  <!-- Grid view -->
                  <div v-if="fileStates[win.id]?.viewMode === 'grid'" class="p-4 grid grid-cols-4 gap-3 content-start">
                    <div v-if="fileStates[win.id]?.loading" class="col-span-4 flex items-center justify-center py-12 text-white/30 text-sm">Loading…</div>
                    <button v-for="entry in fileStates[win.id]?.entries" :key="entry.path"
                      @click="openFileEntry(win.id, entry)" @dblclick="entry.is_dir && loadFiles(win.id, entry.path)"
                      :class="['flex flex-col items-center gap-2 p-3 rounded-xl cursor-pointer transition-[background,transform] duration-150 hover:bg-white/10 active:scale-[0.95] text-center', fileStates[win.id]?.selected?.path === entry.path ? 'bg-white/15 ring-1 ring-[#0a84ff]' : '']">
                      <span class="text-3xl leading-none">{{ fileIcon(entry) }}</span>
                      <span class="text-[11px] text-white/80 w-full truncate">{{ entry.name }}</span>
                    </button>
                  </div>
                  <!-- List view -->
                  <div v-if="fileStates[win.id]?.viewMode === 'list'" class="p-2">
                    <div v-if="fileStates[win.id]?.loading" class="flex items-center justify-center py-12 text-white/30 text-sm">Loading…</div>
                    <button v-for="entry in fileStates[win.id]?.entries" :key="entry.path"
                      @click="openFileEntry(win.id, entry)" @dblclick="entry.is_dir && loadFiles(win.id, entry.path)"
                      :class="['w-full flex items-center gap-3 px-3 py-2 rounded-lg cursor-pointer transition-[background] duration-150 hover:bg-white/8 text-left', fileStates[win.id]?.selected?.path === entry.path ? 'bg-white/15' : '']">
                      <span class="text-xl leading-none shrink-0">{{ fileIcon(entry) }}</span>
                      <span class="flex-1 text-[13px] text-white/85 truncate">{{ entry.name }}</span>
                      <span class="text-[11px] text-white/30 shrink-0">{{ entry.is_dir ? '—' : fileSizeHuman(entry.size) }}</span>
                    </button>
                  </div>
                </div>
                <!-- Preview panel -->
                <div v-if="fileStates[win.id]?.selected" class="w-1/2 border-l border-white/5 flex flex-col overflow-hidden">
                  <div class="h-9 border-b border-white/5 px-3 flex items-center justify-between shrink-0">
                    <span class="text-[12px] text-white/50 truncate">{{ fileStates[win.id]?.selected?.name }}</span>
                    <a v-if="fileStates[win.id]?.selected" :href="`/api/files/download?path=${encodeURIComponent(fileStates[win.id]?.selected?.path || '')}`"
                      class="text-[11px] text-[#0a84ff] hover:text-[#006ee6] transition-colors duration-150 shrink-0 ml-2">⬇ Download</a>
                  </div>
                  <div class="flex-1 overflow-auto">
                    <div v-if="fileStates[win.id]?.preview === '__no_preview__'" class="flex flex-col items-center justify-center h-full gap-3 text-white/25">
                      <span class="text-4xl">{{ fileIcon(fileStates[win.id]!.selected!) }}</span>
                      <span class="text-sm">No preview available</span>
                    </div>
                    <div v-else-if="fileStates[win.id]?.preview?.startsWith('__embed__:')" class="h-full">
                      <object :data="`/api/files/download?path=${encodeURIComponent(fileStates[win.id]!.preview!.slice(10))}`" class="w-full h-full"></object>
                    </div>
                    <pre v-else-if="fileStates[win.id]?.preview" class="text-[12px] font-mono text-green-400/80 p-4 whitespace-pre-wrap wrap-break-word overflow-auto h-full leading-relaxed">{{ fileStates[win.id]?.preview }}</pre>
                    <div v-else class="flex items-center justify-center h-full text-white/20 text-sm">Select a file to preview</div>
                  </div>
                </div>
              </div>
            </div>
          </div>

          <!-- ── Messages ── -->
          <div v-if="win.app === 'messages'" class="absolute inset-0 flex bg-black/10">
            <!-- Channel sidebar -->
            <div class="w-52 bg-black/20 border-r border-white/5 flex flex-col">
              <div class="p-3 border-b border-white/5">
                <p class="text-[11px] text-white/30 font-semibold uppercase tracking-wider mb-2">Channels</p>
                <button v-for="ch in channels" :key="ch.id"
                  @click="() => { win.channel = ch.id; win.title = `Messages — ${ch.name}`; broadcastWin(win); }"
                  :class="['w-full text-left px-3 py-1.5 rounded-lg text-[13px] transition-[background] duration-150', win.channel === ch.id ? 'bg-[#0a84ff] text-white' : 'text-white/70 hover:bg-white/8']">
                  {{ ch.name }}
                </button>
              </div>
              <!-- New Channel -->
              <div class="p-3 mt-auto">
                <div class="flex gap-2">
                  <input v-model="newChannelName" @keyup.enter="createChannel" placeholder="New channel…"
                    class="flex-1 bg-black/30 border border-white/10 rounded-lg px-2 py-1.5 text-[12px] text-white outline-none focus:border-[#0a84ff] transition-[border-color] duration-150" />
                  <button @click="createChannel" class="text-[#0a84ff] px-2 py-1.5 rounded-lg hover:bg-white/10 text-sm transition-[background] duration-150">+</button>
                </div>
              </div>
              <!-- Online users -->
              <div class="border-t border-white/5 p-3">
                <p class="text-[11px] text-white/30 font-semibold uppercase tracking-wider mb-2">Online</p>
                <div v-for="u in Object.values(users)" :key="u.id" class="flex items-center gap-2 py-1">
                  <div class="w-2 h-2 rounded-full shrink-0" :style="{ background: u.color }"></div>
                  <span class="text-[12px] text-white/70 truncate">{{ u.name }}</span>
                </div>
              </div>
            </div>
            <!-- Chat area -->
            <div class="flex-1 flex flex-col">
              <div class="flex-1 p-5 overflow-y-auto flex flex-col gap-3">
                <template v-for="msg in chats.filter(c => c.channel === win.channel)" :key="msg.id">
                  <div :class="['flex flex-col', msg.user_name === myName ? 'items-end' : 'items-start']" style="animation: popIn 250ms cubic-bezier(0.175,0.885,0.32,1.1) both;">
                    <span class="text-[10px] text-white/30 mb-1">{{ msg.user_name }}</span>
                    <div :class="['max-w-[70%] px-3.5 py-2.5 rounded-2xl text-[14px] leading-relaxed', msg.user_name === myName ? 'bg-[#0a84ff] text-white rounded-br-[4px]' : 'bg-white/10 text-white rounded-bl-[4px]']">{{ msg.content }}</div>
                  </div>
                </template>
              </div>
              <div class="p-3.5 border-t border-white/5 bg-black/20">
                <input :value="chatInputs[win.channel || '']" @input="(e) => chatInputs[win.channel || ''] = (e.target as HTMLInputElement).value"
                  @keyup.enter="sendChat(win.channel || '')"
                  :placeholder="`Message ${channels.find(c=>c.id===win.channel)?.name || '#global'}…`"
                  class="w-full bg-black/30 border border-white/10 rounded-2xl px-4 py-2.5 text-[14px] text-white outline-none focus:border-[#0a84ff] transition-[border-color] duration-150" />
              </div>
            </div>
          </div>

          <!-- ── Canvas ── -->
          <div v-if="win.app === 'canvas'" class="absolute inset-0 flex flex-col bg-white">
            <div class="h-10 bg-[#f0f0f0] border-b border-[#ddd] flex items-center gap-4 px-3 shrink-0">
              <div class="flex items-center gap-2 text-[13px] font-medium text-[#333]">
                <div class="w-5 h-5 rounded-full border-2 border-white shadow" :style="{ background: myColor }"></div>
                <span class="text-[12px] text-gray-500">Your color</span>
              </div>
              <input type="range" min="1" max="20" :value="canvasBrushSize[win.canvasId || ''] || 4"
                @input="(e) => canvasBrushSize[win.canvasId || ''] = Number((e.target as HTMLInputElement).value)"
                class="w-24" />
              <button @click="() => { ws?.send(JSON.stringify({type:'CanvasClear', canvas_id: win.canvasId})) }"
                class="px-3 py-1.5 rounded-lg border border-[#ccc] bg-white text-[13px] font-medium hover:bg-[#f0f0f0] active:scale-[0.95] transition-[background,transform] duration-150">
                Clear
              </button>
              <!-- Other user colors legend -->
              <div class="flex items-center gap-2 ml-auto text-[11px] text-gray-400">
                <div v-for="u in Object.values(users).filter(u => u.id !== myId)" :key="u.id" class="flex items-center gap-1">
                  <div class="w-3 h-3 rounded-full" :style="{ background: u.color }"></div>
                  <span>{{ u.name }}</span>
                </div>
              </div>
            </div>
            <canvas :id="`canvas-${win.canvasId}`" class="flex-1 cursor-crosshair touch-none"
              @mousedown="(e) => { const el = e.target as HTMLCanvasElement; if(win.canvasId) onCanvasMousedown(e, win.canvasId, el); }"
              @mousemove="(e) => { const el = e.target as HTMLCanvasElement; if(win.canvasId) onCanvasMousemove(e, win.canvasId, el); }"
              @mouseup="() => win.canvasId && onCanvasMouseup(win.canvasId)"
              @mouseleave="() => win.canvasId && onCanvasMouseup(win.canvasId)">
            </canvas>
          </div>
        </div>

        <!-- Resize handles -->
        <div class="absolute right-0 bottom-0 w-4 h-4 cursor-se-resize z-50" @mousedown.stop="startResize($event, win, 'se')"></div>
        <div class="absolute right-0 top-10 bottom-4 w-1 cursor-e-resize z-50" @mousedown.stop="startResize($event, win, 'e')"></div>
        <div class="absolute bottom-0 left-0 right-4 h-1 cursor-s-resize z-50" @mousedown.stop="startResize($event, win, 's')"></div>
      </div>
    </div>

    <!-- ─── Dock ───────────────────────────────────────────────────────────── -->
    <div class="fixed bottom-3 left-0 right-0 flex justify-center z-9998 pointer-events-none">
      <div class="pointer-events-auto flex items-end gap-2 px-1.5 py-1.5 rounded-3xl border border-white/15"
           style="background: rgba(20,20,20,0.5); backdrop-filter: blur(40px) saturate(150%); -webkit-backdrop-filter: blur(40px) saturate(150%); box-shadow: 0 10px 40px rgba(0,0,0,0.4), inset 0 1px 1px rgba(255,255,255,0.2);">
        <button v-for="item in [
          { id: 'files', name: 'Finder', bg: '#007aff', icon: 'icon-folder', action: spawnFiles },
          { id: 'terminal', name: 'Terminal', bg: '#1e1e1e', icon: 'icon-terminal', action: spawnTerminal },
          { id: 'messages', name: 'Messages', bg: '#34c759', icon: 'icon-messages', action: () => spawnMessages() },
          { id: 'canvas', name: 'Canvas', bg: '#af52de', icon: 'icon-canvas', action: spawnCanvas },
        ]" :key="item.id"
          @click="item.action()"
          :data-name="item.name"
          class="relative w-12 h-12 rounded-[12px] flex items-center justify-center cursor-pointer transition-[transform,margin] duration-200 hover:scale-125 hover:-translate-y-1.5 hover:mx-3 active:scale-110 active:translate-y-0 origin-bottom"
          :style="{ background: item.bg, boxShadow: 'inset 0 1px 1px rgba(255,255,255,0.2), 0 4px 12px rgba(0,0,0,0.3)' }"
          :title="item.name">
          <svg viewBox="0 0 24 24" class="w-7 h-7 fill-white drop-shadow-[0_2px_4px_rgba(0,0,0,0.2)]"><use :href="`#${item.icon}`"/></svg>
          <!-- Active indicator -->
          <div v-if="Object.values(windows).some(w => w.app === item.id)" class="absolute -bottom-1.5 w-1 h-1 bg-white/80 rounded-full"></div>
        </button>
      </div>
    </div>
  </div>

  <!-- SVG Defs (inline, no external requests) -->
  <svg style="display:none">
    <defs>
      <path id="icon-apple" d="M15.2 10.5c-.1-2.5 2-3.7 2.1-3.8-1.2-1.7-3-1.9-3.6-2-.8-.1-2.1.5-2.8.5-.8 0-1.8-.5-2.7-.5-1.2 0-2.4.7-3 1.8-1.3 2.3-.3 5.7 1 7.6.6.9 1.3 1.9 2.3 1.9.9 0 1.3-.6 2.4-.6 1.1 0 1.4.6 2.4.6 1 0 1.6-1 2.2-1.9.8-1.1 1.1-2.2 1.1-2.2-.1-.1-1.4-.5-1.4-2.4zM11.6 3.4c.5-.6.8-1.4.7-2.2-.7 0-1.6.3-2.1.9-.4.5-.8 1.3-.7 2.1.8.1 1.6-.2 2.1-.8z"/>
      <path id="icon-folder" d="M10 4H4c-1.1 0-1.99.9-1.99 2L2 18c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V8c0-1.1-.9-2-2-2h-8l-2-2z"/>
      <path id="icon-terminal" d="M20 4H4c-1.1 0-2 .9-2 2v12c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V6c0-1.1-.9-2-2-2zm0 14H4V8h16v10zm-2-1h-6v-2h6v2zM7.5 17l-1.41-1.41L10.67 11l-4.58-4.59L7.5 5l6 6-6 6z"/>
      <path id="icon-messages" d="M20 2H4c-1.1 0-2 .9-2 2v18l4-4h14c1.1 0 2-.9 2-2V4c0-1.1-.9-2-2-2z"/>
      <path id="icon-canvas" d="M12 3c-4.97 0-9 4.03-9 9s4.03 9 9 9c.83 0 1.5-.67 1.5-1.5 0-.39-.15-.74-.39-1.01-.23-.26-.38-.61-.38-.99 0-.83.67-1.5 1.5-1.5H16c2.76 0 5-2.24 5-5 0-4.42-4.03-8-9-8zm-5.5 9c-.83 0-1.5-.67-1.5-1.5S5.67 9 6.5 9 8 9.67 8 10.5 7.33 12 6.5 12zm3-4C8.67 8 8 7.33 8 6.5S8.67 5 9.5 5s1.5.67 1.5 1.5S10.33 8 9.5 8zm5 0c-.83 0-1.5-.67-1.5-1.5S13.67 5 14.5 5s1.5.67 1.5 1.5S15.33 8 14.5 8zm3 4c-.83 0-1.5-.67-1.5-1.5S16.67 9 17.5 9s1.5.67 1.5 1.5-.67 1.5-1.5 1.5z"/>
    </defs>
  </svg>
</template>

<style>
@font-face {
  font-family: 'JetBrains Mono Nerd Font';
  src: url('/fonts/JetBrainsMonoNerdFont.ttf') format('truetype');
  font-weight: normal; font-style: normal;
}

:root {
  --ease-out: cubic-bezier(0.23, 1, 0.32, 1);
  --ease-in-out: cubic-bezier(0.77, 0, 0.175, 1);
  --ease-spring: cubic-bezier(0.175, 0.885, 0.32, 1.1);
}

*, *::before, *::after { box-sizing: border-box; }
body { margin: 0; overflow: hidden; font-family: -apple-system, BlinkMacSystemFont, "SF Pro Display", "Segoe UI", sans-serif; -webkit-font-smoothing: antialiased; }

@keyframes winEnter {
  from { opacity: 0; transform: scale(0.95); }
  to   { opacity: 1; transform: scale(1); }
}
@keyframes popIn {
  from { opacity: 0; transform: scale(0.9); }
  to   { opacity: 1; transform: scale(1); }
}

::-webkit-scrollbar { width: 8px; height: 8px; }
::-webkit-scrollbar-track { background: transparent; }
::-webkit-scrollbar-thumb { background: rgba(255,255,255,0.2); border-radius: 4px; }
::-webkit-scrollbar-thumb:hover { background: rgba(255,255,255,0.3); }

.xterm-viewport::-webkit-scrollbar-thumb { background: rgba(255,255,255,0.15); }
.xterm-screen { opacity: 0.98; }
</style>
