<script setup lang="ts">
import type { Participant } from '@/types/video-call.ts'
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { FilesetResolver, ImageSegmenter } from '@mediapipe/tasks-vision'
import {
  disposeOpenCV,
  drawBlurred,
  drawCover,
  initOpenCV,
  writeSmoothedMask,
} from '@/lib/filter.ts'
import { useMeetingConfigStore } from '@/stores/meetingConfig'
import { storeToRefs } from 'pinia'

const meetingStore = useMeetingConfigStore()
const { form } = storeToRefs(meetingStore)

type Background =
  | { type: 'color'; value: string }
  | { type: 'image'; image: HTMLImageElement | ImageBitmap }
  | { type: 'blur'; radius: number }

const props = defineProps<{
  participant: Participant
  video: HTMLVideoElement | null
}>()

const emits = defineEmits<{
  stream: [MediaStream]
}>()

const model = defineModel<{
  mask_ready: boolean
}>({ required: true })

const WASM_URL = 'https://cdn.jsdelivr.net/npm/@mediapipe/tasks-vision@latest/wasm'
const SELFIE_MODEL_URL =
  'https://storage.googleapis.com/mediapipe-models/image_segmenter/selfie_segmenter_landscape/float16/latest/selfie_segmenter_landscape.tflite'
const SEGMENT_WIDTH = 256
const SEGMENT_FPS = 15
const SEGMENT_INTERVAL = 1000 / SEGMENT_FPS - 2
const STREAM_FPS = 30

// Watchdog: Keeps repainting the canvas if requestVideoFrameCallback halts (hidden tab,
// unrendered video, broken loop), ensuring captureStream continues sending frames.
const WATCHDOG_MS = 100

// dont render more faster than STREAM_FPS (0.8 for tolerance jitter timestamp)
const MIN_FRAME_MS = (1000 / STREAM_FPS) * 0.8

const FALLBACK_DELEGATE = 'CPU'

const backgroundRef = computed((): Background => {
  switch (form.value.bgremoval) {
    case 'color':
      return { type: 'color', value: '#ffffff' }
    case 'blur':
    default:
      return { type: 'blur', radius: form.value.blurRadius }
  }
})

const canvasRef = ref<HTMLCanvasElement | null>(null)

const inputCanvas = document.createElement('canvas')
const inputCtx = inputCanvas.getContext('2d', { alpha: false })!
const maskCanvas = document.createElement('canvas')
const maskCtx = maskCanvas.getContext('2d')!
const blurCanvas = document.createElement('canvas')
const blurCtx = blurCanvas.getContext('2d', { alpha: false })!

let segmenter: ImageSegmenter | null = null
let useFallbackBlur = false
let maskImage: ImageData | null = null
let mask32: Uint32Array | null = null
let outCtx: CanvasRenderingContext2D | null = null
let videoEl: HTMLVideoElement | null = null
let frameCb = 0
let watchdog = 0
let lastRun = 0
let lastPaint = 0
let lastErrLog = 0
let maskReady = false
let published = false
let disposed = false

function logThrottled(label: string, e: unknown): void {
  const t = performance.now()
  if (t - lastErrLog < 2000) return
  lastErrLog = t
  console.error(`[VideoMaskCanvas] ${label}`, e)
}

async function initSegmenter(): Promise<void> {
  const vision = await FilesetResolver.forVisionTasks(WASM_URL)

  let instance: ImageSegmenter | null = null
  let lastError: Error | null = null

  // Try GPU first, then CPU if GPU fails. If both fail we fall back to a simple
  // canvas-based blur (no segmentation mask).
  for (const delegate of ['GPU', FALLBACK_DELEGATE] as const) {
    try {
      instance = await ImageSegmenter.createFromOptions(vision, {
        baseOptions: { modelAssetPath: SELFIE_MODEL_URL, delegate },
        runningMode: 'VIDEO',
        outputCategoryMask: true,
        outputConfidenceMasks: false,
      })
      console.log(`[VideoMaskCanvas] segmenter initialized with delegate: ${delegate}`)
      useFallbackBlur = false
      break
    } catch (e) {
      lastError = e as Error
      console.warn(`[VideoMaskCanvas] segmenter init failed with ${delegate}:`, e)
    }
  }

  if (disposed) {
    instance?.close()
    return
  }

  if (!instance) {
    console.error('[VideoMaskCanvas] segmenter init failed on all delegates', lastError)
    // No segmentation mask we will use a simple blur fallback.
    segmenter = null
    useFallbackBlur = true
    return
  }

  segmenter = instance
}

async function initFilters(): Promise<void> {
  await initOpenCV()
  if (disposed) disposeOpenCV()
}

function drawSegmenterInput(video: HTMLVideoElement): void {
  const width = SEGMENT_WIDTH
  const height = Math.max(1, Math.round((SEGMENT_WIDTH * video.videoHeight) / video.videoWidth))

  if (inputCanvas.width !== width || inputCanvas.height !== height) {
    inputCanvas.width = width
    inputCanvas.height = height
  }

  inputCtx.drawImage(video, 0, 0, width, height)
}

function updateMaskCanvas(labels: Uint8Array, width: number, height: number): void {
  if (!maskImage || !mask32 || maskImage.width !== width || maskImage.height !== height) {
    maskCanvas.width = width
    maskCanvas.height = height
    maskImage = maskCtx.createImageData(width, height)
    mask32 = new Uint32Array(maskImage.data.buffer)
  }

  writeSmoothedMask(labels, mask32, width, height)
  maskCtx.putImageData(maskImage, 0, 0)
}

function segment(video: HTMLVideoElement): void {
  drawSegmenterInput(video)

  segmenter!.segmentForVideo(inputCanvas, performance.now(), (result) => {
    const mask = result.categoryMask
    if (!mask) return

    updateMaskCanvas(mask.getAsUint8Array(), mask.width, mask.height)
    maskReady = true
  })
}

function drawBackground(
  ctx: CanvasRenderingContext2D,
  video: HTMLVideoElement,
  w: number,
  h: number,
): void {
  const bg = backgroundRef.value
  switch (bg.type) {
    case 'blur':
      // Blur the whole video as background behind the masked person
      drawBlurred(ctx, blurCtx, blurCanvas, video, w, h, bg.radius)
      break
    case 'image':
      drawCover(ctx, bg.image, w, h)
      break
    case 'color':
      ctx.fillStyle = bg.value
      ctx.fillRect(0, 0, w, h)
      break
  }
}

function prepareCanvas(canvas: HTMLCanvasElement, w: number, h: number): CanvasRenderingContext2D {
  // trigger reconfigure encoder (only its needed)
  if (canvas.width !== w || canvas.height !== h) {
    canvas.width = w
    canvas.height = h
  }

  outCtx ??= canvas.getContext('2d')!
  const ctx = outCtx
  ctx.imageSmoothingEnabled = true
  ctx.imageSmoothingQuality = 'high'
  return ctx
}

function publishStream(canvas: HTMLCanvasElement): void {
  if (published) return
  published = true
  emits('stream', canvas.captureStream(STREAM_FPS))
}

function composite(canvas: HTMLCanvasElement, video: HTMLVideoElement): void {
  const w = video.videoWidth
  const h = video.videoHeight
  const ctx = prepareCanvas(canvas, w, h)

  ctx.globalCompositeOperation = 'source-over'
  ctx.clearRect(0, 0, w, h)
  ctx.drawImage(video, 0, 0, w, h)

  ctx.globalCompositeOperation = 'destination-in'
  ctx.drawImage(maskCanvas, 0, 0, w, h)

  ctx.globalCompositeOperation = 'destination-over'
  drawBackground(ctx, video, w, h)

  ctx.globalCompositeOperation = 'source-over'

  if (!published) model.value.mask_ready = true
  publishStream(canvas)
}

function fallbackComposite(canvas: HTMLCanvasElement, video: HTMLVideoElement): void {
  const w = video.videoWidth
  const h = video.videoHeight
  const ctx = prepareCanvas(canvas, w, h)

  ctx.globalCompositeOperation = 'source-over'
  ctx.clearRect(0, 0, w, h)

  const bg = backgroundRef.value
  if (bg.type === 'blur' && useFallbackBlur) {
    // Simple canvas blur – no mask needed.
    drawBlurred(ctx, blurCtx, blurCanvas, video, w, h, bg.radius)
  } else {
    // Raw video until the segmentation mask is ready.
    ctx.drawImage(video, 0, 0, w, h)
  }

  // Emit early so peers receive a continuous stream while the model loads.
  publishStream(canvas)
}

function paintRaw(canvas: HTMLCanvasElement, video: HTMLVideoElement): void {
  try {
    const w = video.videoWidth
    const h = video.videoHeight
    const ctx = prepareCanvas(canvas, w, h)
    ctx.globalCompositeOperation = 'source-over'
    ctx.clearRect(0, 0, w, h)
    ctx.drawImage(video, 0, 0, w, h)
  } catch (e) {
    logThrottled('paintRaw failed', e)
  }
}

function render(now: number): void {
  const video = videoEl
  const canvas = canvasRef.value
  if (disposed || !video || !canvas) return
  if (!video.videoWidth || video.readyState < 2) return

  lastPaint = now

  try {
    const hasSegmenter = !!segmenter

    if (hasSegmenter && now - lastRun >= SEGMENT_INTERVAL) {
      lastRun = now
      try {
        segment(video)
      } catch (e) {
        logThrottled('segment failed', e)
      }
    }

    if (maskReady && hasSegmenter) composite(canvas, video)
    else fallbackComposite(canvas, video)
  } catch (e) {
    logThrottled('render failed', e)
    paintRaw(canvas, video)
  }
}

function onFrame(now: number): void {
  const video = videoEl
  if (!video) return

  frameCb = video.requestVideoFrameCallback(onFrame)

  if (now - lastPaint < MIN_FRAME_MS) return
  render(now)
}

function stopRender(): void {
  if (videoEl && frameCb) videoEl.cancelVideoFrameCallback(frameCb)
  frameCb = 0
  videoEl = null
}

function startRender(video: HTMLVideoElement): void {
  stopRender()
  videoEl = video
  frameCb = video.requestVideoFrameCallback(onFrame)
}

watch(
  () => props.video,
  (video) => {
    if (video) startRender(video)
    else stopRender()
  },
  { immediate: true },
)

onMounted(() => {
  initFilters().catch((e: unknown) => console.error('opencv init error:', e))
  initSegmenter().catch((e: unknown) => console.error('segmenter init error:', e))

  watchdog = window.setInterval(() => {
    const now = performance.now()
    if (now - lastPaint > WATCHDOG_MS * 1.5) render(now)
  }, WATCHDOG_MS)
})

onBeforeUnmount(() => {
  disposed = true
  clearInterval(watchdog)
  stopRender()
  segmenter?.close()
  segmenter = null
  model.value.mask_ready = false
  disposeOpenCV()
})
</script>

<template>
  <canvas ref="canvasRef" class="absolute inset-0 w-full h-full object-cover" />
</template>

<style scoped></style>
