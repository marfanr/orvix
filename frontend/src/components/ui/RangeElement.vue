<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{
  label: string
  description: string
  help?: string
  min: number
  max: number
  step: number
  showAsPercentage?: boolean
}>()

const rangeValue = defineModel<number>({ required: true })

const showingValue = computed(() => {
  if (props.showAsPercentage) {
    return ((rangeValue.value - props.min) / (props.max - props.min)) * 100
  }
  return rangeValue.value
})
</script>

<style>
.range::-webkit-slider-thumb {
  -webkit-appearance: none;
  appearance: none;
  width: 54px;
  height: 34px;
  border-radius: var(--radius-xl);
  background: var(--color-primary);
}
</style>

<template>
  <div class="gap-y-2 flex flex-col">
    <div class="flex flex-row justify-between items-center">
      <label
        for="points"
        class="flex items-center gap-1.5 text-xs font-semibold tracking-wide text-gray-500 uppercase"
        >{{ label }}</label
      >
      <span class="text-md text-gray-800 font-extrabold"
        >{{ showingValue }} {{ props.showAsPercentage ? '%' : '' }}</span
      >
    </div>
    <input
      v-model="rangeValue"
      type="range"
      :min="min"
      :max="max"
      :step="(max - min) / step"
      name="points"
      class="appearance-none border border-gray-200 p-1 rounded-xl cursor-pointer range"
    />
  </div>
</template>
