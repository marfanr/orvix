// SPDX-FileCopyrightText: 2026 Mohammad Arfan
//
// SPDX-License-Identifier: AGPL-3.0
/// <reference lib="webworker" />

import { FilesetResolver, ImageSegmenter } from '@mediapipe/tasks-vision'
import type { Background } from '@/types/video-call.ts'
import { drawBlurred, drawCover, writeSmoothedMask } from '@/lib/filter.ts'

type Src = VideoFrame | ImageBitmap

type InMsg =
  | {
      func: 'start_worker'
      background: Background
      enabled: boolean
      canvas?: OffscreenCanvas
      writable?: WritableStream<VideoFrame>
      mirrored: boolean
    }
  | { func: 'set_source'; readable: ReadableStream<VideoFrame> }
  | { func: 'frame'; bitmap: ImageBitmap }
  | { func: 'ticker'; on: boolean; fps: number }
  | { func: 'set_background'; background: Background; enabled: boolean; mirrored: boolean }
  | { func: 'stop_worker' }

type OutMsg =
  | { func: 'published' }
  | { func: 'mask_ready' }
  | { func: 'frame_done' }
  | { func: 'tick' }

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

let outCanvas: OffscreenCanvas | null = null
let outCtx: OffscreenCanvasRenderingContext2D | null = null
let writer: WritableStreamDefaultWriter<VideoFrame> | null = null

let pipeAbort: AbortController | null = null
let tickId: ReturnType<typeof setInterval> | null = null
let background: Background = { type: 'blur', radius: 10 }
let enabled = true
let mirrored = false

let segmenter: ImageSegmenter | null = null
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

const dims = (s: Src): [number, number] =>
  'displayWidth' in s ? [s.displayWidth, s.displayHeight] : [s.width, s.height]

function logThrottled(label: string, e: unknown): void {
  const t = performance.now()
  if (t - lastErrLog < 2000) return
  lastErrLog = t
  console.error(`[video.worker] ${label}`, e)
}

function setTicker(on: boolean, fps: number): void {
  if (tickId !== null) {
    clearInterval(tickId)
    tickId = null
  }
  if (on) tickId = setInterval(() => post({ func: 'tick' }), 1000 / fps)
}

function setupSink(msg: Extract<InMsg, { func: 'start_worker' }>): void {
  outCanvas = msg.canvas ?? new OffscreenCanvas(1, 1)
  outCtx = outCanvas.getContext('2d')!

  if (msg.writable) {
    writer = msg.writable.getWriter()
    return
  }

  const VTG = (self as any).VideoTrackGenerator
  if (typeof VTG === 'function') {
    try {
      const gen = new VTG()
      writer = gen.writable.getWriter()
      self.postMessage({ func: 'track', track: gen.track }, [gen.track])
    } catch (e) {
      writer = null
      console.warn('[video.worker] VideoTrackGenerator unavailable', e)
    }
  }
}

self.onmessage = (e: MessageEvent<InMsg>) => {
  const msg = e.data
  if (!msg) return

  switch (msg.func) {
    case 'start_worker':
      setupSink(msg)
      background = msg.background
      enabled = msg.enabled
      mirrored = msg.mirrored
      startWorker()
      break
    case 'set_source':
      attachSource(msg.readable)
      break
    case 'frame':
      void renderBitmap(msg.bitmap)
      break
    case 'ticker':
      setTicker(msg.on, msg.fps)
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
  const [vision, modelBuf] = await Promise.all([resolveVision(), fetchModel()])

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
    return
  }

  warmUp(instance)
  segmenter = instance
}

function drawSegmenterInput(src: Src, srcW: number, srcH: number): void {
  const width = SEGMENT_WIDTH
  const height = Math.max(1, Math.round((SEGMENT_WIDTH * srcH) / srcW))

  if (inputCanvas.width !== width || inputCanvas.height !== height) {
    inputCanvas.width = width
    inputCanvas.height = height
  }
  inputCtx.drawImage(src, 0, 0, width, height)
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

function segment(src: Src, w: number, h: number): void {
  drawSegmenterInput(src, w, h)

  segmenter!.segmentForVideo(inputCanvas, performance.now(), (result) => {
    const mask = result.categoryMask
    if (!mask) return

    updateMaskCanvas(mask.getAsUint8Array(), mask.width, mask.height)
    maskReady = true
  })
}

function drawBackground(src: Src, w: number, h: number): void {
  const ctx = outCtx!
  const bg = background
  switch (bg.type) {
    case 'blur':
      drawBlurred(ctx, blurCtx, blurCanvas, src, w, h, bg.radius)
      break
    case 'image':
      ctx.save()
      ctx.setTransform(1, 0, 0, 1, 0, 0)
      drawCover(ctx, bg.image as ImageBitmap, w, h)
      ctx.restore()
      break
    case 'color':
      ctx.fillStyle = bg.value
      ctx.fillRect(0, 0, w, h)
      break
  }
}

function prepareCanvas(w: number, h: number): void {
  const canvas = outCanvas!
  if (canvas.width !== w || canvas.height !== h) {
    canvas.width = w
    canvas.height = h
  }
  outCtx!.imageSmoothingEnabled = true
  outCtx!.imageSmoothingQuality = 'high'
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

function composite(src: Src, w: number, h: number): void {
  prepareCanvas(w, h)
  const ctx = outCtx!

  ctx.globalCompositeOperation = 'source-over'
  ctx.clearRect(0, 0, w, h)
  ctx.drawImage(src, 0, 0, w, h)

  ctx.globalCompositeOperation = 'destination-in'
  ctx.drawImage(maskCanvas, 0, 0, w, h)

  ctx.globalCompositeOperation = 'destination-over'
  drawBackground(src, w, h)

  ctx.globalCompositeOperation = 'source-over'

  markMaskReady()
}

function fallbackComposite(src: Src, w: number, h: number): void {
  prepareCanvas(w, h)
  const ctx = outCtx!

  ctx.globalCompositeOperation = 'source-over'
  ctx.clearRect(0, 0, w, h)

  switch (background.type) {
    case 'blur':
      drawBlurred(ctx, blurCtx, blurCanvas, src, w, h, background.radius)
      break
    case 'color':
      ctx.fillStyle = background.value
      ctx.fillRect(0, 0, w, h)
      break
    default:
      ctx.drawImage(src, 0, 0, w, h)
  }
}

function passthrough(src: Src, w: number, h: number): void {
  prepareCanvas(w, h)
  outCtx!.globalCompositeOperation = 'source-over'
  outCtx!.drawImage(src, 0, 0, w, h)
}

function mirrorOnly(src: Src, w: number, h: number): void {
  prepareCanvas(w, h)
  const ctx = outCtx!
  ctx.globalCompositeOperation = 'source-over'
  ctx.setTransform(-1, 0, 0, 1, w, 0)
  ctx.drawImage(src, 0, 0, w, h)
  ctx.setTransform(1, 0, 0, 1, 0, 0)
}

function render(src: Src): boolean {
  const [w, h] = dims(src)
  if (!w || !h || !outCtx) return false

  prepareCanvas(w, h)

  if (!enabled) {
    if (mirrored) mirrorOnly(src, w, h)
    else passthrough(src, w, h)
    markMaskReady()
  } else {
    const now = performance.now()
    if (segmenter && now - lastRun >= SEGMENT_INTERVAL) {
      lastRun = now
      try {
        segment(src, w, h)
      } catch (e) {
        logThrottled('segment failed', e)
      }
    }

    if (mirrored) outCtx.setTransform(-1, 0, 0, 1, w, 0)
    try {
      if (maskReady && segmenter) composite(src, w, h)
      else fallbackComposite(src, w, h)
    } finally {
      outCtx.setTransform(1, 0, 0, 1, 0, 0)
    }
  }

  markPublished()
  return true
}

function safeRender(src: Src): boolean {
  if (disposed) return false
  try {
    return render(src)
  } catch (e) {
    logThrottled('render failed', e)
    if (initDone || !enabled) {
      try {
        const [w, h] = dims(src)
        passthrough(src, w, h)
        return true
      } catch {}
    }
    return false
  }
}

async function emit(timestamp: number, duration?: number | null): Promise<void> {
  if (!writer || !outCanvas || disposed) return

  let vf: VideoFrame | null = null
  try {
    vf = new VideoFrame(outCanvas, { timestamp, duration: duration ?? undefined })
    await writer.write(vf)
  } catch (e) {
    try {
      vf?.close()
    } catch {}
    logThrottled('emit failed', e)
  }
}

function makeSink(): WritableStream<VideoFrame> {
  return new WritableStream<VideoFrame>({
    async write(frame) {
      try {
        if (safeRender(frame)) await emit(frame.timestamp, frame.duration)
      } finally {
        frame.close()
      }
    },
  })
}

function attachSource(readable: ReadableStream<VideoFrame>): void {
  if (disposed || !outCtx) {
    readable.cancel().catch(() => {})
    return
  }

  pipeAbort?.abort()
  const ac = new AbortController()
  pipeAbort = ac

  readable.pipeTo(makeSink(), { signal: ac.signal }).catch((e: unknown) => {
    if (!ac.signal.aborted) logThrottled('pipeline error', e)
  })
}

async function renderBitmap(bmp: ImageBitmap): Promise<void> {
  try {
    if (safeRender(bmp)) await emit(Math.round(performance.now() * 1000))
  } finally {
    bmp.close()
    post({ func: 'frame_done' })
  }
}

function startWorker(): void {
  t0 = performance.now()

  initSegmenter()
    .catch((e: unknown) => {
      console.error('segmenter init error:', e)
      segmenter = null
    })
    .finally(() => {
      initDone = true
    })
}

function stopWorker(): void {
  setTicker(false, 0)
  disposed = true
  pipeAbort?.abort()
  segmenter?.close()
  segmenter = null
  writer?.close().catch(() => {})
  writer = null
  self.close()
}
