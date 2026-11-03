<script setup lang="ts">
import { computed, useId } from 'vue'
import SelectElement from '@/components/ui/SelectElement.vue'
import ToggleElement from '@/components/ui/ToggleElement.vue'
import { useMeetingConfigStore, type MeetingConfigToggleItem } from '@/stores/meetingConfig'
import { SupportedBitrate, SupportedVideoCodec } from '@/types/video-call.ts'

const store = useMeetingConfigStore()
const form = store.form
const uid = useId()

const bitrateOptions = [
  { value: SupportedBitrate.AUTO, label: 'Auto' },
  { value: SupportedBitrate.B1, label: '1 Mbps' },
  { value: SupportedBitrate.B2, label: '2 Mbps' },
  { value: SupportedBitrate.B4, label: '4 Mbps' },
  { value: SupportedBitrate.B8, label: '8 Mbps' },
]

const codecOptions = [
  { value: SupportedVideoCodec.H264, label: 'H.264' },
  { value: SupportedVideoCodec.VP8, label: 'VP8' },
  { value: SupportedVideoCodec.VP9, label: 'VP9' },
  { value: SupportedVideoCodec.AV1, label: 'AV1' },
]

const priorityOptions = [
  { value: 'low', label: 'Low' },
  { value: 'medium', label: 'Medium' },
  { value: 'high', label: 'High' },
]

const networkToggles: MeetingConfigToggleItem[] = [
  {
    key: 'adaptiveBitrate',
    label: 'Adaptive bitrate',
    description: 'Lower quality automatically when the connection drops.',
  },
  {
    key: 'simulcast',
    label: 'Simulcast',
    description: 'Send multiple video layers so weak peers can pick a lower one.',
  },
]

const estimatedUpload = computed(() => {
  const byBitrate: Record<string, string> = {
    auto: 'adaptive',
    '1': 'up to 1 Mbps',
    '2': 'up to 2 Mbps',
    '4': 'up to 4 Mbps',
    '8': 'up to 8 Mbps',
  }
  return byBitrate[form.maxBitrate] ?? 'adaptive'
})
</script>

<template>
  <div class="flex flex-col gap-y-8">
    <div>
      <h3 class="text-base font-semibold text-gray-900">Network & bandwidth</h3>
      <p class="mt-1 text-sm text-gray-500">
        Tune how media is sent to keep calls stable on any connection.
      </p>

      <div class="mt-5 grid gap-4 sm:grid-cols-2">
        <SelectElement
          :id="`${uid}-bitrate`"
          v-model="form.maxBitrate"
          label="Max upload bitrate"
          :options="bitrateOptions"
          :hint="`Video upload is ${estimatedUpload}.`"
        />

        <SelectElement
          :id="`${uid}-codec`"
          v-model="form.preferredCodec"
          label="Preferred video codec"
          :options="codecOptions"
          hint="for now H.264 is required while server recording is enabled."
        />

        <SelectElement
          :id="`${uid}-priority`"
          v-model="form.networkPriority"
          label="Network priority"
          :options="priorityOptions"
        />


      </div>
    </div>

    <div>
      <h4 class="text-sm font-semibold text-gray-900">Transport</h4>

      <div class="mt-3 divide-y divide-gray-100 rounded-xl border border-gray-200">
        <ToggleElement
          v-for="toggle in networkToggles"
          :key="toggle.key"
          v-model="form[toggle.key]"
          :label="toggle.label"
          :description="toggle.description"
        />
      </div>
    </div>
  </div>
</template>
