<script setup lang="ts">
import { ref, computed } from 'vue'
import { currentWorkspace, workspaceCount, myName, myId, myLatency, users, wsSend } from '../composables/useWs'

const props = defineProps<{ clock: string; activeApp: string }>()

const showUsers = ref(false)

const allUsers = computed(() =>
  Object.values(users.value).map(u => ({
    ...u,
    isSelf: u.id === myId.value,
    latencyLabel: u.id === myId.value
      ? (myLatency.value !== null ? `${myLatency.value}ms` : '—')
      : (u.latency_ms ? `${u.latency_ms}ms` : '—'),
    latencyColor: (ms: number | undefined) => {
      if (!ms) return 'rgba(255,255,255,0.3)'
      if (ms < 50) return '#4ade80'
      if (ms < 150) return '#facc15'
      return '#f87171'
    },
  }))
)

const myLatencyLabel = computed(() =>
  myLatency.value !== null ? `${myLatency.value}ms` : null
)
const myLatencyColor = computed(() => {
  const ms = myLatency.value
  if (ms === null) return 'rgba(255,255,255,0.3)'
  if (ms < 50) return '#4ade80'
  if (ms < 150) return '#facc15'
  return '#f87171'
})
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

    <!-- Right — clock + username + latency + user panel toggle -->
    <div class="flex items-center gap-3 h-full relative">
      <div class="cursor-default opacity-80 font-mono text-[11px] tracking-wide">{{ clock }}</div>

      <!-- Latency badge for self -->
      <div v-if="myLatencyLabel"
        class="text-[10px] font-mono px-1 rounded"
        :style="`color:${myLatencyColor}; background:${myLatencyColor}18`">
        {{ myLatencyLabel }}
      </div>

      <!-- Username button — click to toggle users panel -->
      <button
        @click="showUsers = !showUsers"
        class="text-[10px] font-semibold tracking-tighter uppercase px-1.5 py-0.5 rounded transition-all duration-150 cursor-pointer"
        :style="showUsers
          ? 'background:rgba(255,255,255,0.15); color:rgba(255,255,255,0.9);'
          : 'background:rgba(255,255,255,0.05); color:rgba(255,255,255,0.4);'"
      >
        {{ myName }}
        <span class="ml-1 opacity-60">· {{ allUsers.length }}</span>
      </button>

      <!-- Users Dropdown Panel -->
      <Transition name="userpanel">
        <div v-if="showUsers"
          class="absolute top-8 right-0 rounded-xl border overflow-hidden shadow-[0_16px_48px_rgba(0,0,0,0.6)]"
          style="
            min-width: 220px;
            background: rgba(16,16,20,0.92);
            backdrop-filter: blur(40px) saturate(180%);
            border-color: rgba(255,255,255,0.1);
            z-index: 2147483641;
          "
          @mouseleave="showUsers = false"
        >
          <div class="px-3 pt-2.5 pb-1 text-[9px] uppercase tracking-[0.15em] font-bold"
            style="color:rgba(255,255,255,0.25)">
            Connected Users
          </div>
          <div class="flex flex-col pb-1.5">
            <div v-for="u in allUsers" :key="u.id"
              class="flex items-center gap-2.5 px-3 py-2 transition-colors hover:bg-white/5">
              <!-- Color dot -->
              <div class="w-2 h-2 rounded-full flex-none" :style="`background: ${u.color}; box-shadow: 0 0 6px ${u.color}88`" />
              <!-- Name -->
              <div class="flex-1 min-w-0">
                <span class="text-[12px] font-semibold truncate" :style="`color: ${u.color}`">
                  {{ u.name || u.id }}
                  <span v-if="u.isSelf" class="ml-1 text-[9px] opacity-40 font-normal normal-case">you</span>
                </span>
              </div>
              <!-- Latency -->
              <div class="text-[10px] font-mono flex-none"
                :style="`color: ${u.latencyColor(u.id === myId ? myLatency ?? undefined : u.latency_ms)}`">
                {{ u.latencyLabel }}
              </div>
            </div>
          </div>
        </div>
      </Transition>
    </div>
  </div>
</template>

<style scoped>
.userpanel-enter-active {
  transition: opacity 150ms ease, transform 180ms cubic-bezier(0.16,1,0.3,1);
}
.userpanel-leave-active {
  transition: opacity 100ms ease, transform 100ms ease;
}
.userpanel-enter-from,
.userpanel-leave-to {
  opacity: 0;
  transform: translateY(-6px) scale(0.97);
}
</style>
