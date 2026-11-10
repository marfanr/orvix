<script setup lang="ts">
import type { Background, Participant } from '@/types/video-call.ts'
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useMeetingConfigStore } from '@/stores/meetingConfig'
import { storeToRefs } from 'pinia'

type TrackProcessor = {
  readable: ReadableStream<VideoFrame>
}

type TrackGenerator = MediaStreamTrack & {
  writable: WritableStream<VideoFrame>
}

const Processor = (globalThis as any).MediaStreamTrackProcessor as
  (new (init: { track: MediaStreamTrack; maxBufferSize?: number }) => TrackProcessor) | undefined

const Generator = (globalThis as any).MediaStreamTrackGenerator as
  (new (init: { kind: 'video' }) => TrackGenerator) | undefined

const OUTPUT_FPS = 30
const MAX_INFLIGHT = 2

const hasCanvasSink =
  typeof HTMLCanvasElement !== 'undefined' &&
  'transferControlToOffscreen' in HTMLCanvasElement.prototype

const hasVideoSource =
  typeof HTMLVideoElement !== 'undefined' &&
  'requestVideoFrameCallback' in HTMLVideoElement.prototype &&
  typeof createImageBitmap === 'function'

const hasSink = !!Generator || hasCanvasSink

type Mode = 'mstp' | 'video' | 'none'
const mode: Mode = !hasSink ? 'none' : Processor ? 'mstp' : hasVideoSource ? 'video' : 'none'

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
      return { type: 'color', value: '#ffffff' }

    case 'blur':
    default:
      return { type: 'blur', radius: form.value.blurRadius }
  }
})

const effectsEnabled = computed(() => form.value.bgremoval !== 'disabled')

const videoEl = ref<HTMLVideoElement | null>(null)

let worker: Worker | null = null
let generator: TrackGenerator | null = null
let generatedTrack: MediaStreamTrack | null = null
let outCanvas: HTMLCanvasElement | null = null
let outStream: MediaStream | null = null

let currentTrackId: string | null = null
let currentTrack: MediaStreamTrack | null = null
let currentProcessor: TrackProcessor | null = null

let workerStarted = false

let running = false
let inflight = 0
let rvfcId: number | null = null

function startWorker(): void {
  if (!worker || workerStarted) return

  if (Generator) {
    generator = new Generator({ kind: 'video' })
    generator.contentHint = 'motion'
    const writable = generator.writable

    worker.postMessage(
      {
        func: 'start_worker',
        background: backgroundRef.value,
        enabled: effectsEnabled.value,
        writable,
        mirrored: form.value.mirrorCamera,
      },
      [writable],
    )
  } else {
    outCanvas = document.createElement('canvas')
    outCanvas.width = 640
    outCanvas.height = 480

    const offscreen = outCanvas.transferControlToOffscreen()
    outStream = outCanvas.captureStream(OUTPUT_FPS)

    worker.postMessage(
      {
        func: 'start_worker',
        background: backgroundRef.value,
        enabled: effectsEnabled.value,
        canvas: offscreen,
        mirrored: form.value.mirrorCamera,
      },
      [offscreen],
    )
  }

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

function setWorkerTicker(on: boolean): void {
  worker?.postMessage({ func: 'ticker', on, fps: OUTPUT_FPS })
}

function cancelPending(): void {
  if (rvfcId !== null && videoEl.value) {
    videoEl.value.cancelVideoFrameCallback(rvfcId)
    rvfcId = null
  }
}

function schedule(): void {
  const v = videoEl.value
  if (!running || !v || document.hidden) return
  rvfcId = v.requestVideoFrameCallback(pump)
}

async function grab(): Promise<void> {
  const v = videoEl.value
  if (!running || !v || !worker) return
  if (inflight >= MAX_INFLIGHT) return
  if (v.readyState < 2 || !v.videoWidth) return

  inflight++
  try {
    const bmp = await createImageBitmap(v)
    if (!running || !worker) {
      bmp.close()
      inflight = Math.max(0, inflight - 1)
      return
    }
    worker.postMessage({ func: 'frame', bitmap: bmp }, [bmp])
  } catch {
    inflight = Math.max(0, inflight - 1)
  }
}

async function pump(): Promise<void> {
  rvfcId = null
  if (!running) return
  schedule()
  await grab()
}

function handleVisibility(): void {
  if (!running) return
  cancelPending()
  setWorkerTicker(document.hidden)
  schedule()
}

function startPump(): void {
  if (running) return
  running = true
  document.addEventListener('visibilitychange', handleVisibility)
  setWorkerTicker(document.hidden)
  schedule()
}

function stopPump(): void {
  running = false
  document.removeEventListener('visibilitychange', handleVisibility)
  cancelPending()
  setWorkerTicker(false)
  inflight = 0

  if (videoEl.value) videoEl.value.srcObject = null
}

async function detachProcessor(): Promise<void> {
  const processor = currentProcessor
  currentProcessor = null

  if (processor) {
    try {
      await processor.readable.cancel()
    } catch {}
  }
}

function attachMstp(track: MediaStreamTrack): void {
  if (!worker || !Processor) return

  void detachProcessor()

  const processor = new Processor({ track, maxBufferSize: 2 })
  currentProcessor = processor

  worker.postMessage({ func: 'set_source', readable: processor.readable }, [processor.readable])
}

function attachVideo(track: MediaStreamTrack): void {
  const v = videoEl.value
  if (!v) return

  v.srcObject = new MediaStream([track])
  v.play()
    .then(startPump)
    .catch((e) => console.warn('[VideoEffects] video.play failed', e))
}

function attachTrack(track: MediaStreamTrack | null): void {
  if (!worker || !track) return
  if (track.readyState !== 'live') return
  if (track.id === currentTrackId) return

  currentTrack = track
  currentTrackId = track.id

  if (mode === 'mstp') attachMstp(track)
  else if (mode === 'video') attachVideo(track)
}

function handleTrackEnded(): void {
  if (currentTrack?.readyState !== 'ended') return

  currentTrack = null
  currentTrackId = null

  if (mode === 'mstp') void detachProcessor()
}

function outputStream(): MediaStream | null {
  if (generatedTrack) return new MediaStream([generatedTrack])
  if (generator) return new MediaStream([generator])
  return outStream
}

function handleWorkerMessage(e: MessageEvent<{ func: string; track?: MediaStreamTrack }>): void {
  switch (e.data.func) {
    case 'track':
      if (e.data.track) {
        generatedTrack = e.data.track
        outStream?.getTracks().forEach((t) => t.stop())
        outStream = null
      }
      break

    case 'published': {
      const s = outputStream()
      if (s) emits('stream', s)
      break
    }

    case 'frame_done':
      inflight = Math.max(0, inflight - 1)
      break

    case 'mask_ready':
      model.value = { ...model.value, mask_ready: true }
      break

    case 'tick':
      void grab()
      break
  }
}

function handleWorkerError(e: ErrorEvent): void {
  console.error('[VideoWorker]', e.message, e.error)
}

onMounted(() => {
  if (mode === 'none') {
    console.warn('Background effects not supported in this browser')
    if (props.stream) emits('stream', props.stream)
    model.value = { ...model.value, mask_ready: true }
    return
  }

  worker = new Worker(new URL('../../workers/video.worker.ts', import.meta.url), {
    type: 'module',
  })

  worker.onmessage = handleWorkerMessage
  worker.onerror = handleWorkerError

  startWorker()

  const track = props.stream?.getVideoTracks()[0] ?? null

  if (track) {
    attachTrack(track)
    track.addEventListener('ended', handleTrackEnded)
  }
})

watch([backgroundRef, effectsEnabled, () => form.value.mirrorCamera], () => {
  updateBackground()
})

watch(
  () => props.stream?.getVideoTracks()[0] ?? null,
  (track, previousTrack) => {
    if (mode === 'none') return
    if (track === previousTrack) return

    previousTrack?.removeEventListener('ended', handleTrackEnded)

    attachTrack(track)

    track?.addEventListener('ended', handleTrackEnded)
  },
)

onBeforeUnmount(() => {
  currentTrack?.removeEventListener('ended', handleTrackEnded)

  stopPump()
  void detachProcessor()

  worker?.terminate()
  worker = null
  workerStarted = false

  generator?.stop()
  generator = null
  generatedTrack?.stop()
  generatedTrack = null
  outStream?.getTracks().forEach((t) => t.stop())
  outStream = null
  outCanvas = null

  currentTrack = null
  currentTrackId = null

  model.value = { ...model.value, mask_ready: false }
})
</script>

<template>
  <div
    v-if="mode === 'video'"
    class="pointer-events-none fixed top-0 left-0 size-px overflow-hidden opacity-0"
    aria-hidden="true"
  >
    <video ref="videoEl" muted playsinline autoplay></video>
  </div>
  <span v-else class="hidden" aria-hidden="true"></span>
</template>
