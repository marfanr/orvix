<script setup lang="ts">
import type { Background, Participant } from '@/types/video-call.ts'
import {
  computed,
  onBeforeUnmount,
  onMounted,
  watch,
} from 'vue'
import { useMeetingConfigStore } from '@/stores/meetingConfig'
import { storeToRefs } from 'pinia'

type TrackProcessor = {
  readable: ReadableStream<VideoFrame>
}

type TrackGenerator = MediaStreamTrack & {
  writable: WritableStream<VideoFrame>
}

const Processor = (globalThis as any).MediaStreamTrackProcessor as
  | (new (init: {
      track: MediaStreamTrack
      maxBufferSize?: number
    }) => TrackProcessor)
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

const model = defineModel<{ mask_ready: boolean }>({
  required: true,
})

const backgroundRef = computed((): Background => {
  switch (form.value.bgremoval) {
    case 'color':
      return {
        type: 'color',
        value: '#ffffff',
      }

    case 'blur':
    default:
      return {
        type: 'blur',
        radius: form.value.blurRadius,
      }
  }
})

let worker: Worker | null = null
let generator: TrackGenerator | null = null

let currentTrackId: string | null = null
let currentTrack: MediaStreamTrack | null = null
let currentProcessor: TrackProcessor | null = null

let workerStarted = false
let mounted = false

const effectsEnabled = computed(
  () => form.value.bgremoval !== 'disabled',
)


function startWorker(): void {
  if (!worker || !generator || workerStarted) return

  const writable = generator.writable

  worker.postMessage(
    {
      func: 'start_worker',
      background: backgroundRef.value,
      enabled: effectsEnabled.value,
      writable,
      mirrored: form.value.mirrorCamera
    },
    [writable],
  )

  workerStarted = true
}

function updateBackground(): void {
  if (!worker || !workerStarted) return

  worker.postMessage({
    func: 'set_background',
    background: backgroundRef.value,
    enabled: effectsEnabled.value,
    mirrored: form.value.mirrorCamera,
  })
}

async function detachTrack(): Promise<void> {
  const processor = currentProcessor
  const track = currentTrack

  currentProcessor = null
  currentTrack = null
  currentTrackId = null

  if (processor) {
    try {
      await processor.readable.cancel()
    } catch {
      // Stream may already be closed or transferred.
    }
  }

  if (track) {
    // Do not stop the original camera track.
    // It may be shared with other components.
  }
}

function attachTrack(track: MediaStreamTrack | null): void {
  if (!worker || !Processor || !track) return
  if (track.readyState !== 'live') return
  if (track.id === currentTrackId) return

  void detachTrack()

  currentTrack = track
  currentTrackId = track.id

  const processor = new Processor({
    track,
    maxBufferSize: 2,
  })

  currentProcessor = processor

  worker.postMessage(
    {
      func: 'set_source',
      readable: processor.readable,
    },
    [processor.readable],
  )
}

function handleWorkerMessage(
  e: MessageEvent<{ func: string }>,
): void {
  switch (e.data.func) {
    case 'published':
      if (generator) {
        emits('stream', new MediaStream([generator]))
      }
      break

    case 'mask_ready':
      model.value = {
        ...model.value,
        mask_ready: true,
      }
      break
  }
}

function handleWorkerError(e: ErrorEvent): void {
  console.error('[VideoWorker]', e.message, e.error)
}

function handleTrackEnded(): void {
  if (currentTrack?.readyState === 'ended') {
    void detachTrack()
  }
}

onMounted(() => {
  mounted = true

  if (!Processor || !Generator) {
    console.warn(
      'MediaStreamTrackProcessor/Generator not supported',
    )
    return
  }

  generator = new Generator({ kind: 'video' })
  generator.contentHint = 'motion'

  worker = new Worker(
    new URL('../../workers/video.worker.ts', import.meta.url),
    { type: 'module' },
  )

  worker.onmessage = handleWorkerMessage
  worker.onerror = handleWorkerError

  startWorker()

  const track = props.stream?.getVideoTracks()[0] ?? null

  if (track) {
    attachTrack(track)
  }
})

watch(
  [backgroundRef, effectsEnabled, () => form.value.mirrorCamera],
  () => {
    updateBackground()
  },
)

watch(
  () => props.stream?.getVideoTracks()[0] ?? null,
  (track, previousTrack) => {
    if (track === previousTrack) return

    if (currentTrack) {
      currentTrack.removeEventListener(
        'ended',
        handleTrackEnded,
      )
    }

    attachTrack(track)

    track?.addEventListener(
      'ended',
      handleTrackEnded,
    )
  },
)

onBeforeUnmount(() => {
  mounted = false

  currentTrack?.removeEventListener(
    'ended',
    handleTrackEnded,
  )

  worker?.terminate()
  worker = null

  workerStarted = false

  generator?.stop()
  generator = null

  currentProcessor = null
  currentTrack = null
  currentTrackId = null

  model.value = {
    ...model.value,
    mask_ready: false,
  }
})
</script>

<template>
  <span class="hidden" aria-hidden="true"></span>
</template>
