<script setup lang="ts">
import { ChevronDownIcon, CircleHelpIcon } from 'lucide-vue-next'
import { computed, ref } from 'vue'

interface SelectOption {
  value: string
  label: string
}

defineProps<{
  id: string
  label: string
  options: SelectOption[]
  hint?: string
  help?: string
}>()

const value = defineModel<string>({ required: true })

const helpHovered = ref(false)
const helpPinned = ref(false)
const helpOpen = computed(() => helpHovered.value || helpPinned.value)
</script>

<template>
  <div class="flex flex-col gap-y-1.5">
    <label
      :for="id"
      class="flex items-center gap-1.5 text-xs font-semibold tracking-wide text-gray-500 uppercase"
    >
      <span>{{ label }}</span>

      <span v-if="help" class="relative inline-flex normal-case">
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
          class="pointer-events-none absolute bottom-full left-1/2 z-20 mb-2 w-56 -translate-x-1/2 rounded-lg bg-gray-900 px-3 py-2 text-xs leading-relaxed font-normal break-words text-white normal-case shadow-lg"
        >
          {{ help }}
        </span>
      </span>
    </label>

    <div class="relative">
      <select
        :id="id"
        v-model="value"
        class="h-10 w-full select-custom rounded-lg border border-gray-200 bg-white pr-9 pl-3 text-sm text-gray-900 transition focus:border-primary focus:ring-2 focus:ring-primary/20 focus:outline-none"
      >
        <option v-for="option in options" :key="option.value" :value="option.value">
          {{ option.label }}
        </option>
      </select>
      <ChevronDownIcon
        class="pointer-events-none absolute top-1/2 right-3 size-4 -translate-y-1/2 text-gray-400"
        aria-hidden="true"
      />
    </div>

    <p v-if="hint" class="text-xs text-gray-500">{{ hint }}</p>
  </div>
</template>

<style>
.select-custom {
  text-align: center;
  align-items: center;
}
.select-custom,
.select-custom::picker(select) {
  appearance: base-select;
}

.select-custom::picker(select) {
  margin-top: 9px;
  padding: 4px;
  border: 1px solid var(--color-gray-200);
  border-radius: var(--radius-lg);
  background: white;
  box-shadow: 0 8px 24px rgb(0 0 0 / 0.1);
}

.select-custom option {
  padding: 8px 12px;
  border-radius: var(--radius-md);
  font-size: var(--text-sm);
  cursor: pointer;

}

.select-custom option:hover,
.select-custom option:focus-visible {
  background: var(--color-gray-100); /* ganti warna hover di sini */
}

.select-custom option:checked {
  background: var(--color-primary);
  font-weight: 600;
}

.select-custom::picker-icon {
  visibility: hidden;
}
</style>