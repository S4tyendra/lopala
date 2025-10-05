<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick, markRaw } from 'vue';
import { Terminal } from 'xterm';
import { FitAddon } from 'xterm-addon-fit';
import 'xterm/css/xterm.css';
import { Terminal as TerminalIcon, FileText, Menu } from 'lucide-vue-next';

interface TerminalSession {
  ws: WebSocket;
  term: Terminal;
  fitAddon: FitAddon;
}

interface AppWindow {
  id: string;
  type: 'terminal' | 'log';
  title: string;
  x: number;
  y: number;
  w: number;
  h: number;
  z: number;
  minimized: boolean;
  maximized: boolean;
  termSession: TerminalSession | null;
  _preX?: number;
  _preY?: number;
  _preW?: number;
  _preH?: number;
}

const windows = ref<AppWindow[]>([]);
const activeWindowId = ref<string | null>(null);
let zCounter = 10;

const spawnTerminal = async () => {
  const id = Math.random().toString(36).substring(7);
  const win: AppWindow = {
    id, type: 'terminal', title: `Terminal`,
    x: 100 + (windows.value.length * 40) % 300, 
    y: 100 + (windows.value.length * 40) % 300,
    w: 700, h: 450, z: ++zCounter, 
    minimized: false, maximized: false, termSession: null
  };
  
  windows.value.push(win);
  focusWindow(win.id);
  await nextTick();
  
  const term = new Terminal({
    cursorBlink: true,
    fontSize: 14,
    fontFamily: '"JetBrains Mono Nerd Font", "JetBrains Mono", monospace',
    theme: {
      background: '#0c0c0c',
      foreground: '#d4d4d4',
      cursor: '#00ffcc',
      selectionBackground: '#ffffff33',
    }
  });

  const fitAddon = new FitAddon();
  term.loadAddon(fitAddon);

  const el = document.getElementById(`term-${id}`) as HTMLDivElement;
  if (el) {
    term.open(el);
    fitAddon.fit();
  }

  const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
  const wsUrl = `${protocol}//${window.location.host}/_ws`;
  const ws = new WebSocket(wsUrl);

  win.termSession = {
    ws: markRaw(ws),
    term: markRaw(term),
    fitAddon: markRaw(fitAddon)
  };

  term.onTitleChange((title) => {
    win.title = title || `Terminal`;
  });

  ws.onopen = () => {
    console.log(`[tty-${id}] connected to backend`);
    const dims = { type: 'Resize', data: { rows: term.rows, cols: term.cols } };
    ws.send(JSON.stringify(dims));
  };
  
  ws.onmessage = async (event) => {
    if (event.data instanceof Blob) {
      const arrayBuffer = await event.data.arrayBuffer();
      term.write(new Uint8Array(arrayBuffer));
    }
  };

  term.onData((data) => {
    if (ws.readyState === WebSocket.OPEN) {
      ws.send(new TextEncoder().encode(data));
    }
  });
  
  ws.onclose = () => {
    console.log(`[tty-${id}] disconnected`);
    win.title = `${win.title} (Disconnected)`;
  };
};

const spawnLogs = async () => {
  const win = windows.value.find(w => w.type === 'log');
  if (win) {
    win.minimized = false;
    focusWindow(win.id);
    return;
  }
  const id = Math.random().toString(36).substring(7);
  windows.value.push({
    id, type: 'log', title: `System Logs`,
    x: 200, y: 150, w: 500, h: 350, z: ++zCounter, 
    minimized: false, maximized: false, termSession: null
  });
  focusWindow(id);
};

// Window Management
const focusWindow = (id: string) => {
  const win = windows.value.find(w => w.id === id);
  if (win) {
    win.z = ++zCounter;
    activeWindowId.value = id;
    if (!win.minimized) {
      nextTick(() => {
        win.termSession?.term?.focus();
      });
    }
  }
};

const closeWindow = (id: string) => {
  const i = windows.value.findIndex(w => w.id === id);
  if (i !== -1) {
    const win = windows.value[i];
    win.termSession?.ws?.close();
    win.termSession?.term?.dispose();
    windows.value.splice(i, 1);
  }
};

const toggleMaximize = (win: AppWindow) => {
  if (win.maximized) {
    win.maximized = false;
    win.x = win._preX || 100;
    win.y = win._preY || 100;
    win.w = win._preW || 700;
    win.h = win._preH || 450;
  } else {
    win._preX = win.x;
    win._preY = win.y;
    win._preW = win.w;
    win._preH = win.h;
    win.maximized = true;
    win.x = 0;
    win.y = 0;
    win.w = window.innerWidth;
    win.h = window.innerHeight - 80; // Account for Dock
  }
  nextTick(() => {
    win.termSession?.fitAddon?.fit();
    win.termSession?.ws?.send(JSON.stringify({ type: 'Resize', data: { rows: win.termSession.term.rows, cols: win.termSession.term.cols } }));
  });
};

// Drag & Resize State
let dragState = { isDragging: false, win: null as AppWindow | null, startX: 0, startY: 0, startLeft: 0, startTop: 0 };
let resizeState = { isResizing: false, win: null as AppWindow | null, startX: 0, startY: 0, startW: 0, startH: 0, edge: 'se' };

const startDrag = (e: MouseEvent, win: AppWindow) => {
  if (win.maximized) return;
  dragState = { isDragging: true, win, startX: e.clientX, startY: e.clientY, startLeft: win.x, startTop: win.y };
  focusWindow(win.id);
};

const startResize = (e: MouseEvent, win: AppWindow, edge: string) => {
  if (win.maximized) return;
  e.preventDefault();
  resizeState = { isResizing: true, win, startX: e.clientX, startY: e.clientY, startW: win.w, startH: win.h, edge };
  focusWindow(win.id);
};

const onMouseMove = (e: MouseEvent) => {
  if (dragState.isDragging && dragState.win) {
    dragState.win.x = dragState.startLeft + (e.clientX - dragState.startX);
    dragState.win.y = Math.max(0, dragState.startTop + (e.clientY - dragState.startY));
  }
  if (resizeState.isResizing && resizeState.win) {
    const w = resizeState.win;
    if (resizeState.edge.includes('e')) w.w = Math.max(300, resizeState.startW + (e.clientX - resizeState.startX));
    if (resizeState.edge.includes('s')) w.h = Math.max(200, resizeState.startH + (e.clientY - resizeState.startY));
    w.termSession?.fitAddon?.fit();
  }
};

const onMouseUp = () => {
  if (resizeState.isResizing && resizeState.win) {
     const termSession = resizeState.win.termSession;
     if (termSession) {
        termSession.fitAddon?.fit();
        termSession.ws?.send(JSON.stringify({ type: 'Resize', data: { rows: termSession.term.rows, cols: termSession.term.cols } }));
     }
  }
  dragState.isDragging = false;
  resizeState.isResizing = false;
};

// Keyboard Shortcuts
const onKeyDown = (e: KeyboardEvent) => {
  if ((e.metaKey || e.ctrlKey) && e.key === 'n') {
    e.preventDefault();
    spawnTerminal();
  }
};

onMounted(() => {
  window.addEventListener('mousemove', onMouseMove);
  window.addEventListener('mouseup', onMouseUp);
  window.addEventListener('keydown', onKeyDown);
  
  window.addEventListener('resize', () => {
    windows.value.filter(w => w.maximized || !w.minimized).forEach(w => {
      if (w.maximized) {
        w.w = window.innerWidth;
        w.h = window.innerHeight - 80;
      }
      if (w.termSession) {
        w.termSession.fitAddon?.fit();
        w.termSession.ws?.send(JSON.stringify({ type: 'Resize', data: { rows: w.termSession.term.rows, cols: w.termSession.term.cols } }));
      }
    });
  });

  // Initial Open
  spawnTerminal();
});

onUnmounted(() => {
  window.removeEventListener('mousemove', onMouseMove);
  window.removeEventListener('mouseup', onMouseUp);
  window.removeEventListener('keydown', onKeyDown);
  windows.value.forEach(w => w.termSession?.ws?.close());
});
</script>

<template>
  <!-- Desktop Environment -->
  <div class="h-screen w-screen overflow-hidden bg-slate-950 font-sans text-gray-200 select-none relative" 
       style="background-image: radial-gradient(circle at 50% 10%, #1a202c 0%, #020617 100%);">
    
    <!-- Workspace Layer -->
    <div class="absolute inset-0 overflow-hidden pointer-events-none">
      
      <!-- Windows -->
      <div v-for="win in windows" 
           :key="win.id"
           :style="{ left: win.x + 'px', top: win.y + 'px', width: win.w + 'px', height: win.h + 'px', zIndex: win.z }"
           v-show="!win.minimized"
           class="absolute flex flex-col bg-[#1c1c1e] rounded-xl overflow-hidden shadow-2xl pointer-events-auto border border-[#ffffff15] transition-[shadow,transform] duration-75"
           @mousedown="focusWindow(win.id)"
           :class="[activeWindowId === win.id ? 'shadow-[0_12px_40px_rgba(0,0,0,0.8)] border-[#ffffff25]' : 'shadow-none brightness-75']"
      >
          <!-- MacOS Title Bar -->
          <div class="h-10 bg-[#2d2d30] flex items-center px-4 flex-shrink-0 cursor-default" @mousedown="startDrag($event, win)">
             <!-- Controls -->
             <div class="flex space-x-2 w-20">
                <div @click.stop="closeWindow(win.id)" class="w-3 h-3 rounded-full bg-[#ff5f56] hover:bg-[#ff5f56]/80 cursor-pointer shadow-inner"></div>
                <div @click.stop="win.minimized = true" class="w-3 h-3 rounded-full bg-[#ffbd2e] hover:bg-[#ffbd2e]/80 cursor-pointer shadow-inner"></div>
                <div @click.stop="toggleMaximize(win)" class="w-3 h-3 rounded-full bg-[#27c93f] hover:bg-[#27c93f]/80 cursor-pointer shadow-inner"></div>
             </div>
             
             <!-- Title -->
             <div class="flex-1 text-center text-xs font-semibold text-gray-300 pointer-events-none truncate cursor-default select-none tracking-wide">
                {{ win.title }}
             </div>
             <div class="w-20"></div> <!-- Balance -->
          </div>
          
          <!-- Inner Payload -->
          <div class="flex-1 relative bg-[#060606]">
              <div v-if="win.type === 'terminal'" :id="'term-' + win.id" class="absolute inset-0 p-1"></div>
              
              <div v-if="win.type === 'log'" class="absolute inset-0 p-4 overflow-y-auto text-xs font-mono text-zinc-400 bg-[#0c0c0c] select-text">
                 <p class="text-green-500 mb-2">[System] Lopala Environment Initialized</p>
                 <p v-for="w in windows" :key="'log'+w.id" class="mb-1 text-zinc-300">
                    <span class="text-blue-400">INFO</span> - Window '{{w.title}}' [{{w.type}}] active {{w.w}}x{{w.h}} | z-index: {{w.z}}
                 </p>
              </div>
          </div>
          
          <!-- Invisible Resize Borders -->
          <div class="absolute right-0 bottom-0 w-4 h-4 cursor-se-resize z-50" @mousedown.stop="startResize($event, win, 'se')"></div>
          <div class="absolute right-0 top-0 bottom-4 w-1 cursor-e-resize z-50" @mousedown.stop="startResize($event, win, 'e')"></div>
          <div class="absolute left-0 bottom-0 right-4 h-1 cursor-s-resize z-50" @mousedown.stop="startResize($event, win, 's')"></div>
      </div>
    </div>

    <!-- macOS Dock -->
    <div class="absolute bottom-4 left-1/2 transform -translate-x-1/2 flex items-end space-x-3 bg-white/5 backdrop-blur-xl px-4 py-2.5 rounded-2xl border border-white/10 shadow-2xl z-[99999] pointer-events-auto">
      
      <!-- Terminal App -->
      <div @click="spawnTerminal" class="group flex flex-col items-center cursor-pointer transition-transform hover:-translate-y-2 origin-bottom">
         <div class="w-14 h-14 bg-gradient-to-br from-zinc-700 to-zinc-900 rounded-[14px] flex items-center justify-center border border-white/20 shadow-xl relative overflow-hidden">
            <div class="absolute inset-0 bg-black/20"></div>
            <TerminalIcon class="w-7 h-7 text-white z-10" />
            
            <div class="absolute -top-1 -left-1 w-6 h-6 bg-white/10 rounded-full blur-md"></div>
         </div>
         <div class="w-1 h-1 bg-white/70 rounded-full mt-2" v-if="windows.some(w => w.type === 'terminal')"></div>
      </div>
      
      <div class="w-px h-12 bg-white/10 mx-1"></div>
      
      <!-- Logs App -->
      <div @click="spawnLogs" class="group flex flex-col items-center cursor-pointer transition-transform hover:-translate-y-2 origin-bottom">
         <div class="w-14 h-14 bg-gradient-to-br from-blue-600 to-indigo-800 rounded-[14px] flex items-center justify-center border border-white/20 shadow-xl relative overflow-hidden">
            <div class="absolute inset-0 bg-black/10"></div>
            <FileText class="w-7 h-7 text-white z-10" />
         </div>
         <div class="w-1 h-1 bg-white/70 rounded-full mt-2" v-if="windows.some(w => w.type === 'log')"></div>
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
