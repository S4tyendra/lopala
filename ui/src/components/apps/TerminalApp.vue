<script setup lang="ts">
import { onMounted, onUnmounted, watch } from 'vue'
import { initTerminal, writeToTerminal } from '../../composables/useTerminals'

const props = defineProps<{ winId: string }>()

// Boot terminal once mounted, clean up on unmount
onMounted(() => initTerminal(props.winId))

// xterm writes come from parent via event bus — parent calls writeToTerminal directly
</script>

<template>
  <div :id="`term-${winId}`" class="absolute inset-0 p-[2px]" style="background:#0c0c0c;" />
</template>
