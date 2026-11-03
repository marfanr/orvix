<script setup lang="ts">
defineProps<{
  label: string
}>()

const checked = defineModel<boolean>({ required: true })
</script>

<template>
  <label class="flex cursor-pointer items-start gap-3">
    <input
      v-model="checked"
      type="checkbox"
      class="mt-0.5 size-4 shrink-0 cursor-pointer rounded border-gray-300 accent-primary"
    />
    <span class="text-sm text-gray-700">{{ label }}</span>
  </label>
</template>
