// SPDX-FileCopyrightText: 2026 Mohammad Arfan
//
// SPDX-License-Identifier: AGPL-3.0
/// <reference lib="webworker" />

import { FilesetResolver, ImageSegmenter } from '@mediapipe/tasks-vision'
import type { Background } from '@/types/video-call.ts'
import {
  disposeOpenCV,
  drawBlurred,
  drawCover,
  initOpenCV,
  writeSmoothedMask,
} from '@/lib/filter.ts'

type InMsg =
  | {
      func: 'start_worker'
      background: Background
      enabled: boolean
      writable: WritableStream<VideoFrame>
      mirrored: boolean
    }
  | { func: 'set_source'; readable: ReadableStream<VideoFrame> }
  | { func: 'set_background'; background: Background; enabled: boolean; mirrored: boolean }
  | { func: 'stop_worker' }

type OutMsg = { func: 'published' } | { func: 'mask_ready' }

const post = (m: OutMsg) => self.postMessage(m)

const WASM_URL = 'https://cdn.jsdelivr.net/npm/@mediapipe/tasks-vision@0.10.21/wasm'
const SELFIE_MODEL_URL =
  'https://storage.googleapis.com/mediapipe-models/image_segmenter/selfie_segmenter_landscape/float16/latest/selfie_segmenter_landscape.tflite'
const SEGMENT_WIDTH = 256
const SEGMENT_FPS = 15
const SEGMENT_INTERVAL = 1000 / SEGMENT_FPS - 2
const FALLBACK_DELEGATE = 'CPU'

const inputCanvas = new OffscreenCanvas(1, 1)
const inputCtx = inputCanvas.getContext('2d', { alpha: false })!
const maskCanvas = new OffscreenCanvas(1, 1)
const maskCtx = maskCanvas.getContext('2d')!
const blurCanvas = new OffscreenCanvas(1, 1)
const blurCtx = blurCanvas.getContext('2d', { alpha: false })!
const outCanvas = new OffscreenCanvas(1, 1)
const outCtx = outCanvas.getContext('2d')!

let writable: WritableStream<VideoFrame> | null = null
let pipeAbort: AbortController | null = null
let background: Background = { type: 'blur', radius: 10 }
let enabled = true
let mirrored = false

let segmenter: ImageSegmenter | null = null
let useFallbackBlur = false
let maskImage: ImageData | null = null
let mask32: Uint32Array | null = null
let lastRun = 0
let lastErrLog = 0
let maskReady = false
let maskReadySent = false
let initDone = false
let published = false
let disposed = false
let t0 = 0

const since = () => Math.round(performance.now() - t0)

function logThrottled(label: string, e: unknown): void {
  const t = performance.now()
  if (t - lastErrLog < 2000) return
  lastErrLog = t
  console.error(`[video.worker] ${label}`, e)
}

self.onmessage = (e: MessageEvent<InMsg>) => {
  const msg = e.data
  if (!msg) return

  switch (msg.func) {
    case 'start_worker':
      writable = msg.writable
      background = msg.background
      enabled = msg.enabled
      mirrored = msg.mirrored
      startWorker()
      break
    case 'set_source':
      attachSource(msg.readable)
      break
    case 'set_background':
      background = msg.background
      enabled = msg.enabled
      mirrored = msg.mirrored
      break
    case 'stop_worker':
      stopWorker()
      break
  }
}

async function ensureModuleFactory(loaderUrl: string): Promise<void> {
  const g = self as any
  if (typeof g.ModuleFactory === 'function') return

  const res = await fetch(loaderUrl)
  if (!res.ok) throw new Error(`failed to fetch wasm loader: ${res.status} ${loaderUrl}`)
  const code = await res.text()

  ;(0, eval)(
    `${code}\n;self.ModuleFactory = typeof ModuleFactory !== 'undefined' ? ModuleFactory : self.ModuleFactory;`,
  )

  if (typeof g.ModuleFactory !== 'function') {
    throw new Error('wasm loader evaluated but ModuleFactory is still not set')
  }
}

async function fetchModel(): Promise<ArrayBuffer> {
  const res = await fetch(SELFIE_MODEL_URL)
  if (!res.ok) throw new Error(`failed to fetch model: ${res.status}`)
  return res.arrayBuffer()
}

async function resolveVision() {
  const vision = await FilesetResolver.forVisionTasks(WASM_URL)
  await ensureModuleFactory(vision.wasmLoaderPath)
  return vision
}

function warmUp(instance: ImageSegmenter): void {
  try {
    const width = SEGMENT_WIDTH
    const height = Math.round((SEGMENT_WIDTH * 9) / 16)
    inputCanvas.width = width
    inputCanvas.height = height
    inputCtx.fillStyle = '#808080'
    inputCtx.fillRect(0, 0, width, height)
    instance.segmentForVideo(inputCanvas, performance.now(), () => {})
  } catch (e) {
    console.warn('[video.worker] warm-up failed', e)
  }
}

async function initSegmenter(): Promise<void> {
  // loaded parallel
  const [vision, modelBuf] = await Promise.all([resolveVision(), fetchModel()])
  console.log(`[video.worker] assets ready +${since()}ms`)

  let instance: ImageSegmenter | null = null
  let lastError: unknown = null

  for (const delegate of ['GPU', FALLBACK_DELEGATE] as const) {
    try {
      instance = await ImageSegmenter.createFromOptions(vision, {
        baseOptions: { modelAssetBuffer: new Uint8Array(modelBuf), delegate },
        runningMode: 'VIDEO',
        outputCategoryMask: true,
        outputConfidenceMasks: false,
      })
      console.log(`[video.worker] segmenter delegate: ${delegate} +${since()}ms`)
      useFallbackBlur = false
      break
    } catch (e) {
      lastError = e
      console.warn(`[video.worker] segmenter init failed with ${delegate}:`, e)
    }
  }

  if (disposed) {
    instance?.close()
    return
  }

  if (!instance) {
    console.error('[video.worker] segmenter init failed on all delegates', lastError)
    segmenter = null
    useFallbackBlur = true
    return
  }

  warmUp(instance)
  segmenter = instance
  console.log(`[video.worker] warm-up done +${since()}ms`)
}

async function initFilters(): Promise<void> {
  await initOpenCV()
  console.log(`[video.worker] opencv ready +${since()}ms`)
  if (disposed) disposeOpenCV()
}

function drawSegmenterInput(frame: VideoFrame): void {
  const width = SEGMENT_WIDTH
  const height = Math.max(1, Math.round((SEGMENT_WIDTH * frame.displayHeight) / frame.displayWidth))

  if (inputCanvas.width !== width || inputCanvas.height !== height) {
    inputCanvas.width = width
    inputCanvas.height = height
  }
  inputCtx.drawImage(frame, 0, 0, width, height)
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

function segment(frame: VideoFrame): void {
  drawSegmenterInput(frame)

  segmenter!.segmentForVideo(inputCanvas, performance.now(), (result) => {
    const mask = result.categoryMask
    if (!mask) return

    updateMaskCanvas(mask.getAsUint8Array(), mask.width, mask.height)
    maskReady = true
  })
}

function drawBackground(frame: VideoFrame, w: number, h: number): void {
  const bg = background
  switch (bg.type) {
    case 'blur':
      drawBlurred(outCtx, blurCtx, blurCanvas, frame, w, h, bg.radius)
      break
    case 'image':
      outCtx.save()
      outCtx.setTransform(1, 0, 0, 1, 0, 0)
      drawCover(outCtx, bg.image as ImageBitmap, w, h)
      outCtx.restore()
      break
    case 'color':
      outCtx.fillStyle = bg.value
      outCtx.fillRect(0, 0, w, h)
      break
  }
}

function prepareCanvas(w: number, h: number): void {
  if (outCanvas.width !== w || outCanvas.height !== h) {
    outCanvas.width = w
    outCanvas.height = h
  }
  outCtx.imageSmoothingEnabled = true
  outCtx.imageSmoothingQuality = 'high'
}

function markMaskReady(): void {
  if (maskReadySent) return
  maskReadySent = true
  post({ func: 'mask_ready' })
}

function markPublished(): void {
  if (published) return
  published = true
  console.log(`[video.worker] published +${since()}ms`)
  post({ func: 'published' })
}

function composite(frame: VideoFrame, w: number, h: number): void {
  prepareCanvas(w, h)
  const ctx = outCtx

  ctx.globalCompositeOperation = 'source-over'
  ctx.clearRect(0, 0, w, h)
  ctx.drawImage(frame, 0, 0, w, h)

  ctx.globalCompositeOperation = 'destination-in'
  ctx.drawImage(maskCanvas, 0, 0, w, h)

  ctx.globalCompositeOperation = 'destination-over'
  drawBackground(frame, w, h)

  ctx.globalCompositeOperation = 'source-over'

  markMaskReady()
}

function fallbackComposite(frame: VideoFrame, w: number, h: number): void {
  prepareCanvas(w, h)
  const ctx = outCtx

  ctx.globalCompositeOperation = 'source-over'
  ctx.clearRect(0, 0, w, h)

  if (background.type === 'blur' && useFallbackBlur) {
    drawBlurred(ctx, blurCtx, blurCanvas, frame, w, h, background.radius)
  } else {
    ctx.drawImage(frame, 0, 0, w, h)
  }
}

function mirrorOnly(frame: VideoFrame, w: number, h: number): void {
  prepareCanvas(w, h)
  outCtx.globalCompositeOperation = 'source-over'
  outCtx.setTransform(-1, 0, 0, 1, w, 0)
  outCtx.drawImage(frame, 0, 0, w, h)
  outCtx.setTransform(1, 0, 0, 1, 0, 0)
}

function render(frame: VideoFrame): VideoFrame | null {
  const w = frame.displayWidth
  const h = frame.displayHeight
  if (!w || !h) return null

  prepareCanvas(w, h)

  if (!enabled) {
    mirrorOnly(frame, w, h)
  } else {
    const now = performance.now()
    if (segmenter && now - lastRun >= SEGMENT_INTERVAL) {
      lastRun = now
      try {
        segment(frame)
      } catch (e) {
        logThrottled('segment failed', e)
      }
    }

    if (mirrored) outCtx.setTransform(-1, 0, 0, 1, w, 0)
    try {
      if (maskReady && segmenter) composite(frame, w, h)
      else fallbackComposite(frame, w, h)
    } finally {
      outCtx.setTransform(1, 0, 0, 1, 0, 0)
    }
  }

  return new VideoFrame(outCanvas, {
    timestamp: frame.timestamp,
    duration: frame.duration ?? undefined,
  })
}

function makeTransform(): TransformStream<VideoFrame, VideoFrame> {
  return new TransformStream<VideoFrame, VideoFrame>({
    transform(frame, controller) {
      try {
        if (disposed) return

        if (!enabled && !mirrored) {
          controller.enqueue(frame.clone())
          markMaskReady()
          markPublished()
          return
        }

        if (enabled && !initDone) return

        const out = render(frame)
        if (!out) return
        controller.enqueue(out)

        if (!enabled) {
          markMaskReady()
          markPublished()
        } else if (maskReady || useFallbackBlur) {
          markPublished()
        }
      } catch (e) {
        logThrottled('render failed', e)
        if (initDone || !enabled) {
          try {
            controller.enqueue(frame.clone())
          } catch {}
        }
      } finally {
        frame.close()
      }
    },
  })
}

function attachSource(readable: ReadableStream<VideoFrame>): void {
  if (disposed || !writable) {
    readable.cancel().catch(() => {})
    return
  }

  pipeAbort?.abort()
  const ac = new AbortController()
  pipeAbort = ac

  // preventClose/preventAbort
  readable
    .pipeThrough(makeTransform(), { signal: ac.signal })
    .pipeTo(writable, { preventClose: true, preventAbort: true })
    .catch((e: unknown) => {
      if (!ac.signal.aborted) logThrottled('pipeline error', e)
    })
}

function startWorker(): void {
  t0 = performance.now()

  initSegmenter()
    .catch((e: unknown) => {
      console.error('segmenter init error:', e)
      segmenter = null
      useFallbackBlur = true
    })
    .finally(() => {
      initDone = true
      if (disposed) return
      setTimeout(() => {
        initFilters().catch((e: unknown) => console.error('opencv init error:', e))
      }, 0)
    })
}

function stopWorker(): void {
  disposed = true
  pipeAbort?.abort()
  segmenter?.close()
  segmenter = null
  disposeOpenCV()
  self.close()
}
