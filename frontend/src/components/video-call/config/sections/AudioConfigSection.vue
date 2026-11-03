<script setup lang="ts">
import { computed, useId } from 'vue'
import SelectElement from '@/components/ui/SelectElement.vue'
import ToggleElement from '@/components/ui/ToggleElement.vue'
import { useMeetingConfigStore, type MeetingConfigToggleItem } from '@/stores/meetingConfig'
import { useMicrophoneTest } from '@/composables/useMicrophoneTest'

const store = useMeetingConfigStore()
const form = store.form
const uid = useId()

const { audioInputs, micStream, micError, micLoading, micLevel, startMicTest, stopMicTest } =
  useMicrophoneTest(form)

const micOptions = computed(() => [
  { value: '', label: 'Default microphone' },
  ...audioInputs.value.map((device, index) => ({
    value: device.deviceId,
    label: device.label || `Microphone ${index + 1}`,
  })),
])

const audioQualityOptions = [
  { value: '32', label: 'Standard — 32 kbps' },
  { value: '64', label: 'High — 64 kbps' },
  { value: '128', label: 'Studio — 128 kbps' },
]

const audioToggles: MeetingConfigToggleItem[] = [
  {
    key: 'noiseSuppression',
    label: 'Noise suppression',
    description: 'Filter out background noise from your microphone.',
  },
  {
    key: 'echoCancellation',
    label: 'Echo cancellation',
    description: 'Prevent feedback from speakers being picked up.',
  },
  {
    key: 'autoGain',
    label: 'Automatic gain control',
    description: 'Keep your volume consistent while speaking.',
  },
  {
    key: 'stereo',
    label: 'Stereo audio',
    description: 'Send two audio channels instead of mono.',
  },
]
</script>

<template>
  <div class="flex flex-col gap-y-8">
    <div>
      <h3 class="text-base font-semibold text-gray-900">Audio</h3>
      <p class="mt-1 text-sm text-gray-500">
        Pick a microphone and tune the processing before it reaches the room.
      </p>

      <div class="mt-5 grid gap-4 sm:grid-cols-2">
        <SelectElement
          :id="`${uid}-mic`"
          v-model="form.mic"
          label="Microphone"
          :options="micOptions"
        />

        <SelectElement
          :id="`${uid}-audio-bitrate`"
          v-model="form.audioBitrate"
          label="Audio quality"
          :options="audioQualityOptions"
        />
      </div>

      <div class="mt-4 rounded-xl border border-gray-200 p-4">
        <div class="flex flex-wrap items-center justify-between gap-3">
          <div class="min-w-0">
            <p class="text-sm font-medium text-gray-800">Microphone test</p>
            <p class="mt-0.5 text-xs text-gray-500">Speak to see your live input level.</p>
          </div>
          <button
            type="button"
            :disabled="micLoading"
            class="shrink-0 rounded-lg border border-gray-200 bg-white px-3 py-2 text-xs font-medium text-gray-700 transition hover:border-primary hover:text-primary focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary disabled:cursor-not-allowed disabled:opacity-60"
            @click="micStream ? stopMicTest() : startMicTest()"
          >
            {{ micLoading ? 'Starting…' : micStream ? 'Stop test' : 'Test microphone' }}
          </button>
        </div>

        <div class="mt-4">
          <div class="flex items-center justify-between text-xs text-gray-500">
            <span>Input level</span>
            <span class="tabular-nums">{{ micStream ? `${micLevel}%` : 'Idle' }}</span>
          </div>
          <div
            class="mt-1.5 h-2 w-full overflow-hidden rounded-full bg-gray-200"
            role="meter"
            aria-label="Microphone input level"
            aria-valuemin="0"
            aria-valuemax="100"
            :aria-valuenow="micLevel"
          >
            <div
              class="h-full rounded-full transition-[width] duration-75"
              :class="micLevel > 85 ? 'bg-danger' : 'bg-primary'"
              :style="{ width: `${micLevel}%` }"
            ></div>
          </div>

          <p v-if="micError" role="alert" class="mt-2 text-xs text-red-600">
            {{ micError }}
          </p>
          <p v-else class="mt-2 text-xs text-gray-500">
            {{
              micStream
                ? 'Microphone is live. Audio stays on your device during this test.'
                : 'Your microphone is only used locally while testing.'
            }}
          </p>
        </div>
      </div>
    </div>

    <div>
      <h4 class="text-sm font-semibold text-gray-900">Audio processing</h4>

      <div class="mt-3 divide-y divide-gray-100 rounded-xl border border-gray-200">
        <ToggleElement
          v-for="toggle in audioToggles"
          :key="toggle.key"
          v-model="form[toggle.key]"
          :label="toggle.label"
          :description="toggle.description"
        />
      </div>
    </div>
  </div>
</template>
