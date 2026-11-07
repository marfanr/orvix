<script setup lang="ts">
import { CircleHelpIcon, CheckIcon } from 'lucide-vue-next'
import { computed, ref } from 'vue'

const props = defineProps<{
  label: string
  description: string
  help?: string
  disabled?: boolean
}>()

const enabled = defineModel<boolean>({ required: true })

const helpHovered = ref(false)
const helpPinned = ref(false)
const helpOpen = computed(() => helpHovered.value || helpPinned.value)
</script>

<template>
  <label class="flex items-center justify-between gap-4 px-4 py-3"
  :class="props.disabled ? 'cursor-not-allowed bg-gray-300' : 'cursor-pointer'">
    <span class="min-w-0">
      <span class="flex items-center gap-1.5">
        <span class="block text-sm font-medium text-gray-800">{{ label }}</span>

        <span v-if="help" class="relative inline-flex">
          <button
            type="button"
            class="grid size-4 cursor-help place-items-center rounded-full text-gray-400 transition hover:text-gray-600 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
            :aria-label="`Bantuan: ${label}`"
            :aria-expanded="helpOpen"
            @click.prevent.stop="helpPinned = !helpPinned"
            @keydown.escape="helpPinned = false"
            @mouseenter="helpHovered = true"
            @mouseleave="helpHovered = false"
            @focus="helpHovered = true"
            @blur="helpHovered = false"
          >
            <CircleHelpIcon class="size-3.5" aria-hidden="true" />
          </button>

          <span
            v-show="helpOpen"
            role="tooltip"
            class="pointer-events-none absolute bottom-full left-1/2 z-20 mb-2 w-56 -translate-x-1/2 rounded-lg bg-gray-900 px-3 py-2 text-xs leading-relaxed text-white shadow-lg"
          >
            {{ help }}
          </span>
        </span>
      </span>

      <span class="mt-0.5 block text-xs text-gray-500">{{ description }}</span>
    </span>

    <span class="relative inline-flex shrink-0">
      <input v-model="enabled" type="checkbox" role="switch" class="peer sr-only" :disabled="props.disabled"/>
      <span
        class="h-5 w-9 rounded-full bg-gray-300 transition peer-checked:bg-primary peer-focus-visible:outline-2 peer-focus-visible:outline-offset-2 peer-focus-visible:outline-primary"
      ></span>
      <span
        class="absolute top-0.5 left-0.5 grid size-4 place-items-center rounded-full  shadow transition peer-checked:translate-x-4 peer-checked:[&_.toggle-check]:opacity-100"
        :class="props.disabled ? 'bg-gray-400' : 'bg-white'"
      >
        <CheckIcon
          class="toggle-check size-2.5 text-primary opacity-0 transition"
          aria-hidden="true"
        />
      </span>
    </span>
  </label>
</template>
