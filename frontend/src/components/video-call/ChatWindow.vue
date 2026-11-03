<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { SmilePlusIcon, XIcon } from 'lucide-vue-next'
import type { EmojiExt } from 'vue3-emoji-picker'
import EmojiPicker from 'vue3-emoji-picker'
import 'vue3-emoji-picker/css'
import { useCallStore } from '@/stores/call.ts'
import { storeToRefs } from 'pinia'

const store = useCallStore()
const { chatMessages } = storeToRefs(store)
const { closeChat } = store

const emits = defineEmits<{
  send: [msg: string, target: string]
}>()

const draft = ref('')
const messagesEl = ref<HTMLElement | null>(null)
const textareaEl = ref<HTMLTextAreaElement | null>(null)
const isAtBottom = ref(true)
const isRecipientOpen = ref(false)
const emojiOpen = ref(false)
const emojiBtnRef = ref<HTMLElement | null>(null)
const emojiPanelRef = ref<HTMLElement | null>(null)

const canSend = computed(() => draft.value.trim().length > 0)

function sendMessage() {
  const text = draft.value.trim()
  if (!text) return

  draft.value = ''
  isRecipientOpen.value = false

  emits('send', text, 'all')

  if (textareaEl.value) {
    textareaEl.value.style.height = 'auto'
    textareaEl.value.focus()
  }
}

function onKeydown(event: KeyboardEvent) {
  if (event.key === 'Enter' && !event.shiftKey) {
    event.preventDefault()
    if (canSend.value) sendMessage()
  }
}

function autoGrow() {
  const el = textareaEl.value
  if (!el) return
  el.style.height = 'auto'
  el.style.height = `${Math.min(el.scrollHeight, 128)}px`
}

function scrollToBottom(behavior: ScrollBehavior = 'auto') {
  const el = messagesEl.value
  if (!el) return
  el.scrollTo({ top: el.scrollHeight, behavior })
}

function onScroll() {
  const el = messagesEl.value
  if (!el) return
  isAtBottom.value = el.scrollHeight - el.scrollTop - el.clientHeight < 24
}

watch(
  () => chatMessages.value.length,
  async () => {
    await nextTick()
    if (isAtBottom.value) scrollToBottom('smooth')
  },
)

function onRecipientClick() {
  isRecipientOpen.value = !isRecipientOpen.value
}

function toggleEmoji() {
  emojiOpen.value = !emojiOpen.value
}

function closeEmoji() {
  emojiOpen.value = false
}

async function insertEmoji(selected: EmojiExt) {
  const emoji: string = selected.i
  const el = textareaEl.value
  if (!el) {
    draft.value += emoji
    return
  }

  const start = el.selectionStart ?? draft.value.length
  const end = el.selectionEnd ?? start
  draft.value = draft.value.slice(0, start) + emoji + draft.value.slice(end)

  await nextTick()
  el.focus()
  const caret = start + emoji.length
  el.setSelectionRange(caret, caret)
  autoGrow()
}

function onDocPointerDown(e: PointerEvent) {
  if (!emojiOpen.value) return
  const target = e.target as Node
  if (emojiBtnRef.value?.contains(target)) return
  if (emojiPanelRef.value?.contains(target)) return
  closeEmoji()
}

function onDocKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape') {
    emojiOpen.value = false
    isRecipientOpen.value = false
  }
}

onMounted(() => {
  nextTick(() => scrollToBottom())
  textareaEl.value?.focus()
  document.addEventListener('pointerdown', onDocPointerDown)
  document.addEventListener('keydown', onDocKeydown)
})

onUnmounted(() => {
  document.removeEventListener('pointerdown', onDocPointerDown)
  document.removeEventListener('keydown', onDocKeydown)
})
</script>

<template>
  <div
    class="fixed inset-0 z-40 flex flex-col overflow-hidden bg-gray-50 lg:static lg:inset-auto lg:z-auto lg:w-1/4 lg:min-w-[300px] lg:max-w-full lg:shrink-0 lg:border-l lg:border-gray-200"
  >
    <!-- Header -->
    <header class="flex items-center justify-between px-4 py-3 border-b border-gray-200">
      <h2 class="text-sm font-semibold text-gray-900 flex items-center gap-x-2">
        Chat
        <span
          v-if="chatMessages.length"
          class="px-1.5 py-0.5 rounded-full bg-gray-100 text-[10px] font-medium text-gray-500 tabular-nums"
        >
          {{ chatMessages.length }}
        </span>
      </h2>
      <button
        @click="closeChat"
        class="p-2 -mr-2 rounded-full text-gray-500 hover:bg-gray-100 hover:text-gray-700 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary transition"
        aria-label="Tutup chat"
      >
        <XIcon class="w-5 h-5" />
      </button>
    </header>
    <!-- Messages -->
    <div
      ref="messagesEl"
      id="messages"
      class="relative flex-1 overflow-y-auto px-4 py-3 flex flex-col gap-y-3 text-[13px] leading-snug chat-scroll"
      role="log"
      aria-live="polite"
      aria-label="Pesan chat"
      @scroll.passive="onScroll"
    >
      <!-- Empty state -->
      <div
        v-if="chatMessages.length === 0"
        class="flex-1 flex flex-col items-center justify-center gap-y-1 text-center px-4"
      >
        <div
          class="w-10 h-10 rounded-full bg-gray-100 flex items-center justify-center text-gray-400"
        >
          <svg
            class="w-5 h-5"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.8"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
          >
            <path
              d="M21 11.5a8.38 8.38 0 01-.9 3.8 8.5 8.5 0 01-7.6 4.7 8.38 8.38 0 01-3.8-.9L3 21l1.9-5.7a8.38 8.38 0 01-.9-3.8 8.5 8.5 0 014.7-7.6 8.38 8.38 0 013.8-.9h.5a8.48 8.48 0 018 8v.5z"
            />
          </svg>
        </div>
        <p class="text-xs font-medium text-gray-500">Belum ada pesan</p>
        <p class="text-[11px] text-gray-400">Kirim pesan ke semua peserta</p>
      </div>

      <!-- Day divider -->
      <template v-else>
        <div class="flex items-center gap-x-2 my-1" aria-hidden="true">
          <span class="h-px flex-1 bg-gray-200"></span>
          <span class="text-[10px] font-medium text-gray-400 uppercase tracking-wide"
            >Hari ini</span
          >
          <span class="h-px flex-1 bg-gray-200"></span>
        </div>

        <div
          v-for="msg in chatMessages"
          :key="msg.id"
          class="group flex flex-col"
          :class="msg.outgoing ? 'items-end' : 'items-start'"
        >
          <div
            class="flex items-baseline gap-x-2 max-w-full"
            :class="msg.outgoing && 'flex-row-reverse'"
          >
            <span class="font-semibold text-gray-800 truncate">{{ msg.sender }}</span>
            <time class="text-[10px] text-gray-400 shrink-0 tabular-nums" :datetime="msg.time">
              {{ msg.time }}
            </time>
          </div>
          <p
            class="mt-0.5 px-3 py-1.5 max-w-[85%] break-words rounded-xl"
            :class="
              msg.outgoing
                ? 'bg-primary/15 text-gray-800 '
                : 'bg-white text-gray-800 border border-gray-200 '
            "
          >
            {{ msg.text }}
          </p>
        </div>
      </template>
    </div>

    <!-- Jump to latest -->
    <Transition
      enter-active-class="transition duration-200 ease-out"
      enter-from-class="opacity-0 translate-y-1"
      enter-to-class="opacity-100 translate-y-0"
      leave-active-class="transition duration-150 ease-in"
      leave-from-class="opacity-100"
      leave-to-class="opacity-0 translate-y-1"
    >
      <button
        v-if="!isAtBottom && chatMessages.length"
        @click="scrollToBottom('smooth')"
        class="self-center -mt-11 mb-1 z-10 flex items-center gap-x-1.5 px-3 py-1.5 rounded-full bg-white border border-gray-200 shadow-sm text-xs font-medium text-gray-700 hover:bg-gray-50 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary transition"
      >
        <svg
          class="w-3.5 h-3.5"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
          aria-hidden="true"
        >
          <path d="M12 5v14m7-7l-7 7-7-7" />
        </svg>
        Ke pesan terbaru
      </button>
    </Transition>

    <!-- Composer -->
    <div
      class="px-3 pb-[max(0.75rem,env(safe-area-inset-bottom))] pt-2 border-t border-gray-200 flex flex-col gap-y-2"
    >
      <div class="relative flex items-center gap-x-2 text-xs text-gray-500">
        <span>To:</span>
        <button
          @click="onRecipientClick"
          class="flex items-center gap-x-1 px-2 py-1 rounded border border-gray-300 bg-white text-gray-700 hover:bg-gray-100 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary transition"
          aria-haspopup="listbox"
          :aria-expanded="isRecipientOpen"
        >
          Semua
          <svg
            class="w-3 h-3 transition-transform"
            :class="isRecipientOpen && 'rotate-180'"
            viewBox="0 0 20 20"
            fill="currentColor"
            aria-hidden="true"
          >
            <path d="M5.5 7.5L10 12l4.5-4.5H5.5z" />
          </svg>
        </button>

        <ul
          v-if="isRecipientOpen"
          class="absolute bottom-full left-6 mb-1.5 w-40 py-1 rounded-lg border border-gray-200 bg-white shadow-lg z-20"
          role="listbox"
          aria-label="Kirim ke"
        >
          <li
            role="option"
            aria-selected="true"
            class="px-3 py-1.5 text-xs text-gray-800 bg-primary-soft font-medium cursor-pointer"
          >
            Semua
          </li>
          <li
            role="option"
            aria-selected="false"
            class="px-3 py-1.5 text-xs text-gray-600 hover:bg-gray-50 cursor-default"
          >
            Hanya saya (pribadi)
          </li>
        </ul>
      </div>

      <div
        class="rounded-2xl border border-gray-300 bg-white transition focus-within:border-primary focus-within:ring-2 focus-within:ring-primary/20"
      >
        <textarea
          ref="textareaEl"
          id="chat-input"
          v-model="draft"
          rows="1"
          aria-label="Tulis pesan"
          placeholder="Tulis pesan..."
          class="block w-full resize-none bg-transparent px-3.5 pt-2.5 pb-1 text-base sm:text-[13px] text-gray-800 placeholder:text-gray-400 outline-none max-h-32 overflow-y-auto"
          @keydown="onKeydown"
          @input="autoGrow"
        ></textarea>

        <div class="flex items-center justify-between px-2 pb-1.5">
          <div ref="emojiBtnRef" class="relative flex items-center gap-x-0.5 text-gray-400">
            <button
              @click="toggleEmoji"
              class="p-2 rounded-full hover:bg-gray-100 hover:text-gray-600 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary transition"
              aria-label="Pilih emoji"
              title="Emoji"
              aria-haspopup="dialog"
              :aria-expanded="emojiOpen"
              :class="emojiOpen && 'bg-gray-100 text-gray-600'"
            >
              <SmilePlusIcon class="w-[18px] h-[18px]" />
            </button>
            <button
              class="p-2 rounded-full hover:bg-gray-100 hover:text-gray-600 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary transition"
              aria-label="Lampirkan file"
            >
              <svg
                class="w-[18px] h-[18px]"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="1.8"
                stroke-linecap="round"
                stroke-linejoin="round"
                aria-hidden="true"
              >
                <path
                  d="M21 11.5l-8.5 8.5a5 5 0 01-7-7L14 4.5a3.5 3.5 0 015 5L10.5 18a2 2 0 01-3-3L15 7.5"
                />
              </svg>
            </button>

            <Teleport to="body">
              <div
                v-if="emojiOpen"
                ref="emojiPanelRef"
                class="fixed inset-x-2 bottom-2 z-50 rounded-lg border border-gray-200 bg-white shadow-xl overflow-hidden sm:inset-x-auto sm:right-3 sm:w-[357px] sm:bottom-24"
                role="dialog"
                aria-label="Pilih emoji"
              >
                <EmojiPicker
                  :native="true"
                  theme="light"
                  :hide-search="true"
                  :style="{ '--picker-h': '320px' }"
                  class="!h-[var(--picker-h)] !w-full !max-w-none !shadow-none !rounded-none"
                  @select="insertEmoji"
                />
              </div>
            </Teleport>
          </div>

          <span v-if="canSend" class="text-[10px] text-gray-400" aria-hidden="true">Enter</span>
          <button
            id="send-btn"
            :disabled="!canSend"
            @click="sendMessage"
            class="p-2 rounded-full text-white bg-primary hover:bg-primary-hover focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary transition disabled:bg-gray-200 disabled:text-gray-400 disabled:cursor-not-allowed"
            aria-label="Kirim"
          >
            <svg class="w-4 h-4" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
              <path
                d="M3.4 20.4l17.5-7.5a1 1 0 000-1.8L3.4 3.6a1 1 0 00-1.4 1.2L4 11l9 1-9 1-2 6.2a1 1 0 001.4 1.2z"
              />
            </svg>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.chat-scroll {
  scrollbar-width: thin;
  scrollbar-color: rgb(209 213 219) transparent;
}

.chat-scroll::-webkit-scrollbar {
  width: 6px;
}

.chat-scroll::-webkit-scrollbar-thumb {
  border-radius: 9999px;
  background-color: rgb(209 213 219);
}

.chat-scroll::-webkit-scrollbar-track {
  background: transparent;
}
</style>
