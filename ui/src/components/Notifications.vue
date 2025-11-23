<script setup lang="ts">
import { notifications } from '../composables/useNotifications'

const dismiss = (id: string) => {
  notifications.value = notifications.value.filter(n => n.id !== id)
}

const borderColor = (type: string) => {
  if (type === 'success') return 'rgba(34,197,94,0.35)'
  if (type === 'warning') return 'rgba(234,179,8,0.35)'
  if (type === 'error')   return 'rgba(239,68,68,0.35)'
  return 'rgba(255,255,255,0.15)'
}

const accentColor = (type: string) => {
  if (type === 'success') return '#22c55e'
  if (type === 'warning') return '#eab308'
  if (type === 'error')   return '#ef4444'
  return '#60a5fa'
}
</script>

<template>
  <TransitionGroup
    tag="div"
    name="notif"
    class="fixed flex flex-col gap-2 pointer-events-none select-none"
    style="top: 40px; right: 16px; z-index: 2147483645; width: 300px;"
  >
    <div
      v-for="n in notifications"
      :key="n.id"
      @click="dismiss(n.id)"
      class="pointer-events-auto flex items-start gap-3 px-4 py-3 rounded-2xl shadow-2xl cursor-pointer"
      style="
        background: rgba(26, 26, 30, 0.92);
        backdrop-filter: blur(24px) saturate(160%);
        -webkit-backdrop-filter: blur(24px) saturate(160%);
      "
      :style="{
        border: `1px solid ${borderColor(n.type)}`,
        boxShadow: `0 12px 40px rgba(0,0,0,0.45), inset 0 1px 0 rgba(255,255,255,0.07), 0 0 0 0.5px ${borderColor(n.type)}`
      }"
    >
      <!-- Icon / accent bar -->
      <div class="flex-none flex items-center justify-center w-8 h-8 rounded-full text-[18px]"
        :style="{ background: `${accentColor(n.type)}18` }">
        {{ n.icon ?? '🔔' }}
      </div>

      <!-- Text -->
      <div class="flex-1 min-w-0 pt-0.5">
        <div class="font-semibold text-[13px] text-white leading-tight">{{ n.title }}</div>
        <div v-if="n.body" class="text-[12px] mt-0.5" style="color: rgba(255,255,255,0.45)">{{ n.body }}</div>
      </div>

      <!-- Close glyph -->
      <div class="flex-none text-[11px] mt-0.5" style="color: rgba(255,255,255,0.25)">✕</div>
    </div>
  </TransitionGroup>
</template>

<style scoped>
.notif-enter-active {
  transition: all 0.3s cubic-bezier(0.23, 1, 0.32, 1);
}
.notif-leave-active {
  transition: all 0.22s cubic-bezier(0.4, 0, 1, 1);
}
.notif-enter-from {
  opacity: 0;
  transform: translateX(32px) scale(0.95);
}
.notif-leave-to {
  opacity: 0;
  transform: translateX(16px) scale(0.96);
}
.notif-move {
  transition: transform 0.28s cubic-bezier(0.23, 1, 0.32, 1);
}
</style>
