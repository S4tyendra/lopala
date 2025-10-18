<script setup lang="ts">
import { ref } from 'vue'
import { currentWorkspace, workspaceCount, myName, wsSend } from '../composables/useWs'

const props = defineProps<{ clock: string; activeApp: string }>()
</script>

<template>
  <!-- Fixed top bar, lives above ALL windows via z-index -->
  <div
    class="fixed top-0 left-0 right-0 h-7 flex items-center justify-between px-4 text-[13px] font-medium text-white border-b"
    style="
      z-index: 2147483640;
      background: rgba(25,25,28,0.45);
      backdrop-filter: blur(24px) saturate(130%);
      -webkit-backdrop-filter: blur(24px) saturate(130%);
      border-color: rgba(255,255,255,0.08);
      user-select: none;
    "
  >
    <!-- Left —  logo + active app -->
    <div class="flex items-center gap-4 h-full">
      <span class="cursor-default px-1.5 h-full flex items-center">
        <!-- Apple-style logo SVG -->
        <svg viewBox="0 0 24 24" class="w-3.5 h-3.5 fill-white opacity-80">
          <path d="M15.2 10.5c-.1-2.5 2-3.7 2.1-3.8-1.2-1.7-3-1.9-3.6-2-.8-.1-2.1.5-2.8.5-.8 0-1.8-.5-2.7-.5-1.2 0-2.4.7-3 1.8-1.3 2.3-.3 5.7 1 7.6.6.9 1.3 1.9 2.3 1.9.9 0 1.3-.6 2.4-.6 1.1 0 1.4.6 2.4.6 1 0 1.6-1 2.2-1.9.8-1.1 1.1-2.2 1.1-2.2-.1-.1-1.4-.5-1.4-2.4zM11.6 3.4c.5-.6.8-1.4.7-2.2-.7 0-1.6.3-2.1.9-.4.5-.8 1.3-.7 2.1.8.1 1.6-.2 2.1-.8z"/>
        </svg>
      </span>
      <span class="font-semibold text-[13px] cursor-default opacity-90">{{ activeApp }}</span>
    </div>

    <!-- Center — workspace dots -->
    <div class="flex items-center gap-1.5">
      <button
        v-for="i in workspaceCount" :key="i"
        @click="currentWorkspace = i - 1"
        :style="currentWorkspace === i - 1
          ? 'background:rgba(255,255,255,0.9); transform:scale(1.15)'
          : 'background:rgba(255,255,255,0.25)'"
        class="w-2 h-2 rounded-full cursor-pointer transition-[background,transform] duration-200 hover:brightness-125"
      />
      <button
        v-if="workspaceCount < 10"
        @click="() => { workspaceCount++; wsSend({type:'SetWorkspaceCount',count:workspaceCount}) }"
        class="text-[11px] ml-1 opacity-30 hover:opacity-70 transition-opacity duration-150 cursor-pointer"
        style="color:white">+</button>
    </div>

    <!-- Right — clock + username -->
    <div class="flex items-center gap-4 h-full">
      <span class="cursor-default opacity-75 text-[12px]">{{ clock }}</span>
      <span class="text-[11px] opacity-40 cursor-default">{{ myName }}</span>
    </div>
  </div>
</template>
