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
        <img src="/linux.svg" class="w-4 h-4 object-contain" style="filter: brightness(0) invert(1);" alt="Linux" />
      </div>
      <div class="font-bold tracking-tight cursor-default opacity-90 transition-all duration-200 ease-out">
        {{ activeApp }}
      </div>
    </div>

    <!-- Center — workspace dots -->
    <div class="flex items-center gap-2">
      <button
        v-for="i in workspaceCount" :key="i"
        @click="currentWorkspace = i - 1"
        :class="currentWorkspace === i - 1 ? 'w-2.5 bg-white shadow-[0_0_8px_rgba(255,255,255,0.5)]' : 'w-2 bg-white/20'"
        class="h-2 rounded-full cursor-pointer transition-all duration-200 ease-out hover:bg-white/40 active:scale-90"
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
