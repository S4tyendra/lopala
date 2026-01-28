<script setup lang="ts">
const features = [
  { title: 'Terminal', desc: 'Full PTY support with multi-user sync and professional toolbars.' },
  { id: 'files', title: 'Finder', desc: 'Powerful file manager with grid/list views and drag-and-drop uploads.' },
  { title: 'Spotlight', desc: 'Global search for apps and files via Ctrl+K, powered by ripgrep.' },
  { title: 'Collaboration', desc: 'Real-time multi-user cursor sync and shared window management.' },
]

const shortcuts = [
  { keys: 'Ctrl + K', desc: 'Open Spotlight Search' },
  { keys: 'Ctrl + Shift + C', desc: 'Copy terminal selection' },
  { keys: 'Double Click Title', desc: 'Maximize/Restore window' },
  { keys: 'wdl <file>', desc: 'Download file via Terminal to browser' },
]
</script>

<template>
  <div class="h-full overflow-y-auto px-6 py-8 select-none" style="background:rgba(10,10,12,0.95)">
    <div class="max-w-[700px] mx-auto font-sans">
      <div class="text-3xl font-bold mb-2 text-white/95">Lopala OS</div>
      <div class="text-[14px] text-white/60 mb-10 leading-relaxed font-medium">Welcome to your ultimate cloud-native
        workspace. Everything you see is synced across all clients in real-time.</div>

      <div class="section mb-8">
        <h3 class="text-[12px] font-bold uppercase tracking-wider text-white/40 mb-3">Core Features</h3>
        <div class="grid gap-3">
          <div v-for="f in features" :key="f.title"
            class="bg-white/5 border border-white/5 rounded-2xl p-4 transition-all hover:bg-white/10 group">
            <div class="text-[15px] font-bold text-white/90 mb-1">{{ f.title }}</div>
            <div class="text-[13px] text-white/60 leading-snug">{{ f.desc }}</div>
          </div>
        </div>
      </div>

      <div class="section mb-8">
        <h3 class="text-[12px] font-bold uppercase tracking-wider text-white/40 mb-3">Master the Flow</h3>
        <div class="grid gap-2">
          <div v-for="s in shortcuts" :key="s.keys"
            class="flex items-center justify-between p-3 rounded-xl bg-white/5 border border-white/5">
            <span class="text-[13px] font-medium text-white/70">{{ s.desc }}</span>
            <span
              class="text-[11px] font-bold font-mono bg-white/10 px-2 py-1 rounded border border-white/10 text-white/90">{{
              s.keys }}</span>
          </div>
        </div>
      </div>

      <div class="section mb-8">
        <h3 class="text-[12px] font-bold uppercase tracking-wider text-white/40 mb-3">Development & Support</h3>
        <div class="flex gap-4">
          <a href="https://github.com/s4tyendra/lopala" target="_blank" rel="noopener noreferrer"
            class="flex-1 p-4 rounded-2xl bg-blue-500/15 border border-blue-400/20 text-center hover:bg-blue-500/25 transition-all active:scale-95 duration-200 group">
            <div class="text-[13px] font-bold text-blue-300">GitHub Repo</div>
            <div class="text-[11px] text-blue-300/70 mt-0.5">Star the project! ★</div>
          </a>
          <a href="https://satyendra.com/blog/why-i-built-lopala" target="_blank" rel="noopener noreferrer"
            class="flex-1 p-4 rounded-2xl bg-purple-500/15 border border-purple-400/20 text-center hover:bg-purple-500/25 transition-all active:scale-95 duration-200 group">
            <div class="text-[13px] font-bold text-purple-300">Developer Blog</div>
            <div class="text-[11px] text-purple-300/70 mt-0.5">Read the deep dives</div>
          </a>
        </div>
      </div>

      <!-- Legal -->
      <div
        class="mt-16 pt-8 border-t border-white/5 text-white/20 text-[10px] text-center uppercase tracking-widest pb-12">
        Crafted with passion by Satyendra · MIT License
      </div>
    </div>
  </div>
</template>

<style scoped>
.section h3 {
  border-left: 2px solid rgba(255, 255, 255, 0.1);
  padding-left: 12px;
}

::-webkit-scrollbar {
  width: 4px;
}

::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.1);
  border-radius: 10px;
}
</style>
