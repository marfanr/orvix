<script setup lang="ts">
import { ref, watch, onBeforeUnmount } from 'vue'
import type { NewMessageEvent } from '@/types/video-call'

const props = withDefaults(
  defineProps<{
    message?: NewMessageEvent
    duration?: number
  }>(),
  { duration: 3000 },
)

const emit = defineEmits<{
  resolve: []
}>()

const visible = ref(false)
let timer: ReturnType<typeof setTimeout> | undefined

function clearTimer() {
  if (timer !== undefined) {
    clearTimeout(timer)
    timer = undefined
  }
}

watch(
  () => props.message?.eventId,
  (eventId) => {
    clearTimer()
    if (eventId === undefined) {
      visible.value = false
      return
    }
    visible.value = true
    timer = setTimeout(() => {
      timer = undefined
      visible.value = false
      emit('resolve')
    }, props.duration)
  },
  { immediate: true },
)

onBeforeUnmount(clearTimer)
</script>

<template>
  <Teleport to="body">
    <Transition
      mode="out-in"
      enter-active-class="transition duration-200 ease-out"
      enter-from-class="translate-y-2 opacity-0"
      enter-to-class="translate-y-0 opacity-100"
      leave-active-class="transition duration-150 ease-in"
      leave-from-class="translate-y-0 opacity-100"
      leave-to-class="translate-y-2 opacity-0"
    >
      <div
        v-if="visible && message"
        :key="message.eventId"
        class="pointer-events-none fixed bottom-20 md:bottom-24 inset-x-0 z-99 flex justify-center"
      >
        <div
          role="status"
          aria-live="polite"
          class="pointer-events-auto w-80 max-w-[calc(100vw-2rem)] rounded-xl border border-gray-200/80 bg-white p-4 shadow-xl ring-1 ring-black/5"
        >
          <div class="flex items-center gap-3">
            <div
              class="flex size-10 shrink-0 items-center justify-center rounded-full bg-indigo-50 text-sm font-semibold text-brown-ribbon-600 ring-1 ring-indigo-100"
            >
              {{ message.sender[0]?.toUpperCase() }}
            </div>

            <div class="min-w-0 flex-1">
              <p class="truncate text-sm font-medium text-shadow-brown-ribbon-800">
                {{ message.sender }}
              </p>
              <p class="mt-0.5 truncate text-sm text-gray-500">{{ message.text }}</p>
            </div>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>
