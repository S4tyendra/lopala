<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick, markRaw } from 'vue';
import { Terminal } from 'xterm';
import { FitAddon } from 'xterm-addon-fit';
import 'xterm/css/xterm.css';
import { Terminal as TerminalIcon, Plus, X, Command, ActivitySquare } from 'lucide-vue-next';

interface TerminalSession {
  id: string;
  title: string;
  term: Terminal | null;
  fitAddon: FitAddon | null;
  ws: WebSocket | null;
  connected: boolean;
  element: HTMLDivElement | null;
}

const sessions = ref<TerminalSession[]>([]);
const activeSessionId = ref<string | null>(null);
const globalConnected = ref(false);
const hostUrl = ref(window.location.host);

const initTerminalSession = async () => {
  const id = Math.random().toString(36).substring(7);
  
  const newSession: TerminalSession = {
    id,
    title: `tty-${id}`,
    term: null,
    fitAddon: null,
    ws: null,
    connected: false,
    element: null,
  };
  
  sessions.value.push(newSession);
  activeSessionId.value = id;
  
  await nextTick(); // Wait for DOM element to render
  
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
  
  term.onTitleChange((title) => {
    newSession.title = title || `tty-${id}`;
  });

  const el = document.getElementById(`term-${id}`) as HTMLDivElement;
  if (el) {
    term.open(el);
    fitAddon.fit();
    newSession.element = markRaw(el);
  }
  
  newSession.term = markRaw(term);
  newSession.fitAddon = markRaw(fitAddon);

  const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
  const wsUrl = `${protocol}//${window.location.host}/_ws`;
  const ws = new WebSocket(wsUrl);
  newSession.ws = markRaw(ws);

  ws.onopen = () => {
    newSession.connected = true;
    updateGlobalStatus();
    term.write('\r\n\x1b[32m✔ Connected to Lopala Node\x1b[0m\r\n');
    
    const dims = { type: 'Resize', data: { rows: term.rows, cols: term.cols } };
    ws.send(JSON.stringify(dims));
  };

  ws.onmessage = async (event) => {
    if (event.data instanceof Blob) {
      const arrayBuffer = await event.data.arrayBuffer();
      term.write(new Uint8Array(arrayBuffer));
    }
  };

  ws.onclose = () => {
    newSession.connected = false;
    updateGlobalStatus();
    term.write('\r\n\x1b[31m✘ Connection Closed\x1b[0m\r\n');
  };

  term.onData((data) => {
    if (ws.readyState === WebSocket.OPEN) {
      ws.send(new TextEncoder().encode(data));
    }
  });
};

const updateGlobalStatus = () => {
  globalConnected.value = sessions.value.some(s => s.connected);
};

const switchSession = (id: string) => {
  activeSessionId.value = id;
  nextTick(() => {
    const session = sessions.value.find(s => s.id === id);
    if (session && session.fitAddon) {
      session.fitAddon.fit();
    }
  });
};

const closeSession = (event: Event, id: string) => {
  event.stopPropagation();
  const session = sessions.value.find(s => s.id === id);
  if (session) {
    session.ws?.close();
    session.term?.dispose();
    
    const index = sessions.value.findIndex(s => s.id === id);
    sessions.value.splice(index, 1);
    
    if (activeSessionId.value === id) {
      if (sessions.value.length > 0) {
        switchSession(sessions.value[sessions.value.length - 1].id);
      } else {
        activeSessionId.value = null;
      }
    }
  }
  updateGlobalStatus();
};

onMounted(() => {
  initTerminalSession();
  
  window.addEventListener('resize', () => {
    const activeSession = sessions.value.find(s => s.id === activeSessionId.value);
    if (activeSession && activeSession.fitAddon) {
      activeSession.fitAddon.fit();
      
      const dims = { type: 'Resize', data: { rows: activeSession.term?.rows, cols: activeSession.term?.cols } };
      activeSession.ws?.send(JSON.stringify(dims));
    }
  });
});

onUnmounted(() => {
  sessions.value.forEach(s => s.ws?.close());
});
</script>

<template>
  <div class="flex min-h-screen bg-[#060606] text-white">
    <!-- Glassy Sidebar -->
    <aside class="w-64 border-r border-[#ffffff10] bg-[#ffffff05] backdrop-blur-xl flex flex-col pt-6 pb-6 space-y-8 h-screen z-10 shrink-0">
      <div class="flex items-center space-x-3 px-6">
        <div class="w-10 h-10 bg-linear-to-br from-cyan-400 to-blue-600 rounded-xl flex items-center justify-center shadow-lg shadow-cyan-500/20">
          <TerminalIcon class="w-6 h-6 text-white" />
        </div>
        <h1 class="text-xl font-bold tracking-tight">LOPALA</h1>
      </div>

      <nav class="flex-1 space-y-1 px-4 overflow-y-auto w-full scrollbar-hide">
        <p class="text-xs font-semibold text-zinc-500 mb-3 ml-2 uppercase tracking-wider">Sessions</p>
        
        <div 
          v-for="(session) in sessions" :key="session.id"
          @click="switchSession(session.id)"
          :class="['flex items-center justify-between px-3 py-2.5 rounded-lg transition-all cursor-pointer group border', 
                   activeSessionId === session.id ? 'bg-[#ffffff10] border-[#ffffff10] text-cyan-400 shadow-sm' : 'border-transparent text-zinc-400 hover:bg-[#ffffff05]']"
        >
          <div class="flex items-center space-x-3 overflow-hidden">
            <Command class="w-4 h-4 shrink-0" />
            <span class="font-medium text-sm truncate">{{ session.title }}</span>
          </div>
          
          <div class="flex items-center space-x-2">
            <div :class="['w-1.5 h-1.5 rounded-full', session.connected ? 'bg-green-500 shadow-[0_0_5px_#22c55e]' : 'bg-zinc-600']"></div>
            <button @click="(e) => closeSession(e, session.id)" class="opacity-0 group-hover:opacity-100 transition-opacity hover:bg-zinc-800 p-1 rounded">
              <X class="w-3 h-3" />
            </button>
          </div>
        </div>

        <button @click="initTerminalSession" class="w-full mt-2 flex items-center justify-center space-x-2 px-3 py-2 text-zinc-400 border border-dashed border-[#ffffff20] hover:border-cyan-500 hover:text-cyan-400 rounded-lg transition-all text-sm font-medium">
          <Plus class="w-4 h-4" />
          <span>New Terminal</span>
        </button>
      </nav>

      <div class="pt-6 border-t border-[#ffffff10]">
        <div class="flex items-center justify-between px-6">
          <div class="flex items-center space-x-2">
            <div :class="['w-2 h-2 rounded-full', globalConnected ? 'bg-green-500 shadow-[0_0_8px_#22c55e]' : 'bg-red-500 shadow-[0_0_8px_#ef4444]']"></div>
            <span class="text-xs font-mono text-zinc-500">{{ globalConnected ? 'CLUSTER LIVE' : 'OFFLINE' }}</span>
          </div>
        </div>
      </div>
    </aside>

    <!-- Main View -->
    <main class="flex-1 flex flex-col h-screen min-w-0 relative">
      <!-- Top Bar -->
      <header class="h-16 shrink-0 border-b border-[#ffffff10] bg-[#ffffff02] backdrop-blur-md flex items-center justify-between px-8 z-10">
        <div class="flex items-center space-x-6">
          <div class="flex flex-col">
            <span class="text-[10px] text-zinc-500 font-bold uppercase tracking-wider">Host</span>
            <span class="text-sm font-medium text-zinc-200">{{ hostUrl }}</span>
          </div>
          <div class="w-px h-6 bg-[#ffffff10]"></div>
          <div class="flex flex-col">
            <span class="text-[10px] text-zinc-500 font-bold uppercase tracking-wider">Status</span>
            <span class="text-sm font-medium text-zinc-200">{{ sessions.length }} Active PTYs</span>
          </div>
        </div>
      </header>

      <!-- Terminal Area -->
      <section class="flex-1 p-6 relative min-h-0 bg-[#060606]">
        <div v-if="sessions.length === 0" class="w-full h-full flex flex-col items-center justify-center space-y-4 text-zinc-500 z-10 relative">
          <ActivitySquare class="w-16 h-16 opacity-30" />
          <p>No active terminal sessions.</p>
          <button @click="initTerminalSession" class="px-6 py-2 bg-cyan-600 hover:bg-cyan-500 text-white font-medium rounded-lg transition-colors">Start New Session</button>
        </div>

        <div v-for="session in sessions" :key="session.id" 
             v-show="activeSessionId === session.id"
             class="w-full h-full bg-[#0c0c0c] rounded-2xl border border-[#ffffff10] overflow-hidden shadow-2xl relative group flex flex-col">
             
           <div class="h-10 bg-[#161616] border-b border-[#ffffff05] flex items-center px-4 shrink-0 shadow-sm relative z-20">
             <div class="flex space-x-2">
               <div class="w-2.5 h-2.5 rounded-full bg-[#ff5f56]"></div>
               <div class="w-2.5 h-2.5 rounded-full bg-[#ffbd2e]"></div>
               <div class="w-2.5 h-2.5 rounded-full bg-[#27c93f]"></div>
             </div>
             <div class="flex-1 flex justify-center text-xs font-mono text-zinc-500 select-none overflow-hidden text-ellipsis whitespace-nowrap px-4 max-w-sm mx-auto">
               {{ session.title }}
             </div>
           </div>

          <div :id="'term-' + session.id" class="flex-1 w-full p-4 overflow-hidden relative z-10"></div>
          
          <div class="absolute -top-1/2 -right-1/2 w-full h-full bg-cyan-500/5 blur-[120px] rounded-full pointer-events-none"></div>
          <div class="absolute -bottom-1/2 -left-1/2 w-full h-full bg-blue-500/5 blur-[120px] rounded-full pointer-events-none"></div>
        </div>
      </section>

      <div class="absolute inset-0 bg-[url('https://grainy-gradients.vercel.app/noise.svg')] opacity-10 pointer-events-none"></div>
      <div class="absolute inset-0 bg-[linear-gradient(to_right,#ffffff02_1px,transparent_1px),linear-gradient(to_bottom,#ffffff02_1px,transparent_1px)] bg-size-[40px_40px] pointer-events-none leading-none"></div>
    </main>
  </div>
</template>

<style>
@font-face {
  font-family: 'JetBrains Mono Nerd Font';
  src: url('/fonts/JetBrainsMonoNerdFont.ttf') format('truetype');
  font-weight: normal;
  font-style: normal;
}

.scrollbar-hide::-webkit-scrollbar {
    display: none;
}
.scrollbar-hide {
    -ms-overflow-style: none;
    scrollbar-width: none;
}
.xterm-viewport::-webkit-scrollbar {
  width: 6px;
}
.xterm-viewport::-webkit-scrollbar-thumb {
  background: #ffffff10;
  border-radius: 10px;
}
.xterm-viewport::-webkit-scrollbar-thumb:hover {
  background: #ffffff20;
}
.xterm-screen {
  opacity: 0.95;
}
</style>
