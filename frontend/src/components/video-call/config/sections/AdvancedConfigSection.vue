<script setup lang="ts">
import { useId } from 'vue'
import CheckboxElement from '@/components/ui/CheckboxElement.vue'
import InputElement from '@/components/ui/InputElement.vue'
import ToggleElement from '@/components/ui/ToggleElement.vue'
import { useMeetingConfig, type MeetingConfigToggleItem } from '@/composables/useMeetingConfig'

const { form, resetDefaults } = useMeetingConfig()
const uid = useId()

const advancedToggles: MeetingConfigToggleItem[] = [
  {
    key: 'forceRelay',
    label: 'Relay all traffic',
    description: 'Route media through TURN only. More private, higher latency.',
  },
  {
    key: 'diagStats',
    label: 'Connection stats overlay',
    description: 'Show live bitrate, jitter, and packet loss on video tiles.',
  },
]
</script>

<template>
  <div class="flex flex-col gap-y-8">
    <div>
      <h4 class="text-sm font-semibold text-gray-900">Connection</h4>

      <div class="mt-3 divide-y divide-gray-100 rounded-xl border border-gray-200">
        <ToggleElement
          v-for="toggle in advancedToggles"
          :key="toggle.key"
          v-model="form[toggle.key]"
          :label="toggle.label"
          :description="toggle.description"
        />
      </div>
    </div>

    <fieldset class="min-w-0">
      <legend class="text-sm font-semibold text-gray-900">Diagnostics</legend>

      <div class="mt-3 flex flex-col gap-3 rounded-xl border border-gray-200 p-4">
        <CheckboxElement v-model="form.logSdp" label="Log SDP offers and answers to the console" />
        <CheckboxElement v-model="form.logIce" label="Log ICE candidates" />
        <CheckboxElement v-model="form.bitrateGraph" label="Show bitrate graph" />
      </div>
    </fieldset>

    <div
      class="flex flex-col gap-3 rounded-xl border border-gray-200 bg-gray-50 p-4 sm:flex-row sm:items-center sm:justify-between"
    >
      <div>
        <p class="text-sm font-medium text-gray-800">Reset to defaults</p>
        <p class="mt-0.5 text-xs text-gray-500">
          Restore the recommended settings for this device.
        </p>
      </div>
      <button
        type="button"
        class="shrink-0 rounded-lg border border-gray-200 bg-white px-3 py-2 text-xs font-medium text-gray-700 transition hover:border-primary hover:text-primary focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
        @click="resetDefaults"
      >
        Reset
      </button>
    </div>
  </div>
</template>
