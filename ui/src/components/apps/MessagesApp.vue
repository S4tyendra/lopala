<script setup lang="ts">
import { ref, computed, nextTick, watch } from 'vue'
import { chats, channels, users, myName, myId, wsSend } from '../../composables/useWs'

const props = defineProps<{ winId: string; channel: string }>()
const emit = defineEmits<{ (e: 'openChannel', channel: string): void }>()

const input = ref('')
const chatEl = ref<HTMLElement | null>(null)
const showPicker = ref(false)

const QUICK_EMOJIS = ['🔥','💬','🎯','🚀','📢','🎨','🧪','💡','🌍','🎵','🛠️','📸','🤝','⚡','🏆','👾','🦄','🍀','🎲','🔮']

const myMessages = computed(() => chats.value.filter(c => c.channel === props.channel))

watch(myMessages, () => nextTick(() => {
  if (chatEl.value) chatEl.value.scrollTop = chatEl.value.scrollHeight
}), { immediate: true })

const send = () => {
  const content = input.value.trim()
  if (!content) return
  wsSend({ type: 'SendChat', channel: props.channel, content, user_name: myName.value })
  input.value = ''
}

const createChannel = (emoji: string) => {
  wsSend({ type: 'CreateChannel', name: emoji, created_by: myName.value })
  showPicker.value = false
}

const activeChannel = computed(() => channels.value.find(c => c.id === props.channel))
const myUser = computed(() => Object.values(users.value).find(u => u.id === myId.value))
</script>

<template>
  <div class="absolute inset-0 flex" style="background:rgba(15,15,20,0.8)">

    <!-- ── Slim channel sidebar (52px) ── -->
    <div class="flex flex-col items-center pt-2 pb-2 gap-1.5 shrink-0"
      style="width:52px;background:rgba(0,0,0,0.35);border-right:1px solid rgba(255,255,255,0.05)">

      <!-- Channel buttons: emoji only, circular -->
      <button
        v-for="ch in channels" :key="ch.id"
        @click="emit('openChannel', ch.id)"
        :title="ch.id"
        class="w-9 h-9 rounded-[12px] flex items-center justify-center text-[20px] cursor-pointer transition-all duration-150"
        :style="channel === ch.id
          ? 'background:rgba(96,165,250,0.25);box-shadow:0 0 0 1.5px #60a5fa;'
          : 'background:rgba(255,255,255,0.04)'"
        style="font-family:'SamsungOneUI','Noto Color Emoji',emoji">
        {{ ch.name }}
      </button>

      <div class="w-6 h-px" style="background:rgba(255,255,255,0.08);margin:2px 0"/>

      <!-- Add channel with emoji picker -->
      <div class="relative">
        <button @click.stop="showPicker = !showPicker"
          class="w-9 h-9 rounded-[12px] flex items-center justify-center text-[16px] font-light cursor-pointer transition-all duration-150"
          style="background:rgba(255,255,255,0.05);color:rgba(255,255,255,0.4)"
          title="New channel">
          +
        </button>

        <!-- Emoji picker popup -->
        <div v-if="showPicker"
          @click.stop
          class="absolute z-50 p-2 rounded-2xl grid gap-1 shadow-2xl"
          style="left:52px;top:0;width:188px;grid-template-columns:repeat(5,1fr);background:rgba(30,30,36,0.98);border:1px solid rgba(255,255,255,0.1);backdrop-filter:blur(16px)">
          <button
            v-for="emoji in QUICK_EMOJIS" :key="emoji"
            @click="createChannel(emoji)"
            class="w-8 h-8 rounded-lg flex items-center justify-center text-[18px] cursor-pointer transition-[background] duration-100 hover:brightness-125"
            style="background:rgba(255,255,255,0.04);font-family:'SamsungOneUI','Noto Color Emoji',emoji">
            {{ emoji }}
          </button>
        </div>
      </div>

      <!-- Spacer -->
      <div class="flex-1"/>

      <!-- Online users as color dots -->
      <div v-for="u in Object.values(users)" :key="u.id"
        :title="u.name"
        class="w-2.5 h-2.5 rounded-full"
        :style="{ background: u.color }"/>
    </div>

    <!-- ── Chat area ── -->
    <div class="flex-1 flex flex-col overflow-hidden min-w-0">

      <!-- Header: channel name + online count -->
      <div class="h-9 flex items-center gap-2 px-3 shrink-0 border-b" style="border-color:rgba(255,255,255,0.05);background:rgba(0,0,0,0.12)">
        <span class="text-[19px]" style="font-family:'SamsungOneUI','Noto Color Emoji',emoji; line-height:1">{{ activeChannel?.name ?? '#' }}</span>
        <span class="text-[11px]" style="color:rgba(255,255,255,0.25)">{{ Object.values(users).length }} online</span>
      </div>

      <!-- Messages -->
      <div ref="chatEl" class="flex-1 overflow-y-auto p-3 flex flex-col gap-2 min-w-0">
        <div v-for="msg in myMessages" :key="msg.id"
          :class="['flex flex-col min-w-0', msg.user_name === myName ? 'items-end' : 'items-start']"
          style="animation:popIn 180ms cubic-bezier(0.175,0.885,0.32,1.1) both">
          <span class="text-[9px] px-1 mb-0.5" style="color:rgba(255,255,255,0.22)">{{ msg.user_name }}</span>
          <div class="px-3 py-2 rounded-2xl text-[13px] leading-relaxed max-w-[80%] break-words"
            :class="msg.user_name === myName ? 'rounded-br-[4px]' : 'rounded-bl-[4px]'"
            :style="msg.user_name === myName
              ? 'background:#0a84ff;color:white'
              : 'background:rgba(255,255,255,0.08);color:rgba(255,255,255,0.88)'">
            {{ msg.content }}
          </div>
        </div>
      </div>

      <!-- Input -->
      <div class="px-3 py-2.5 shrink-0 border-t" style="border-color:rgba(255,255,255,0.05);background:rgba(0,0,0,0.12)">
        <input v-model="input" @keyup.enter="send"
          :placeholder="`Message ${activeChannel?.name ?? '#'}…`"
          class="w-full rounded-2xl px-4 py-2 text-[13px] outline-none border transition-shadow duration-150"
          style="background:rgba(255,255,255,0.06);border-color:rgba(255,255,255,0.09);color:white"
          @focus="(e) => (e.target as HTMLInputElement).style.boxShadow='0 0 0 2px rgba(96,165,250,0.4)'"
          @blur="(e) => (e.target as HTMLInputElement).style.boxShadow=''"
        />
      </div>
    </div>
  </div>
</template>
