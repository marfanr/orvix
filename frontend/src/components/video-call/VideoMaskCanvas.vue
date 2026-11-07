// SPDX-FileCopyrightText: 2026 Mohammad Arfan
//
// SPDX-License-Identifier: AGPL-3.0

<script setup lang="ts">
import type { Background, Participant } from '@/types/video-call.ts'
import { computed, onBeforeUnmount, onMounted, watch } from 'vue'
import { useMeetingConfigStore } from '@/stores/meetingConfig'
import { storeToRefs } from 'pinia'

type TrackProcessor = { readable: ReadableStream<VideoFrame> }
type TrackGenerator = MediaStreamTrack & { writable: WritableStream<VideoFrame> }

const Processor = (globalThis as any).MediaStreamTrackProcessor as
  | (new (init: { track: MediaStreamTrack; maxBufferSize?: number }) => TrackProcessor)
  | undefined
const Generator = (globalThis as any).MediaStreamTrackGenerator as
  | (new (init: { kind: 'video' }) => TrackGenerator)
  | undefined

const meetingStore = useMeetingConfigStore()
const { form } = storeToRefs(meetingStore)

const props = defineProps<{
  participant: Participant
  stream: MediaStream | null
}>()

const emits = defineEmits<{
  stream: [MediaStream]
}>()

const model = defineModel<{ mask_ready: boolean }>({ required: true })

const backgroundRef = computed((): Background => {
  switch (form.value.bgremoval) {
    case 'color':
      return { type: 'color', value: '#ffffff' }
    case 'blur':
    default:
      return { type: 'blur', radius: form.value.blurRadius }
  }
})

let worker: Worker | null = null
let generator: TrackGenerator | null = null
let currentTrackId: string | null = null

function attachTrack(track: MediaStreamTrack | null): void {
  if (!worker || !Processor || !track) return
  if (track.readyState !== 'live' || track.id === currentTrackId) return
  currentTrackId = track.id

  const processor = new Processor({ track, maxBufferSize: 2 })
  worker.postMessage({ func: 'set_source', readable: processor.readable }, [processor.readable])
}

function handleWorkerMessage(e: MessageEvent<{ func: string }>): void {
  switch (e.data.func) {
    case 'published':
      if (generator) emits('stream', new MediaStream([generator]))
      break
    case 'mask_ready':
      model.value = { ...model.value, mask_ready: true }
      break
  }
}

onMounted(() => {
  // for now, only foccused on Chrome Based
  if (!Processor || !Generator) {
    console.warn('MediaStreamTrackProcessor/Generator not supported')
    return
  }

  generator = new Generator({ kind: 'video' })
  ;(generator as MediaStreamTrack).contentHint = 'motion'

  worker = new Worker(new URL('../../workers/video.worker.ts', import.meta.url), {
    type: 'module',
  })
  worker.onmessage = handleWorkerMessage
  worker.onerror = (e) => console.warn(e)

  worker.postMessage(
    { func: 'start_worker', background: backgroundRef.value, writable: generator.writable },
    [generator.writable],
  )

  attachTrack(props.stream?.getVideoTracks()[0] ?? null)
})

watch(
  () => props.stream?.getVideoTracks()[0] ?? null,
  (track) => attachTrack(track),
)

watch(backgroundRef, (background) => {
  worker?.postMessage({ func: 'set_background', background })
})

onBeforeUnmount(() => {
  worker?.postMessage({ func: 'stop_worker' })
  worker?.terminate()
  worker = null
  generator?.stop()
  generator = null
  currentTrackId = null
  model.value = { ...model.value, mask_ready: false }
})
</script>

<template>
  <span class="hidden" aria-hidden="true" />
</template>
