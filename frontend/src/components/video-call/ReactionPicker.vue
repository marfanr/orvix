<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { SmilePlusIcon } from 'lucide-vue-next'
import type { EmojiExt } from 'vue3-emoji-picker'
import EmojiPicker from 'vue3-emoji-picker'
import 'vue3-emoji-picker/css'

const emit = defineEmits<{
  select: [emoji: string]
}>()

const props = withDefaults(
  defineProps<{
    variant?: 'icon' | 'menu'
  }>(),
  {
    variant: 'icon',
  },
)

const PICKER_GAP = 12
const PICKER_W = 325
const PICKER_H = 450
const PICKER_MIN_H = 160
const EDGE = 8

const pickerOpen = ref(false)
const triggerRef = ref<HTMLElement | null>(null)
const panelRef = ref<HTMLElement | null>(null)
const isDesktop = ref(false)
const anchorStyle = ref<Record<string, string>>({})
const pickerHeight = ref(PICKER_H)

let mql: MediaQueryList | null = null

const onMqlChange = (e: MediaQueryListEvent) => {
  isDesktop.value = e.matches
}

const updatePosition = () => {
  const el = triggerRef.value
  if (!el) return
  const r = el.getBoundingClientRect()

  const width = Math.min(PICKER_W, window.innerWidth - EDGE * 2)
  const centered = r.left + r.width / 2 - width / 2
  const left = Math.min(Math.max(centered, EDGE), window.innerWidth - width - EDGE)

  anchorStyle.value = {
    left: `${left}px`,
    width: `${width}px`,
    bottom: `${window.innerHeight - r.top + PICKER_GAP}px`,
  }

  pickerHeight.value = Math.max(Math.min(PICKER_H, r.top - PICKER_GAP - EDGE), PICKER_MIN_H)
}

const panelStyle = computed(() => (isDesktop.value ? anchorStyle.value : undefined))
const pickerStyle = computed(() => ({ '--picker-h': `${pickerHeight.value}px` }))

const toggle = () => {
  if (!pickerOpen.value) updatePosition()
  pickerOpen.value = !pickerOpen.value
}

const close = () => {
  pickerOpen.value = false
}

const pickEmoji = (selected: EmojiExt) => {
  emit('select', selected.i)
  close()
}

const onDocPointerDown = (e: PointerEvent) => {
  if (!pickerOpen.value) return
  const target = e.target as Node
  if (triggerRef.value?.contains(target)) return
  if (panelRef.value?.contains(target)) return
  close()
}

const onDocKeydown = (e: KeyboardEvent) => {
  if (e.key === 'Escape') close()
}

onMounted(() => {
  mql = window.matchMedia('(min-width: 640px)')
  isDesktop.value = mql.matches
  mql.addEventListener('change', onMqlChange)
  document.addEventListener('pointerdown', onDocPointerDown)
  document.addEventListener('keydown', onDocKeydown)
  window.addEventListener('resize', updatePosition)
})

onUnmounted(() => {
  mql?.removeEventListener('change', onMqlChange)
  document.removeEventListener('pointerdown', onDocPointerDown)
  document.removeEventListener('keydown', onDocKeydown)
  window.removeEventListener('resize', updatePosition)
})
</script>

<template>
  <div ref="triggerRef" class="relative shrink-0">
    <button
      type="button"
      aria-label="Reactions"
      title="Reactions"
      aria-haspopup="true"
      :aria-expanded="pickerOpen"
      @click="toggle"
      class="cursor-pointer touch-manipulation focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
      :class="[
        props.variant === 'menu'
          ? 'flex min-h-[44px] w-full items-center gap-x-3 rounded-lg px-3 py-2.5 text-left text-sm font-medium text-gray-700 transition active:bg-gray-100 sm:hover:bg-gray-100'
          : 'flex min-h-[44px] min-w-[44px] items-center justify-center rounded-full bg-gray-100 p-1.5 text-gray-700 transition hover:bg-gray-200 active:bg-gray-200 sm:p-2',
        pickerOpen ? (props.variant === 'menu' ? 'bg-gray-100' : 'bg-gray-200') : '',
      ]"
    >
      <SmilePlusIcon
        :class="props.variant === 'menu' ? 'size-5 shrink-0' : 'h-5 w-5 sm:h-6 sm:w-6'"
        aria-hidden="true"
      />
      <span v-if="props.variant === 'menu'">Reactions</span>
    </button>

    <Teleport to="body">
      <div
        v-if="pickerOpen"
        ref="panelRef"
        data-reaction-picker
        class="fixed inset-0 z-[9999] flex h-dvh flex-col bg-white sm:inset-auto sm:block sm:h-auto sm:bg-transparent"
        :style="panelStyle"
      >
        <!-- header mobile -->
        <div
          class="flex shrink-0 items-center justify-between border-b border-gray-200 bg-white px-4 pb-3 pt-[max(0.75rem,env(safe-area-inset-top))] sm:hidden"
        >
          <span class="text-base font-medium text-gray-800">Reactions</span>
          <button
            type="button"
            aria-label="Close"
            class="min-h-[44px] rounded-lg px-3 py-1.5 text-sm font-medium text-gray-700 active:bg-gray-100 touch-manipulation"
            @click="close"
          >
            Close
          </button>
        </div>

        <div
          class="min-h-0 flex-1 bg-white pb-[env(safe-area-inset-bottom)] sm:bg-transparent sm:pb-0"
        >
          <EmojiPicker
            :native="true"
            theme="light"
            :hide-search="false"
            :style="pickerStyle"
            @select="pickEmoji"
            class="!h-full !w-full !max-w-none !rounded-none !shadow-none sm:!h-[var(--picker-h)] sm:!w-full sm:!rounded-lg sm:!shadow-xl"
          />
        </div>
      </div>
    </Teleport>
  </div>
</template>
