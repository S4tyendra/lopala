<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { Terminal } from 'xterm';
import { FitAddon } from 'xterm-addon-fit';
import 'xterm/css/xterm.css';
import { Terminal as TerminalIcon, Cpu, Globe, Settings, Network } from 'lucide-vue-next';

const terminalRef = ref<HTMLDivElement | null>(null);
const ws = ref<WebSocket | null>(null);
const tunnelUrl = ref<string | null>(null);
const latency = ref<number>(0);
const connected = ref(false);

let term: Terminal;
let fitAddon: FitAddon;

const initTerminal = () => {
  term = new Terminal({
    cursorBlink: true,
    fontSize: 14,
    fontFamily: 'JetBrains Mono, monospace',
    theme: {
      background: '#0c0c0c',
      foreground: '#d4d4d4',
      cursor: '#00ffcc',
      selectionBackground: '#ffffff33',
    }
  });

  fitAddon = new FitAddon();
  term.loadAddon(fitAddon);

  if (terminalRef.value) {
    term.open(terminalRef.value);
    fitAddon.fit();
  }

  // WS Setup
  const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
  const wsUrl = `${protocol}//${window.location.host}/_ws`;
  ws.value = new WebSocket(wsUrl);

  ws.value.onopen = () => {
    connected.value = true;
    term.write('\r\n\x1b[32m✔ Connected to Lopala Server\x1b[0m\r\n');
    
    // Initial resize sync
    const dims = { type: 'Resize', data: { rows: term.rows, cols: term.cols } };
    ws.value?.send(JSON.stringify(dims));
  };

  ws.value.onmessage = async (event) => {
    if (event.data instanceof Blob) {
      const arrayBuffer = await event.data.arrayBuffer();
      term.write(new Uint8Array(arrayBuffer));
    }
  };

  ws.value.onclose = () => {
    connected.value = false;
    term.write('\r\n\x1b[31m✘ Connection Closed\x1b[0m\r\n');
  };

  // Input handling
  term.onData((data) => {
    if (ws.value?.readyState === WebSocket.OPEN) {
      ws.value.send(new TextEncoder().encode(data));
    }
  });

  window.addEventListener('resize', () => fitAddon.fit());
};

onMounted(() => {
  initTerminal();
});

onUnmounted(() => {
  ws.value?.close();
});
</script>

<template>
  <div class="flex min-h-screen bg-[#060606] text-white">
    <!-- Glassy Sidebar -->
    <aside class="w-64 border-r border-[#ffffff10] bg-[#ffffff05] backdrop-blur-xl flex flex-col p-6 space-y-8 h-screen z-10">
      <div class="flex items-center space-x-3">
        <div class="w-10 h-10 bg-linear-to-br from-cyan-400 to-blue-600 rounded-xl flex items-center justify-center shadow-lg shadow-cyan-500/20">
          <TerminalIcon class="w-6 h-6 text-white" />
        </div>
        <h1 class="text-xl font-bold tracking-tight">LOPALA</h1>
      </div>

      <nav class="flex-1 space-y-1">
        <a href="#" class="flex items-center space-x-3 px-4 py-3 bg-[#ffffff10] rounded-lg border border-[#ffffff10] text-cyan-400 transition-all">
          <Cpu class="w-5 h-5" />
          <span class="font-medium text-sm">Dashboard</span>
        </a>
        <a href="#" class="flex items-center space-x-3 px-4 py-3 text-zinc-400 hover:bg-[#ffffff05] rounded-lg transition-all">
          <Network class="w-5 h-5" />
          <span class="font-medium text-sm">Tunnel Logs</span>
        </a>
        <a href="#" class="flex items-center space-x-3 px-4 py-3 text-zinc-400 hover:bg-[#ffffff05] rounded-lg transition-all">
          <Globe class="w-5 h-5" />
          <span class="font-medium text-sm">Network</span>
        </a>
      </nav>

      <div class="pt-6 border-t border-[#ffffff10]">
        <div class="flex items-center justify-between px-4">
          <div class="flex items-center space-x-2">
            <div :class="['w-2 h-2 rounded-full', connected ? 'bg-green-500 shadow-[0_0_8px_#22c55e]' : 'bg-red-500 shadow-[0_0_8px_#ef4444]']"></div>
            <span class="text-xs font-mono text-zinc-500">{{ connected ? 'LIVE' : 'OFFLINE' }}</span>
          </div>
          <Settings class="w-4 h-4 text-zinc-500 hover:text-white cursor-pointer transition-colors" />
        </div>
      </div>
    </aside>

    <!-- Main View -->
    <main class="flex-1 flex flex-col overflow-hidden relative">
      <!-- Top Bar / Glassy Nav -->
      <header class="h-16 border-b border-[#ffffff10] bg-[#ffffff02] backdrop-blur-md flex items-center justify-between px-8 z-10">
        <div class="flex items-center space-x-6">
          <div class="flex flex-col">
            <span class="text-[10px] text-zinc-500 font-bold uppercase tracking-wider">Environment</span>
            <span class="text-sm font-medium">Ubuntu 24.04 LTS</span>
          </div>
          <div class="w-px h-6 bg-[#ffffff10]"></div>
          <div class="flex flex-col">
            <span class="text-[10px] text-zinc-500 font-bold uppercase tracking-wider">Uptime</span>
            <span class="text-sm font-medium">0.4 ms lag</span>
          </div>
        </div>

        <div class="flex items-center space-x-4">
          <button class="bg-[#ffffff10] border border-[#ffffff10] px-4 py-1.5 rounded-full text-xs font-semibold hover:bg-[#ffffff20] transition-all">
            Share Tunnel
          </button>
        </div>
      </header>

      <!-- Terminal Area -->
      <section class="flex-1 p-6 relative">
        <!-- Terminal Container -->
        <div class="w-full h-full bg-[#0c0c0c] rounded-2xl border border-[#ffffff10] overflow-hidden shadow-2xl relative group">
          <!-- Terminal Header Buttons (Mac style) -->
           <div class="absolute top-4 left-4 flex space-x-2 z-20">
             <div class="w-2.5 h-2.5 rounded-full bg-[#ff5f56]"></div>
             <div class="w-2.5 h-2.5 rounded-full bg-[#ffbd2e]"></div>
             <div class="w-2.5 h-2.5 rounded-full bg-[#27c93f]"></div>
           </div>

          <!-- The Terminal -->
          <div ref="terminalRef" class="w-full h-full p-4 pt-10"></div>
          
          <!-- Background Glows -->
          <div class="absolute -top-1/2 -right-1/2 w-full h-full bg-cyan-500/5 blur-[120px] rounded-full pointer-events-none"></div>
          <div class="absolute -bottom-1/2 -left-1/2 w-full h-full bg-blue-500/5 blur-[120px] rounded-full pointer-events-none"></div>
        </div>
      </section>

      <!-- Grid Background -->
      <div class="absolute inset-0 bg-[url('https://grainy-gradients.vercel.app/noise.svg')] opacity-20 pointer-events-none"></div>
      <div class="absolute inset-0 bg-[linear-gradient(to_right,#ffffff05_1px,transparent_1px),linear-gradient(to_bottom,#ffffff05_1px,transparent_1px)] bg-size-[40px_40px] pointer-events-none leading-none"></div>
    </main>
  </div>
</template>

<style>
/* Xterm override for transparency/custom scroll */
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
