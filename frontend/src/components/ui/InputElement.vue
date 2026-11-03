<script setup lang="ts">
import { computed, ref, useId } from 'vue'

interface Props {
  label: string
  type?: string
  placeholder?: string
  autocomplete?: string
  error?: string
  id?: string
  hideLabel?: boolean
  hint?: string
  monospace?: boolean
  min?: string
}
const showPassword = ref(false)
const props = withDefaults(defineProps<Props>(), {
  type: 'text',
  placeholder: '',
  autocomplete: undefined,
  error: '',
  id: undefined,
  hideLabel: true,
  hint: '',
  monospace: false,
  min: undefined,
})

const model = defineModel<string>({ default: '' })

const uid = useId()
const inputId = computed(() => props.id ?? `field-${uid}`)
const errorId = computed(() => `${inputId.value}-error`)
const hintId = computed(() => `${inputId.value}-hint`)
const describedBy = computed(() => {
  const ids: string[] = []
  if (props.error) ids.push(errorId.value)
  if (props.hint) ids.push(hintId.value)
  return ids.length > 0 ? ids.join(' ') : undefined
})
</script>

<template>
  <div>
    <label
      v-if="!hideLabel"
      :for="inputId"
      class="mb-1.5 block text-xs font-semibold tracking-wide text-gray-500 uppercase"
    >
      {{ label }}
    </label>
    <label v-else :for="inputId" class="sr-only">{{ label }}</label>
    <input
      :id="inputId"
      v-model="model"
      :type="type"
      v-if="type != 'password'"
      :autocomplete="autocomplete"
      :placeholder="placeholder || label"
      :min="min"
      :aria-invalid="!!error"
      :aria-describedby="describedBy"
      class="h-11 w-full rounded-lg border border-slate-200 bg-white px-4 text-sm placeholder:text-slate-500 transition focus:border-primary focus:outline-none focus:ring-2 focus:ring-primary/20"
      :class="monospace ? 'font-mono' : ''"
    />
    <div class="relative" v-else>
      <input
        :id="inputId"
        v-model="model"
        :type="showPassword ? 'text' : 'password'"
        :autocomplete="autocomplete"
        :placeholder="placeholder || label"
        :aria-invalid="!!error"
        :aria-describedby="describedBy"
        class="h-11 w-full rounded-lg border border-slate-200 bg-white pl-4 pr-11 text-sm placeholder:text-slate-500 transition focus:border-primary focus:outline-none focus:ring-2 focus:ring-primary/20"
        :class="monospace ? 'font-mono' : ''"
      />

      <button
        type="button"
        class="absolute inset-y-0 right-0 flex w-11 items-center justify-center text-slate-500 hover:text-slate-700 focus-visible:outline-2 focus-visible:outline-offset-[-2px] focus-visible:outline-primary"
        :aria-label="showPassword ? 'Hide password' : 'Show password'"
        @click="showPassword = !showPassword"
      >
        <svg
          v-if="!showPassword"
          class="size-4"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
          aria-hidden="true"
        >
          <path d="M2 12s3.500-7 10-7 10 7 10 7-3.500 7-10 7S2 12 2 12z" />
          <circle cx="12" cy="12" r="3" />
        </svg>
        <svg
          v-else
          class="size-4"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
          aria-hidden="true"
        >
          <path
            d="M17.940 17.940A10.940 10.940 0 0 1 12 19c-6.500 0-10-7-10-7a18.500 18.500 0 0 1 5.060-5.940M9.900 4.240A9.120 9.120 0 0 1 12 5c6.500 0 10 7 10 7a18.500 18.500 0 0 1-2.160 3.190M1 1l22 22"
          />
        </svg>
      </button>
    </div>
    <p v-if="hint" :id="hintId" class="mt-1 text-xs text-gray-500">
      {{ hint }}
    </p>
    <p v-if="error" :id="errorId" class="mt-1 text-xs text-red-600">
      {{ error }}
    </p>
  </div>
</template>
