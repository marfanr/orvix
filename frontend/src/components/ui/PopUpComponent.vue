<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import { XIcon } from 'lucide-vue-next'
import { PopUpWidth } from '@/types/video-call.ts'

const contentEl = ref<HTMLElement | null>(null)

defineExpose({ contentEl })

const widthToClass = (w?: PopUpWidth): string => {
  switch (w) {
    case PopUpWidth.MEDIUM:
      return 'lg:w-[560px]'
    case PopUpWidth.LARGE:
      return 'lg:w-10/11'
  }
  return 'lg:w-[660px]'
}

const props = defineProps<{
  isOpen: boolean
  width?: PopUpWidth
}>()

const emits = defineEmits<{
  close: []
}>()

const onKeydown = (e: KeyboardEvent) => {
  if (e.key !== 'Escape') return
  else if (props.isOpen) emits('close')
}
onMounted(() => document.addEventListener('keydown', onKeydown))
onUnmounted(() => {
  document.removeEventListener('keydown', onKeydown)
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
        v-if="props.isOpen"
        class="fixed inset-0 z-45 flex items-stretch justify-center lg:items-center lg:p-6"
      >
        <div
          class="absolute inset-0 hidden bg-black/40 backdrop-blur-[2px] lg:block"
          aria-hidden="true"
          @click="emits('close')"
        />

        <div
          ref="contentEl"
          class="relative flex h-full w-full flex-col overflow-hidden bg-white lg:h-[520px] lg:max-h-[85vh] lg:rounded-lg lg:border lg:border-gray-200 lg:shadow-2xl z-55"
          :class="widthToClass(props.width)"
          role="dialog"
          aria-modal="true"
          aria-labelledby="members-title"
        >
          <header class="flex items-center justify-between border-b border-gray-200 px-5 py-4">
            <h2
              id="members-title"
              class="flex items-center gap-x-2 text-lg font-medium text-gray-900"
            >
              <slot name="title">Popup</slot>
            </h2>

            <button
              type="button"
              @click="emits('close')"
              class="grid size-8 place-items-center rounded-full text-gray-500 transition hover:bg-gray-100 hover:text-gray-900 focus-visible:outline-2 focus-visible:outline-primary"
              aria-label="Close popup"
            >
              <XIcon class="h-5 w-5" />
            </button>
          </header>

          <slot />
        </div>
      </div>
    </Transition>
  </Teleport>
</template>
