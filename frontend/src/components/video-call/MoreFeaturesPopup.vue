<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref, watch, type Component } from 'vue'
import { createFocusTrap, type FocusTrap } from 'focus-trap'
import {
  CrownIcon,
  Dice5Icon,
  GamepadIcon,
  ListChecksIcon,
  MusicIcon,
  PenToolIcon,
  PuzzleIcon,
  TimerIcon,
  XIcon,
} from 'lucide-vue-next'
import { useMoreFeaturesPanel } from '@/composables/useMoreFeaturesPanel'

const { isOpen, close } = useMoreFeaturesPanel()

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

interface FeatureItem {
  key: string
  label: string
  icon: Component
  enabled: boolean
}

const features: FeatureItem[] = [
  { key: 'music', label: 'Play Music', icon: MusicIcon, enabled: false },
  { key: 'chess', label: 'Chess Game', icon: CrownIcon, enabled: false },
  { key: 'dice', label: 'Dice Roll', icon: Dice5Icon, enabled: false },
  { key: 'whiteboard', label: 'Whiteboard', icon: PenToolIcon, enabled: false },
  { key: 'poll', label: 'Polling', icon: ListChecksIcon, enabled: false },
  { key: 'timer', label: 'Shared Timer', icon: TimerIcon, enabled: false },
  { key: 'minigames', label: 'Mini Games', icon: GamepadIcon, enabled: false },
  { key: 'puzzle', label: 'Puzzle', icon: PuzzleIcon, enabled: false },
]

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
  <Teleport to="body">
    <Transition
      enter-active-class="transition duration-200 ease-out"
      enter-from-class="opacity-0"
      enter-to-class="opacity-100"
      leave-active-class="transition duration-150 ease-in"
      leave-from-class="opacity-100"
      leave-to-class="opacity-0"
    >
      <div
        v-if="isOpen"
        class="fixed inset-0 z-45 flex items-stretch justify-center lg:items-center lg:justify-center lg:p-6"
      >
        <div
          class="absolute inset-0 hidden bg-black/40 backdrop-blur-[2px] lg:block"
          aria-hidden="true"
          @click="close"
        />

        <div
          ref="dialogEl"
          class="relative flex w-full h-full flex-col overflow-hidden bg-white lg:h-[560px] lg:max-h-[85vh] lg:w-[640px] lg:rounded-lg lg:border lg:border-gray-200 lg:shadow-2xl"
          role="dialog"
          aria-modal="true"
          aria-labelledby="more-features-title"
        >
          <header class="flex items-center justify-between px-5 py-4 border-b border-gray-200">
            <h2 id="more-features-title" class="text-lg font-medium text-gray-900">More features</h2>
            <button
              type="button"
              @click="close"
              class="grid size-8 place-items-center rounded-full text-gray-500 transition hover:bg-gray-100 hover:text-gray-900 focus-visible:outline-2 focus-visible:outline-primary"
              aria-label="Close more features"
            >
              <XIcon class="w-5 h-5" />
            </button>
          </header>

          <div class="flex-1 min-h-0 overflow-y-auto p-4 sm:p-5 chat-scroll">
            <ul
              class="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 gap-3 list-none"
              role="list"
            >
              <li v-for="f in features" :key="f.key">
                <button
                  type="button"
                  :disabled="!f.enabled"
                  :aria-label="f.enabled ? f.label : `${f.label} (coming soon)`"
                  :title="f.enabled ? f.label : `${f.label} (coming soon)`"
                  class="group relative flex w-full flex-col items-center justify-center gap-x-2 gap-y-2 rounded-lg border border-gray-200 bg-white px-3 py-6 text-center transition focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary touch-manipulation"
                  :class="
                    f.enabled
                      ? 'cursor-pointer text-gray-800 hover:border-primary hover:bg-primary/5'
                      : 'cursor-not-allowed text-gray-400 opacity-70'
                  "
                >
                  <component
                    :is="f.icon"
                    class="size-7"
                    :class="f.enabled ? 'text-primary' : 'text-gray-400'"
                  />
                  <span class="text-xs font-medium leading-tight">{{ f.label }}</span>
                  <span
                    v-if="!f.enabled"
                    class="absolute top-2 right-2 rounded-full bg-gray-100 px-1.5 py-0.5 text-[9px] font-medium uppercase tracking-wide text-gray-500"
                  >
                    Soon
                  </span>
                </button>
              </li>
            </ul>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
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
