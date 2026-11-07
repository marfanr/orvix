<script setup lang="ts">
import { computed, useId } from 'vue'
import { VideoIcon } from 'lucide-vue-next'
import SelectElement from '@/components/ui/SelectElement.vue'
import ToggleElement from '@/components/ui/ToggleElement.vue'
import { useCameraPreview } from '@/composables/useCameraPreview'
import { type MeetingConfigToggleItem, useMeetingConfigStore } from '@/stores/meetingConfig'
import RangeElement from '@/components/ui/RangeElement.vue'

const store = useMeetingConfigStore()
const form = store.form
const uid = useId()

const {
  videoInputs,
  videoPreviewEl,
  videoStream,
  videoError,
  videoLoading,
  activeCameraLabel,
  startVideoPreview,
  stopVideoPreview,
} = useCameraPreview(form)

const cameraOptions = computed(() => [
  { value: '', label: 'Default camera' },
  ...videoInputs.value.map((device, index) => ({
    value: device.deviceId,
    label: device.label || `Camera ${index + 1}`,
  })),
])

const resolutionOptions = [
  { value: '360p', label: '360p — data saver' },
  { value: '720p', label: '720p — recommended' },
  { value: '1080p', label: '1080p — high quality' },
]

const BackgroundRemovalOptions = [
  { value: 'disabled', label: 'disabled' },
  { value: 'blur', label: 'blur' },
  { value: 'image', label: 'image' },
  { value: 'color', label: 'color' },
]

const framerateOptions = [
  { value: '15', label: '15 fps — low bandwidth' },
  { value: '24', label: '24 fps — cinematic' },
  { value: '30', label: '30 fps — standard' },
  { value: '60', label: '60 fps — smooth motion' },
]

const contentHintOptions = [
  { value: 'motion', label: 'Motion — camera video' },
  { value: 'detail', label: 'Detail — screen share or text' },
]



const videoToggles: MeetingConfigToggleItem[] = [
  {
    key: 'lowLightMode',
    label: 'Low-light mode',
    description: 'Brighten the image when lighting is poor.',
  },
]
</script>

<template>
  <div class="flex flex-col gap-y-8">
    <div>
      <h3 class="text-base font-semibold text-gray-900">Video</h3>
      <p class="mt-1 text-sm text-gray-500">
        Choose the capture quality and how your camera is delivered.
      </p>

      <div class="mt-4 overflow-hidden rounded-xl border border-gray-200 bg-stone-900">
        <div class="relative aspect-video">
          <video v-show="videoStream" ref="videoPreviewEl" autoplay muted playsinline class="h-full w-full object-cover"
            :class="form.mirrorPreview ? '-scale-x-100' : ''"></video>

          <div v-if="!videoStream"
            class="absolute inset-0 flex flex-col items-center justify-center gap-2 px-4 text-center">
            <VideoIcon class="size-8 text-gray-500" aria-hidden="true" />
            <p v-if="videoError" role="alert" class="max-w-xs text-sm text-red-300">
              {{ videoError }}
            </p>
            <p v-else class="text-sm text-gray-400">Camera preview is off</p>
            <button type="button" :disabled="videoLoading"
              class="mt-1 rounded-lg bg-white/10 px-3 py-2 text-xs font-medium text-white transition hover:bg-white/20 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-white disabled:cursor-not-allowed disabled:opacity-60"
              @click="startVideoPreview">
              {{ videoLoading ? 'Starting…' : 'Start preview' }}
            </button>
          </div>
        </div>

        <div class="flex items-center justify-between gap-3 border-t border-white/10 px-3 py-2">
          <p class="min-w-0 truncate text-xs text-gray-300">{{ activeCameraLabel }}</p>
          <button v-if="videoStream" type="button"
            class="shrink-0 rounded-md px-2 py-1 text-xs font-medium text-gray-200 transition hover:bg-white/10 hover:text-white focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-white"
            @click="stopVideoPreview">
            Stop preview
          </button>
        </div>
      </div>

      <div class="mt-5 grid gap-4 sm:grid-cols-2 ">
        <div class="col-span-3">
          <SelectElement v-model="form.bgremoval" :id="`${uid}-bg-removal`" label="Background Removal"
            :options="BackgroundRemovalOptions" />

          <div class="my-6 flex flex-col gap-y-3" v-if="form.bgremoval != 'disabled'">
            <h4 class="text-xs font-semibold text-gray-900">Background Removal</h4>
            <RangeElement v-if="form.bgremoval == 'blur'" v-model="form.blurRadius" label="blur radius" description=""
              :min="10" :max="64" :step="5" :show-as-percentage="true" />
          </div>
        </div>

        <SelectElement :id="`${uid}-camera`" v-model="form.camera" label="Camera" :options="cameraOptions" />


        <SelectElement :id="`${uid}-resolution`" v-model="form.resolution" label="Resolution"
          :options="resolutionOptions" />

        <div class="col-span-3">
          <ToggleElement v-model="form.mirrorCamera" label="Mirror my video"
            description="Flip your camera preview horizontally." />
        </div>

        <SelectElement :id="`${uid}-framerate`" v-model="form.framerate" label="Frame rate"
          :options="framerateOptions" />

        <div class="sm:col-span-3">
          <SelectElement :id="`${uid}-content-hint`" v-model="form.contentHint" label="Content hint"
            :options="contentHintOptions" hint="Detail keeps text sharp but may use more bandwidth." />
        </div>
      </div>
    </div>

    <div>
      <h4 class="text-sm font-semibold text-gray-900">Preview</h4>

      <div class="mt-3 divide-y divide-gray-100 rounded-xl border border-gray-200">
        <ToggleElement v-for="toggle in videoToggles" :key="toggle.key" v-model="form[toggle.key]" :label="toggle.label"
          :description="toggle.description" :disabled="true"/>
      </div>
    </div>
  </div>
</template>
