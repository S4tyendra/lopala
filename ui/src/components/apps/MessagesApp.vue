<script setup lang="ts">
import { ref, computed, nextTick, watch } from 'vue'
import { chats, channels, users, myName, myId, wsSend } from '../../composables/useWs'

const props = defineProps<{ winId: string; channel: string }>()
const emit = defineEmits<{ (e: 'openChannel', channel: string): void }>()

const input = ref('')
const chatEl = ref<HTMLElement | null>(null)
const showEmojiPicker = ref(false)

// Common emojis for channel creation
const QUICK_EMOJIS = ['🔥','💬','🎯','🚀','📢','🎨','🧪','💡','🌍','🎵','🛠️','📸','🤝','⚡','🏆']

const myMessages = computed(() => chats.value.filter(c => c.channel === props.channel))

watch(myMessages, () => {
  nextTick(() => {
    if (chatEl.value) chatEl.value.scrollTop = chatEl.value.scrollHeight
  })
})

const send = () => {
  const content = input.value.trim()
  if (!content) return
  wsSend({ type: 'SendChat', channel: props.channel, content, user_name: myName.value })
  input.value = ''
}

const createChannel = (emoji: string) => {
  wsSend({ type: 'CreateChannel', name: emoji, created_by: myName.value })
  showEmojiPicker.value = false
}

const currentChannel = computed(() => channels.value.find(c => c.id === props.channel))
</script>

<template>
  <div class="absolute inset-0 flex" style="background:rgba(15,15,15,0.7)">
    <!-- Channel Sidebar -->
    <div class="w-16 flex flex-col items-center py-3 gap-1.5 shrink-0 border-r" style="background:rgba(0,0,0,0.3);border-color:rgba(255,255,255,0.05)">
      <!-- Channel buttons (emoji only) -->
      <button v-for="ch in channels" :key="ch.id"
        @click="emit('openChannel', ch.id)"
        :title="ch.name"
        :style="channel === ch.id
          ? 'background:rgba(10,132,255,0.35);color:white;box-shadow:0 0 0 2px #0a84ff'
          : 'color:rgba(255,255,255,0.6)'"
        class="w-10 h-10 rounded-2xl flex items-center justify-center text-[22px] cursor-pointer transition-all duration-150 hover:brightness-125"
        style="font-family:'SamsungColorEmoji','Noto Color Emoji',emoji">
        {{ ch.name }}
      </button>

      <!-- Divider -->
      <div class="w-6 h-px my-1" style="background:rgba(255,255,255,0.1)"></div>

      <!-- Add channel -->
      <div class="relative">
        <button @click="showEmojiPicker = !showEmojiPicker"
          class="w-10 h-10 rounded-2xl flex items-center justify-center text-[20px] cursor-pointer transition-all duration-150 hover:brightness-125"
          style="background:rgba(255,255,255,0.06);color:rgba(255,255,255,0.4)">+</button>

        <!-- Emoji Picker Popup -->
        <div v-if="showEmojiPicker"
          class="absolute left-12 top-0 z-50 p-2 rounded-xl grid grid-cols-5 gap-1 shadow-2xl"
          style="background:rgba(35,35,40,0.97);border:1px solid rgba(255,255,255,0.12);backdrop-filter:blur(12px);width:180px">
          <button v-for="emoji in QUICK_EMOJIS" :key="emoji"
            @click="createChannel(emoji)"
            class="w-8 h-8 rounded-lg flex items-center justify-center text-[18px] cursor-pointer transition-[background] duration-100 hover:brightness-125"
            style="font-family:'SamsungColorEmoji','Noto Color Emoji',emoji;background:rgba(255,255,255,0.05)">
            {{ emoji }}
          </button>
        </div>
      </div>

      <!-- Online users as colored dots at bottom -->
      <div class="mt-auto flex flex-col items-center gap-1.5 pb-1">
        <div v-for="u in Object.values(users)" :key="u.id"
          :title="u.name"
          class="w-3 h-3 rounded-full border border-black/30"
          :style="{ background: u.color }"></div>
      </div>
    </div>

    <!-- Chat Area -->
    <div class="flex-1 flex flex-col overflow-hidden">
      <!-- Channel header -->
      <div class="h-9 flex items-center px-4 shrink-0 border-b" style="border-color:rgba(255,255,255,0.05);background:rgba(0,0,0,0.1)">
        <span class="text-[18px] mr-2" style="font-family:'SamsungColorEmoji','Noto Color Emoji',emoji">{{ currentChannel?.name ?? '#' }}</span>
        <span class="text-[12px]" style="color:rgba(255,255,255,0.35)">{{ Object.values(users).length }} online</span>
      </div>

      <!-- Messages -->
      <div ref="chatEl" class="flex-1 overflow-y-auto p-4 flex flex-col gap-2">
        <div v-for="msg in myMessages" :key="msg.id"
          :class="['flex flex-col', msg.user_name === myName ? 'items-end' : 'items-start']"
          style="animation: popIn 200ms cubic-bezier(0.175,0.885,0.32,1.1) both;">
          <span class="text-[9px] mb-0.5 px-1" style="color:rgba(255,255,255,0.25)">{{ msg.user_name }}</span>
          <div :class="['px-3 py-2 rounded-2xl text-[13px] leading-relaxed max-w-[75%]',
            msg.user_name === myName ? 'rounded-br-[4px]' : 'rounded-bl-[4px]']"
            :style="msg.user_name === myName
              ? 'background:#0a84ff;color:white'
              : 'background:rgba(255,255,255,0.09);color:rgba(255,255,255,0.88)'">
            {{ msg.content }}
          </div>
        </div>
      </div>

      <!-- Input -->
      <div class="px-3 pb-3 pt-2 shrink-0" style="background:rgba(0,0,0,0.15)">
        <input v-model="input" @keyup.enter="send"
          :placeholder="`Message ${currentChannel?.name ?? '#'}…`"
          class="w-full rounded-2xl px-4 py-2 text-[13px] outline-none transition-[box-shadow] duration-150"
          style="background:rgba(255,255,255,0.07);border:1px solid rgba(255,255,255,0.1);color:white"
          @focus="(e) => (e.target as HTMLInputElement).style.boxShadow='0 0 0 2px #0a84ff55'"
          @blur="(e) => (e.target as HTMLInputElement).style.boxShadow=''"
        />
      </div>
    </div>
  </div>
</template>
