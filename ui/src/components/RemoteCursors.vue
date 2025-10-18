<script setup lang="ts">
import { computed } from 'vue'
import { users, myId, currentWorkspace } from '../composables/useWs'

// Only show remote users on same workspace
const remoteUsers = computed(() =>
  Object.values(users.value).filter(u => u.id !== myId.value && u.workspace === currentWorkspace.value)
)
</script>

<template>
  <!-- Rendered ABOVE everything except nothing — max z-index layer -->
  <div
    class="fixed inset-0 pointer-events-none"
    style="z-index: 2147483647"
  >
    <div
      v-for="u in remoteUsers" :key="u.id"
      class="absolute"
      :style="{ left: u.x + 'px', top: u.y + 'px', transition: 'left 60ms linear, top 60ms linear' }"
    >
      <!-- Cursor shape -->
      <svg width="22" height="22" viewBox="0 0 22 22" style="transform: translate(-2px,-2px) rotate(-5deg); filter: drop-shadow(0 2px 4px rgba(0,0,0,0.4))">
        <path :fill="u.color" d="M3,0 L3,18 L7,14 L11,22 L13,21 L9,13 L15,13 Z" stroke="rgba(0,0,0,0.3)" stroke-width="0.5"/>
      </svg>
      <!-- Name label -->
      <div
        class="absolute left-4 top-3 px-2 py-0.5 rounded-full text-[10px] font-semibold whitespace-nowrap shadow-lg"
        :style="{ background: u.color, color: '#000', opacity: 0.92 }"
      >{{ u.name }}</div>
    </div>
  </div>
</template>
