<script setup lang="ts">
import { ref } from 'vue'
import { currentWorkspace, workspaceCount, myName, wsSend } from '../composables/useWs'

const props = defineProps<{ clock: string; activeApp: string }>()
</script>

<template>
  <!-- Fixed top bar, lives above ALL windows via z-index -->
  <div
    class="fixed top-0 left-0 right-0 h-7 flex items-center justify-between px-3.5 text-[12px] font-medium text-white border-b"
    style="
      z-index: 2147483640;
      background: rgba(18,18,22,0.4);
      backdrop-filter: blur(20px) saturate(160%);
      -webkit-backdrop-filter: blur(20px) saturate(160%);
      border-color: rgba(255,255,255,0.06);
      user-select: none;
    "
  >
    <!-- Left — logo + active app -->
    <div class="flex items-center gap-3.5 h-full">
      <div class="cursor-default px-1 h-full flex items-center transition-opacity duration-200 hover:opacity-100 opacity-80 active:scale-95">
        <svg viewBox="0 0 24 24" class="w-3.5 h-3.5 fill-white">
          <path d="M15.2 10.5c-.1-2.5 2-3.7 2.1-3.8-1.2-1.7-3-1.9-3.6-2-.8-.1-2.1.5-2.8.5-.8 0-1.8-.5-2.7-.5-1.2 0-2.4.7-3 1.8-1.3 2.3-.3 5.7 1 7.6.6.9 1.3 1.9 2.3 1.9.9 0 1.3-.6 2.4-.6 1.1 0 1.4.6 2.4.6 1 0 1.6-1 2.2-1.9.8-1.1 1.1-2.2 1.1-2.2-.1-.1-1.4-.5-1.4-2.4zM11.6 3.4c.5-.6.8-1.4.7-2.2-.7 0-1.6.3-2.1.9-.4.5-.8 1.3-.7 2.1.8.1 1.6-.2 2.1-.8z"/>
        </svg>
      </div>
      <div class="font-bold tracking-tight cursor-default opacity-90 transition-all duration-250 ease-[var(--ease-out)]">
        {{ activeApp }}
      </div>
    </div>

    <!-- Center — workspace dots -->
    <div class="flex items-center gap-2">
      <button
        v-for="i in workspaceCount" :key="i"
        @click="currentWorkspace = i - 1"
        :class="currentWorkspace === i - 1 ? 'w-2.5 bg-white shadow-[0_0_8px_rgba(255,255,255,0.5)]' : 'w-2 bg-white/20'"
        class="h-2 rounded-full cursor-pointer transition-all duration-250 ease-[var(--ease-out)] hover:bg-white/40 active:scale-90"
      />
      <button
        v-if="workspaceCount < 10"
        @click="() => { workspaceCount++; wsSend({type:'SetWorkspaceCount',count:workspaceCount}) }"
        class="w-4 h-4 flex items-center justify-center rounded-full opacity-30 hover:opacity-100 hover:bg-white/10 transition-all duration-200 cursor-pointer active:scale-90"
        style="color:white; font-size: 14px; line-height: 1;">+</button>
    </div>

    <!-- Right — clock + username -->
    <div class="flex items-center gap-4 h-full">
      <div class="cursor-default opacity-80 font-mono text-[11px] tracking-wide">{{ clock }}</div>
      <div class="text-[10px] opacity-40 font-semibold tracking-tighter uppercase cursor-default px-1.5 py-0.5 rounded bg-white/5">{{ myName }}</div>
    </div>
  </div>
</template>
