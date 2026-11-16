<!--
SPDX-FileCopyrightText: 2026 Mohammad Arfan

SPDX-License-Identifier: AGPL-3.0
-->

<script setup lang="ts">
import { type ComponentPublicInstance, computed, nextTick, onBeforeUnmount, ref, watch } from 'vue'
import type { Participant } from '@/types/video-call'
import { MicIcon, MicOffIcon } from 'lucide-vue-next'
import VideoMaskCanvas from '@/components/video-call/VideoMaskCanvas.vue'
import { useMeetingConfigStore } from '@/stores/meetingConfig.ts'
import { storeToRefs } from 'pinia'
import { useCallStore } from '@/stores/call.ts'
import AudioComponent from './AudioComponent.vue'

const meetingStore = useMeetingConfigStore()
const { form } = storeToRefs(meetingStore)
const callStore = useCallStore()
const { outputStream } = storeToRefs(callStore)

const LOCAL_PARTICIPANT_ID = 0
const RESTORE_DELAY_MS = 1000

const model = ref<{
  mask_ready: boolean
}>({
  mask_ready: false,
})

const props = defineProps<{
  participant: Participant
  tileClass?: string
}>()

const showCanvas = computed(
  () =>
    props.participant.id === LOCAL_PARTICIPANT_ID &&
    (form.value.bgremoval != 'disabled' || form.value.mirrorCamera) &&
    !props.participant.cameraOff,
)

const MicStateIcon = computed(() => (props.participant.muted ? MicOffIcon : MicIcon))

function initials(name: string): string {
  return name.trim().charAt(0).toUpperCase() || '?'
}

const videoRef = ref<HTMLVideoElement | null>(null)

const sourceStream = ref<MediaStream | null>(null)
const previewStream = ref<MediaStream | null>(null)
let canvasStream: MediaStream | null = null
let outStream: MediaStream | null = null

function trackIds(tracks: MediaStreamTrack[]): string {
  return tracks
    .map((t) => t.id)
    .sort()
    .join(',')
}

function isOwnOutput(s: MediaStream): boolean {
  if (!canvasStream) return false
  const ids = new Set(canvasStream.getVideoTracks().map((t) => t.id))
  return s.getVideoTracks().some((t) => ids.has(t.id))
}

watch(
  () => props.participant.stream,
  (s) => {
    if (!s) return
    const stream = s as MediaStream
    if (stream === outStream || isOwnOutput(stream)) return
    sourceStream.value = stream
  },
  { immediate: true },
)

function attachStream(el: HTMLMediaElement, stream: MediaStream): void {
  if (el.srcObject !== stream) el.srcObject = stream
  el.play().catch((err) => console.warn(`${el.tagName.toLowerCase()} play error:`, err))
}

async function setVideoStream(): Promise<void> {
  await nextTick()
  const stream = (showCanvas.value && previewStream.value) || sourceStream.value
  if (videoRef.value && stream) attachStream(videoRef.value, stream)
}

function buildOutStream(): void {
  if (!canvasStream || !showCanvas.value) return

  const audioTracks = sourceStream.value ? sourceStream.value.getAudioTracks() : []

  outStream = new MediaStream([...canvasStream.getVideoTracks(), ...audioTracks])

  outputStream.value = outStream
}

function onCanvasStream(stream: MediaStream): void {
  if (!stream.getVideoTracks().length) return
  canvasStream = stream
  previewStream.value = new MediaStream(stream.getVideoTracks())
  buildOutStream()
}

function restoreOutputStream(): void {
  const mine = outStream
  outStream = null
  canvasStream = null
  previewStream.value = null

  if (mine && outputStream.value === mine) {
    outputStream.value = null
  }
}

watch(showCanvas, (enabled) => {
  if (!enabled) restoreOutputStream()
  setVideoStream()
})

watch(previewStream, () => setVideoStream())

watch(sourceStream, (raw) => {
  setVideoStream()
  if (!outStream || !canvasStream || !raw) return
  const currentAudio = trackIds(outStream.getAudioTracks())
  const nextAudio = trackIds(raw.getAudioTracks())
  if (currentAudio !== nextAudio) buildOutStream()
})

onBeforeUnmount(() => {
  const mine = outStream
  outStream = null
  canvasStream = null
  previewStream.value = null
  if (!mine) return

  setTimeout(() => {
    if (outputStream.value === mine) outputStream.value = null
  }, RESTORE_DELAY_MS)
})

function setVideoRef(el: Element | ComponentPublicInstance | null): void {
  videoRef.value = el instanceof HTMLVideoElement ? el : null
  if (videoRef.value) setVideoStream()
}
</script>

<template>
  <div
    class="relative flex h-full min-h-0 w-full min-w-0 items-center justify-center overflow-hidden bg-stone-900 text-white"
    :class="tileClass"
  >
    <div
      v-if="participant.cameraOff"
      class="flex size-16 shrink-0 items-center justify-center rounded-full bg-brown-ribbon-800 sm:size-20 md:size-24"
    >
      <span class="text-lg font-medium sm:text-xl md:text-2xl" aria-hidden="true">
        {{ initials(participant.name) }}
      </span>
    </div>

    <div v-show="!participant.cameraOff" class="absolute inset-0">
      <VideoMaskCanvas
        v-if="showCanvas"
        :participant="participant"
        :stream="sourceStream"
        :model-value="model"
        @stream="onCanvasStream"
      />
      <video
        :ref="setVideoRef"
        class="h-full w-full bg-black object-cover"
        autoplay
        playsinline
        muted
      ></video>
    </div>

    <AudioComponent :participant="participant" />

    <div
      v-if="participant.speaking"
      class="pointer-events-none absolute inset-0 rounded-[inherit] ring-2 ring-success ring-inset"
      aria-hidden="true"
    />

    <div
      class="absolute bottom-1 left-1 flex max-w-[calc(100%-0.5rem)] items-center gap-x-1.5 rounded-md bg-black/70 p-1 text-[10px] sm:bottom-2 sm:left-2 sm:max-w-[calc(100%-1rem)] sm:gap-x-2 sm:p-1.5 sm:text-xs md:bottom-4 md:left-4 md:gap-x-3 md:p-2"
    >
      <span class="truncate">{{ participant.name }}</span>
      <span class="sr-only">{{ participant.muted ? 'Muted' : 'Unmuted' }}</span>
      <component
        :is="MicStateIcon"
        class="size-3 shrink-0 sm:size-3.5 md:size-[15px]"
        aria-hidden="true"
      />
    </div>
  </div>
</template>
