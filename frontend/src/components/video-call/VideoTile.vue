<script setup lang="ts">
import {
  type ComponentPublicInstance,
  computed,
  nextTick,
  onBeforeUnmount,
  onMounted,
  ref,
  watch,
} from 'vue'
import { MicIcon, MicOffIcon } from 'lucide-vue-next'
import type { Participant } from '@/types/video-call'
import { FilesetResolver, ImageSegmenter } from '@mediapipe/tasks-vision'
import {
  disposeOpenCV,
  drawBlurred,
  drawCover,
  initOpenCV,
  writeSmoothedMask,
} from '@/lib/filter.ts'

type Background =
  | { type: 'color'; value: string }
  | { type: 'image'; image: HTMLImageElement | ImageBitmap }
  | { type: 'blur'; radius: number }

const props = defineProps<{
  participant: Participant
  tileClass?: string
}>()

const LOCAL_PARTICIPANT_ID = 0
const WASM_URL = 'https://cdn.jsdelivr.net/npm/@mediapipe/tasks-vision@latest/wasm'
const MODEL_URL =
  'https://storage.googleapis.com/mediapipe-models/image_segmenter/selfie_segmenter_landscape/float16/latest/selfie_segmenter_landscape.tflite'
const SEGMENT_WIDTH = 256
const TARGET_FPS = 30
const MIN_FRAME_INTERVAL = 1000 / TARGET_FPS - 2

const background: Background = { type: 'blur', radius: 32 }

const videoRef = ref<HTMLVideoElement | null>(null)
const audioRef = ref<HTMLAudioElement | null>(null)
const canvasRef = ref<HTMLCanvasElement | null>(null)

const isLocal = computed(() => props.participant.id === LOCAL_PARTICIPANT_ID)
const showCanvas = isLocal.value
const MicStateIcon = computed(() => (props.participant.muted ? MicOffIcon : MicIcon))

const inputCanvas = document.createElement('canvas')
const inputCtx = inputCanvas.getContext('2d', { alpha: false })!
const maskCanvas = document.createElement('canvas')
const maskCtx = maskCanvas.getContext('2d')!
const blurCanvas = document.createElement('canvas')
const blurCtx = blurCanvas.getContext('2d', { alpha: false })!

let segmenter: ImageSegmenter | null = null
let maskImage: ImageData | null = null
let mask32: Uint32Array | null = null
let rafId = 0
let lastVideoTime = -1
let lastRun = 0
let disposed = false

async function initSegmenter(): Promise<void> {
  const vision = await FilesetResolver.forVisionTasks(WASM_URL)
  const instance = await ImageSegmenter.createFromOptions(vision, {
    baseOptions: { modelAssetPath: MODEL_URL, delegate: 'GPU' },
    runningMode: 'VIDEO',
    outputCategoryMask: true,
    outputConfidenceMasks: false,
  })

  if (disposed) {
    instance.close()
    return
  }
  segmenter = instance
  rafId = requestAnimationFrame(segmentLoop)
}

async function initFilters(): Promise<void> {
  await initOpenCV()
  if (disposed) disposeOpenCV()
}

function isNewFrame(video: HTMLVideoElement, now: number): boolean {
  if (video.readyState < 2 || !video.videoWidth) return false
  if (video.currentTime === lastVideoTime) return false
  return now - lastRun >= MIN_FRAME_INTERVAL
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

function drawBackground(
  ctx: CanvasRenderingContext2D,
  video: HTMLVideoElement,
  w: number,
  h: number,
): void {
  switch (background.type) {
    case 'blur':
      drawBlurred(ctx, blurCtx, blurCanvas, video, w, h, background.radius)
      break
    case 'image':
      drawCover(ctx, background.image, w, h)
      break
    case 'color':
      ctx.fillStyle = background.value
      ctx.fillRect(0, 0, w, h)
      break
  }
}

function composite(canvas: HTMLCanvasElement, video: HTMLVideoElement): void {
  const w = video.videoWidth
  const h = video.videoHeight

  if (canvas.width !== w || canvas.height !== h) {
    canvas.width = w
    canvas.height = h
  }

  const ctx = canvas.getContext('2d')!
  ctx.imageSmoothingEnabled = true
  ctx.imageSmoothingQuality = 'high'

  ctx.globalCompositeOperation = 'source-over'
  ctx.clearRect(0, 0, w, h)
  ctx.drawImage(video, 0, 0, w, h)

  ctx.globalCompositeOperation = 'destination-in'
  ctx.drawImage(maskCanvas, 0, 0, w, h)

  ctx.globalCompositeOperation = 'destination-over'
  drawBackground(ctx, video, w, h)

  ctx.globalCompositeOperation = 'source-over'
}

function segmentLoop(now: number): void {
  rafId = requestAnimationFrame(segmentLoop)

  const video = videoRef.value
  const canvas = canvasRef.value
  if (!video || !canvas || !segmenter) return
  if (!isNewFrame(video, now)) return

  lastRun = now
  lastVideoTime = video.currentTime
  drawSegmenterInput(video)

  segmenter.segmentForVideo(inputCanvas, performance.now(), (result) => {
    const mask = result.categoryMask
    if (!mask) return

    updateMaskCanvas(mask.getAsUint8Array(), mask.width, mask.height)
    composite(canvas, video)
  })
}

function attachStream(el: HTMLMediaElement, stream: MediaStream): void {
  if (el.srcObject !== stream) el.srcObject = stream
  el.play().catch((err) => console.warn(`${el.tagName.toLowerCase()} play error:`, err))
}

async function setVideoStream(): Promise<void> {
  await nextTick()
  const { stream } = props.participant
  if (videoRef.value && stream) attachStream(videoRef.value, stream)
}

async function setAudioStream(): Promise<void> {
  await nextTick()
  const { stream, muted } = props.participant
  if (!audioRef.value || !stream) return

  audioRef.value.muted = isLocal.value || muted
  attachStream(audioRef.value, stream)
}

function setVideoRef(el: Element | ComponentPublicInstance | null): void {
  videoRef.value = el instanceof HTMLVideoElement ? el : null
  if (videoRef.value) setVideoStream()
}

function setAudioRef(el: Element | ComponentPublicInstance | null): void {
  audioRef.value = el instanceof HTMLAudioElement ? el : null
  if (audioRef.value) setAudioStream()
}

watch(() => props.participant.stream, setVideoStream)
watch(() => [props.participant.muted, props.participant.stream], setAudioStream)
watch(
  () => props.participant.cameraOff,
  (off) => {
    if (!off) setVideoStream()
  },
)

onMounted(() => {
  if (!showCanvas) return
  initFilters().catch((e: unknown) => console.error('opencv init error:', e))
  initSegmenter().catch((e: unknown) => console.error('segmenter init error:', e))
})

onBeforeUnmount(() => {
  disposed = true
  cancelAnimationFrame(rafId)
  segmenter?.close()
  segmenter = null
  disposeOpenCV()
})
</script>

<template>
  <div
    class="relative overflow-hidden rounded-sm min-w-0 min-h-0 h-full w-full bg-stone-900 text-white flex items-center justify-center"
    :class="tileClass"
  >
    <div
      v-if="participant.cameraOff"
      class="size-16 sm:size-20 md:size-24 shrink-0 rounded-full bg-brown-ribbon-800 flex items-center justify-center"
    >
      <span class="text-white text-lg sm:text-xl md:text-2xl font-medium" aria-hidden="true">
        {{ participant.name[0]?.toUpperCase() }}
      </span>
    </div>

    <div v-else class="absolute inset-0">
      <canvas
        v-if="showCanvas"
        ref="canvasRef"
        class="absolute inset-0 w-full h-full object-cover"
      />
      <video
        :ref="setVideoRef"
        class="w-full h-full bg-black object-cover"
        :class="{ 'opacity-0 pointer-events-none': showCanvas }"
        autoplay
        playsinline
        muted
      ></video>
    </div>

    <audio :ref="setAudioRef" autoplay playsinline muted />

    <div
      class="absolute bottom-1 left-1 sm:bottom-2 sm:left-2 md:bottom-4 md:left-4 p-1 sm:p-1.5 md:p-2 text-[10px] sm:text-xs flex flex-row gap-x-1.5 sm:gap-x-2 md:gap-x-3 items-center bg-zinc-800/60 rounded-md text-white max-w-[calc(100%-0.5rem)] sm:max-w-[calc(100%-1rem)]"
    >
      <span class="truncate">{{ participant.name }}</span>
      <component
        :is="MicStateIcon"
        class="size-3 sm:size-3.5 md:size-[15px]"
        :aria-label="participant.muted ? 'Muted' : 'Unmuted'"
      />
    </div>
  </div>
</template>
