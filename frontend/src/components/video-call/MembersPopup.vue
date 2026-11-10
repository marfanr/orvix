// SPDX-FileCopyrightText: 2026 Mohammad Arfan
//
// SPDX-License-Identifier: AGPL-3.0

<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { createFocusTrap, type FocusTrap } from 'focus-trap'
import { MicOffIcon, VideoOffIcon } from 'lucide-vue-next'
import { useMembersPanel } from '@/composables/useMembersPanel'
import PopUpComponent from '@/components/ui/PopUpComponent.vue'

const { participants, isOpen, close } = useMembersPanel()

const dialogEl = ref<HTMLElement | null>(null)
let trap: FocusTrap | null = null

watch(isOpen, async (open) => {
  if (open) {
    await nextTick()
    if (dialogEl.value) {
      trap = createFocusTrap(dialogEl.value, { returnFocusOnDeactivate: true })
      trap.activate()
    }
  } else {
    trap?.deactivate()
    trap = null
  }
})

onUnmounted(() => {
  trap?.deactivate()
  trap = null
})

const count = computed(() => participants.value.length)

function initials(name: string) {
  return name[0]?.toUpperCase() ?? '?'
}

function onDocKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape') close()
}

onMounted(() => {
  document.addEventListener('keydown', onDocKeydown)
})

onUnmounted(() => {
  document.removeEventListener('keydown', onDocKeydown)
})
</script>

<template>
  <PopUpComponent :is-open="isOpen" @close="close()">
    <template #title>
      Participants
      <span
        class="px-2 py-0.5 rounded-full bg-primary/10 text-xs font-medium text-primary tabular-nums"
      >
        {{ count }}
      </span>
    </template>

    <ul class="flex-1 min-h-0 overflow-y-auto list-none p-3 flex flex-col gap-y-1 chat-scroll">
      <li
        v-for="p in participants"
        :key="p.id"
        class="flex items-center gap-x-3 rounded-xl px-3 py-2.5 hover:bg-gray-50 transition"
      >
        <span
          class="w-10 h-10 shrink-0 rounded-full bg-primary flex items-center justify-center text-sm font-medium text-white"
          aria-hidden="true"
        >
          {{ initials(p.name) }}
        </span>

        <span class="min-w-0 flex-1 flex flex-col">
          <span class="text-[13px] font-medium text-gray-800 truncate">
            {{ p.name }}
          </span>
        </span>

        <span
          v-if="!p.conn_id"
          class="shrink-0 px-2 py-0.5 rounded-full bg-primary/10 text-[10px] font-semibold text-primary"
        >
          You
        </span>

        <span class="shrink-0 flex items-center gap-x-1 text-gray-400">
          <MicOffIcon v-if="p.muted" class="w-4 h-4" aria-label="Microphone off" />
          <VideoOffIcon v-if="p.cameraOff" class="w-4 h-4" aria-label="Camera off" />
        </span>
      </li>
    </ul>
  </PopUpComponent>
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
