<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted, nextTick, computed } from 'vue';
import { Terminal } from 'xterm';
import { FitAddon } from 'xterm-addon-fit';
import 'xterm/css/xterm.css';
import { Terminal as TerminalIcon, MessageCircle, X, Navigation, MousePointer2 } from 'lucide-vue-next';

interface AppWindow {
  id: string;
  app: string;
  workspace: number;
  x: number;
  y: number;
  w: number;
  h: number;
  z: number;
  minimized: boolean;
  maximized: boolean;
  title: string;
  channel?: string;
}

interface User {
  id: string;
  name: string;
  x: number;
  y: number;
  workspace: number;
}

interface ChatMessage {
  id: string;
  channel: string;
  user_name: string;
  content: string;
  timestamp: number;
}

const myId = ref(Math.random().toString(36).substring(7));
const myName = ref('');
const showNamePrompt = ref(true);

const currentWorkspace = ref(1);
const workspaceCount = ref(4);

const users = ref<Record<string, User>>({});
const windows = ref<Record<string, AppWindow>>({});
const chats = ref<ChatMessage[]>([]);

const ws = ref<WebSocket | null>(null);

const terminals = new Map<string, { term: Terminal, fitAddon: FitAddon }>();

const join = () => {
  if (!myName.value.trim()) myName.value = `user_${myId.value.substring(0, 4)}`;
  showNamePrompt.value = false;
  connectWs();
};

const connectWs = () => {
  const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
  ws.value = new WebSocket(`${protocol}//${window.location.host}/_ws`);
  
  ws.value.onopen = () => {
    ws.value?.send(JSON.stringify({ type: 'UserJoined', user: { id: myId.value, name: myName.value, x: 0, y: 0, workspace: currentWorkspace.value } }));
  };

  ws.value.onmessage = (event) => {
    const data = JSON.parse(event.data);
    
    if (data.type === 'SyncState') {
      const state = data.state;
      users.value = state.users;
      windows.value = state.windows;
      chats.value = state.chats;
      workspaceCount.value = state.workspace_count;
      checkTerminals();
    } else if (data.type === 'UserJoined') {
      users.value[data.user.id] = data.user;
    } else if (data.type === 'UserLeft') {
      delete users.value[data.id];
    } else if (data.type === 'CursorMove') {
      if (data.id !== myId.value && users.value[data.id]) {
        users.value[data.id].x = data.x;
        users.value[data.id].y = data.y;
        users.value[data.id].workspace = data.workspace;
      }
    } else if (data.type === 'SpawnWindow') {
      windows.value[data.window.id] = data.window;
      checkTerminals();
    } else if (data.type === 'UpdateWindow') {
      if (!resizeState.isResizing && !dragState.isDragging || data.window.id !== dragState.win?.id) {
        windows.value[data.window.id] = data.window;
      }
    } else if (data.type === 'CloseWindow') {
      delete windows.value[data.id];
      checkTerminals();
    } else if (data.type === 'PtyOut') {
      const t = terminals.get(data.id);
      if (t) t.term.write(data.data);
    } else if (data.type === 'HistoryData') {
      const t = terminals.get(data.id);
      if (t) t.term.write(data.data);
    } else if (data.type === 'ChatMsg') {
      chats.value.push(data.msg);
    } else if (data.type === 'UpdateTitle') {
      if (windows.value[data.id]) {
         windows.value[data.id].title = data.title;
      }
    } else if (data.type === 'SetWorkspaceCount') {
      workspaceCount.value = data.count;
    }
  };
};

const checkTerminals = () => {
  for (const id of terminals.keys()) {
    if (!windows.value[id]) {
      terminals.get(id)?.term.dispose();
      terminals.delete(id);
    }
  }
  
  for (const [id, win] of Object.entries(windows.value)) {
    if (win.app === 'terminal' && !terminals.has(id) && win.workspace === currentWorkspace.value) {
      nextTick(() => {
        const el = document.getElementById(`term-${id}`);
        if (el) {
          const term = new Terminal({
            cursorBlink: true,
            fontSize: 14,
            fontFamily: '"JetBrains Mono Nerd Font", "JetBrains Mono", monospace',
            theme: { background: '#0c0c0c', foreground: '#d4d4d4' }
          });
          const fitAddon = new FitAddon();
          term.loadAddon(fitAddon);
          term.open(el);
          fitAddon.fit();
          terminals.set(id, { term, fitAddon });
          
          ws.value?.send(JSON.stringify({ type: 'RequestHistory', id }));
          
          term.onData((data) => {
            ws.value?.send(JSON.stringify({ type: 'PtyIn', id, data }));
          });
        }
      });
    }
  }
};

watch(currentWorkspace, () => {
  checkTerminals();
  ws.value?.send(JSON.stringify({ type: 'CursorMove', id: myId.value, x: 0, y: 0, workspace: currentWorkspace.value }));
});

let lastCursorEmit = 0;
const onMouseMove = (e: MouseEvent) => {
  const now = Date.now();
  if (now - lastCursorEmit > 40) { // Throttle cursor emit
    lastCursorEmit = now;
    if (ws.value?.readyState === WebSocket.OPEN && !showNamePrompt.value) {
      ws.value.send(JSON.stringify({ type: 'CursorMove', id: myId.value, x: e.clientX, y: e.clientY, workspace: currentWorkspace.value }));
    }
  }
  
  if (dragState.isDragging && dragState.win) {
    dragState.win.x = dragState.startLeft + (e.clientX - dragState.startX);
    dragState.win.y = Math.max(0, dragState.startTop + (e.clientY - dragState.startY));
    broadcastWinUpdate(dragState.win);
  }
  if (resizeState.isResizing && resizeState.win) {
    const w = resizeState.win;
    if (resizeState.edge.includes('e')) w.w = Math.max(250, resizeState.startW + (e.clientX - resizeState.startX));
    if (resizeState.edge.includes('s')) w.h = Math.max(150, resizeState.startH + (e.clientY - resizeState.startY));
    
    const t = terminals.get(w.id);
    if (t) t.fitAddon.fit();
    broadcastWinUpdate(w);
  }
};

let dragState = { isDragging: false, win: null as AppWindow | null, startX: 0, startY: 0, startLeft: 0, startTop: 0 };
let resizeState = { isResizing: false, win: null as AppWindow | null, startX: 0, startY: 0, startW: 0, startH: 0, edge: 'se' };

const startDrag = (e: MouseEvent, win: AppWindow) => {
  dragState = { isDragging: true, win, startX: e.clientX, startY: e.clientY, startLeft: win.x, startTop: win.y };
  win.z = Date.now() % 10000;
  broadcastWinUpdate(win);
};

const startResize = (e: MouseEvent, win: AppWindow, edge: string) => {
  e.preventDefault();
  resizeState = { isResizing: true, win, startX: e.clientX, startY: e.clientY, startW: win.w, startH: win.h, edge };
  win.z = Date.now() % 10000;
  broadcastWinUpdate(win);
};

const onMouseUp = () => {
  if (resizeState.isResizing && resizeState.win) {
    const t = terminals.get(resizeState.win.id);
    if (t) {
      t.fitAddon.fit();
      ws.value?.send(JSON.stringify({ type: 'PtyResize', id: resizeState.win.id, rows: t.term.rows, cols: t.term.cols }));
    }
  }
  dragState.isDragging = false;
  resizeState.isResizing = false;
};

const broadcastWinUpdate = (win: AppWindow) => {
  ws.value?.send(JSON.stringify({ type: 'UpdateWindow', window: win }));
};

const spawnTerminal = () => {
  const ObjectVals = Object.values(windows.value).filter(w => w.workspace === currentWorkspace.value);
  const id = Math.random().toString(36).substring(7);
  const win: AppWindow = {
    id, app: 'terminal', workspace: currentWorkspace.value,
    x: 100 + (ObjectVals.length * 40) % 300, 
    y: 100 + (ObjectVals.length * 40) % 300,
    w: 700, h: 450, z: Date.now() % 10000, 
    minimized: false, maximized: false, title: `tty-${id}`
  };
  ws.value?.send(JSON.stringify({ type: 'SpawnWindow', window: win }));
};

const spawnMessages = () => {
  const existing = Object.values(windows.value).find(w => w.app === 'messages' && w.workspace === currentWorkspace.value);
  if (existing) {
     existing.z = Date.now() % 10000;
     broadcastWinUpdate(existing);
     return;
  }

  const id = Math.random().toString(36).substring(7);
  const win: AppWindow = {
    id, app: 'messages', workspace: currentWorkspace.value,
    x: 200, y: 150, w: 400, h: 500, z: Date.now() % 10000, 
    minimized: false, maximized: false, title: `Global Chat`, channel: 'global'
  };
  ws.value?.send(JSON.stringify({ type: 'SpawnWindow', window: win }));
};

const closeWindow = (id: string) => {
  ws.value?.send(JSON.stringify({ type: 'CloseWindow', id }));
};

const chatInputs = ref<Record<string, string>>({});
const sendChat = (channel: string) => {
  const content = chatInputs.value[channel];
  if (!content) return;
  ws.value?.send(JSON.stringify({ type: 'SendChat', channel, content, user_name: myName.value }));
  chatInputs.value[channel] = '';
};

onMounted(() => {
  window.addEventListener('mousemove', onMouseMove);
  window.addEventListener('mouseup', onMouseUp);
});
</script>

<template>
  <div v-if="showNamePrompt" class="fixed inset-0 bg-slate-950 flex flex-col items-center justify-center z-99999">
     <div class="bg-[#1c1c1e] p-8 rounded-2xl shadow-2xl border border-white/10 w-96 max-w-full">
         <h1 class="text-white text-xl font-semibold mb-2">Welcome to WebOS</h1>
         <p class="text-zinc-400 text-sm mb-6">Enter your display name to join the collaborative workspace.</p>
         <input v-model="myName" @keyup.enter="join" placeholder="e.g. Satoshi" class="w-full bg-black/50 border border-white/10 rounded-lg px-4 py-3 text-white focus:outline-none focus:border-cyan-500 transition-colors mb-4" />
         <button @click="join" class="w-full bg-cyan-600 hover:bg-cyan-500 text-white font-medium py-3 rounded-lg transition-colors shadow-lg shadow-cyan-900/20">Join Session</button>
     </div>
  </div>

  <div v-else class="h-screen w-screen overflow-hidden bg-slate-950 font-sans text-gray-200 select-none relative" 
       style="background-image: radial-gradient(circle at 50% 10%, #1a202c 0%, #020617 100%);">
    
    <!-- Cursors overlay -->
    <div v-for="user in Object.values(users).filter(u => u.id !== myId && u.workspace === currentWorkspace)" 
         :key="user.id" 
         class="absolute z-99999 pointer-events-none transition-all duration-75 block"
         :style="{ left: user.x + 'px', top: user.y + 'px' }">
      <MousePointer2 class="w-5 h-5 text-cyan-400 drop-shadow-md" style="transform: rotate(-15deg) translate(-2px, -2px);" />
      <div class="px-2 py-0.5 bg-cyan-600 rounded text-xs text-white shadow ml-3 mt-1 whitespace-nowrap opacity-90">
        {{ user.name }}
      </div>
    </div>

    <!-- Workspaces Top Bar -->
    <div class="absolute top-2 left-1/2 -translate-x-1/2 bg-white/5 backdrop-blur-xl border border-white/10 rounded-full px-4 py-1.5 flex items-center space-x-2 z-99999 pointer-events-auto">
      <div v-for="i in workspaceCount" :key="i"
           @click="currentWorkspace = i"
           :class="['w-10 h-6 flex items-center justify-center rounded-sm cursor-pointer text-xs font-semibold transition-all', currentWorkspace === i ? 'bg-white/20 text-white shadow-sm' : 'hover:bg-white/5 text-gray-400']">
         {{ i }}
      </div>
      <div v-if="workspaceCount < 10" @click="() => { workspaceCount++; ws?.send(JSON.stringify({ type: 'SetWorkspaceCount', count: workspaceCount })) }" class="w-6 h-6 flex items-center justify-center rounded-md cursor-pointer text-gray-400 hover:bg-white/10 text-lg leading-none pb-0.5">
         +
      </div>
    </div>

    <!-- Workspace Layer -->
    <div class="absolute inset-0 overflow-hidden pointer-events-none">
      <div v-for="win in Object.values(windows)" 
           :key="win.id"
           v-show="win.workspace === currentWorkspace"
           :style="{ left: win.x + 'px', top: win.y + 'px', width: win.w + 'px', height: win.h + 'px', zIndex: win.z }"
           class="absolute flex flex-col bg-[#1c1c1e] rounded-xl overflow-hidden shadow-2xl pointer-events-auto border border-[#ffffff15] transition-[shadow] duration-75"
           @mousedown="() => { win.z = Date.now() % 10000; broadcastWinUpdate(win); }"
           :class="{'shadow-[0_12px_45px_rgba(0,0,0,0.8)] border-[#ffffff25]': true}"
      >
          <!-- MacOS Title Bar -->
          <div class="h-10 bg-[#2d2d30] flex items-center px-4 shrink-0 cursor-default" @mousedown="startDrag($event, win)">
             <div class="flex space-x-2 w-20">
                <div @click.stop="closeWindow(win.id)" class="w-3 h-3 rounded-full bg-[#ff5f56] hover:bg-[#ff5f56]/80 cursor-pointer shadow-inner"></div>
                <div class="w-3 h-3 rounded-full bg-[#ffbd2e] shadow-inner opacity-50"></div>
                <div class="w-3 h-3 rounded-full bg-[#27c93f] shadow-inner opacity-50"></div>
             </div>
             <div class="flex-1 text-center text-xs font-semibold text-gray-300 pointer-events-none truncate cursor-default select-none tracking-wide">
                {{ win.title }}
             </div>
             <div class="w-20"></div>
          </div>
          
          <!-- Inner Payload -->
          <div class="flex-1 relative bg-[#060606]">
              <div v-if="win.app === 'terminal'" :id="'term-' + win.id" class="absolute inset-0 p-1"></div>
              
              <div v-if="win.app === 'messages'" class="absolute inset-0 flex flex-col bg-[#111111] overflow-hidden">
                 <div class="flex-1 overflow-y-auto p-4 space-y-4">
                    <div v-for="msg in chats.filter(c => c.channel === win.channel)" :key="msg.id" class="flex flex-col">
                       <span class="text-[10px] text-gray-500 mb-0.5">{{ msg.user_name }}</span>
                       <span class="text-sm text-gray-200 bg-white/5 inline-block w-fit px-3 py-1.5 rounded-r-xl rounded-bl-xl border border-white/5">{{ msg.content }}</span>
                    </div>
                 </div>
                 <div class="h-14 bg-[#1a1a1c] border-t border-white/5 flex items-center px-3 shrink-0">
                    <input v-model="chatInputs[win.channel || '']" @keyup.enter="() => sendChat(win.channel || '')" placeholder="Message..." class="flex-1 bg-black/30 border border-white/10 rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:border-indigo-500 transition-colors" />
                 </div>
              </div>
          </div>
          
          <!-- Invisible Resize Borders -->
          <div class="absolute right-0 bottom-0 w-4 h-4 cursor-se-resize z-50" @mousedown.stop="startResize($event, win, 'se')"></div>
          <div class="absolute right-0 top-0 bottom-4 w-1 cursor-e-resize z-50" @mousedown.stop="startResize($event, win, 'e')"></div>
          <div class="absolute left-0 bottom-0 right-4 h-1 cursor-s-resize z-50" @mousedown.stop="startResize($event, win, 's')"></div>
      </div>
    </div>

    <!-- macOS Dock -->
    <div class="absolute bottom-4 left-1/2 transform -translate-x-1/2 flex items-end space-x-3 bg-white/5 backdrop-blur-xl px-4 py-2.5 rounded-2xl border border-white/10 shadow-2xl z-99999 pointer-events-auto">
      <!-- Terminal App -->
      <div @click="spawnTerminal" class="group flex flex-col items-center cursor-pointer transition-transform hover:-translate-y-2 origin-bottom">
         <div class="w-14 h-14 bg-linear-to-br from-zinc-700 to-zinc-900 rounded-[14px] flex items-center justify-center border border-white/20 shadow-xl relative overflow-hidden">
            <div class="absolute inset-0 bg-black/20"></div>
            <TerminalIcon class="w-7 h-7 text-white z-10" />
            <div class="absolute -top-1 -left-1 w-6 h-6 bg-white/10 rounded-full blur-md"></div>
         </div>
      </div>
      
      <div class="w-px h-12 bg-white/10 mx-1"></div>
      
      <!-- Messages App -->
      <div @click="spawnMessages" class="group flex flex-col items-center cursor-pointer transition-transform hover:-translate-y-2 origin-bottom">
         <div class="w-14 h-14 bg-linear-to-br from-indigo-500 to-blue-700 rounded-[14px] flex items-center justify-center border border-white/20 shadow-xl relative overflow-hidden">
            <div class="absolute inset-0 bg-black/10"></div>
            <MessageCircle class="w-7 h-7 text-white z-10" />
         </div>
      </div>
    </div>
  </div>
</template>

<style>
@font-face {
  font-family: 'JetBrains Mono Nerd Font';
  src: url('/fonts/JetBrainsMonoNerdFont.ttf') format('truetype');
  font-weight: normal;
  font-style: normal;
}

body {
  margin: 0;
  overflow: hidden;
}

.xterm-viewport::-webkit-scrollbar {
  width: 10px;
}
.xterm-viewport::-webkit-scrollbar-thumb {
  background: #ffffff20;
  border-radius: 4px;
}
.xterm-viewport::-webkit-scrollbar-thumb:hover {
  background: #ffffff40;
}
.xterm-screen {
  opacity: 0.98;
}
</style>
