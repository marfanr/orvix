<script setup lang="ts">
import { useId } from 'vue'
import CheckboxElement from '@/components/ui/CheckboxElement.vue'
import SelectElement from '@/components/ui/SelectElement.vue'
import ToggleElement from '@/components/ui/ToggleElement.vue'
import { useMeetingConfig, type MeetingConfigToggleItem } from '@/composables/useMeetingConfig'

const { form } = useMeetingConfig()
const uid = useId()

const recordingToggles: MeetingConfigToggleItem[] = [
  {
    key: 'recordingEnabled',
    label: 'Allow recording',
    description: 'Let the host start a recording of this meeting.',
  },
  {
    key: 'recordingNotify',
    label: 'Notify participants',
    description: 'Show a banner in the call while recording is active.',
  },
]

const layoutOptions = [
  { value: 'grid', label: 'Grid' },
  { value: 'speaker', label: 'Speaker focus' },
  { value: 'audio-only', label: 'Audio only' },
]

const qualityOptions = [
  { value: 'call', label: 'Match call quality' },
  { value: '720p', label: '720p' },
  { value: '1080p', label: '1080p' },
]

const formatOptions = [
  { value: 'mp4', label: 'MP4 (H.264)' },
  { value: 'webm', label: 'WebM (VP8)' },
]
</script>

<template>
  <div class="flex flex-col gap-y-8">
    <div>
      <h3 class="text-base font-semibold text-gray-900">Recording</h3>
      <p class="mt-1 text-sm text-gray-500">
        Control how this meeting is captured by the server recorder.
      </p>

      <div class="mt-5 divide-y divide-gray-100 rounded-xl border border-gray-200">
        <ToggleElement
          v-for="toggle in recordingToggles"
          :key="toggle.key"
          v-model="form[toggle.key]"
          :label="toggle.label"
          :description="toggle.description"
        />
      </div>
    </div>

    <div class="grid gap-4 sm:grid-cols-2">
      <SelectElement
        :id="`${uid}-rec-layout`"
        v-model="form.recordingLayout"
        label="Layout"
        :options="layoutOptions"
      />

      <SelectElement
        :id="`${uid}-rec-quality`"
        v-model="form.recordingQuality"
        label="Recording quality"
        :options="qualityOptions"
      />

      <div class="sm:col-span-2">
        <SelectElement
          :id="`${uid}-rec-format`"
          v-model="form.recordingFormat"
          label="Container format"
          :options="formatOptions"
          hint="The server recorder only supports H.264, so MP4 is recommended."
        />
      </div>
    </div>

    <fieldset class="min-w-0">
      <legend class="text-sm font-semibold text-gray-900">Include in recording</legend>

      <div class="mt-3 flex flex-col gap-3 rounded-xl border border-gray-200 p-4">
        <CheckboxElement v-model="form.recordVideo" label="Video tracks" />
        <CheckboxElement v-model="form.recordAudio" label="Audio tracks" />
        <CheckboxElement v-model="form.recordChat" label="Chat transcript" />
      </div>
    </fieldset>
  </div>
</template>
