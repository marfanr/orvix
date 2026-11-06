import cvModule from '@techstark/opencv-js'

const BLUR_KSIZE = 7
const BLUR_DOWNSCALE = 4
const TEMPORAL = 0.8
const SHARPEN = 1.5

const MASK_ON = 0xff000000
const MASK_OFF = 0

const SUPPORTS_CTX_FILTER =
    typeof CanvasRenderingContext2D !== 'undefined' && 'filter' in CanvasRenderingContext2D.prototype

let cv: any = null
let kernel: any = null
let blurSize: any = null

let src: any = null
let tmp: any = null
let prev: any = null
let blur: any = null
let zero: any = null
let alphaMat: any = null
let rgba: any = null
let planes: any = null
let matW = 0
let matH = 0
let hasPrev = false

async function getOpenCv(): Promise<any> {
  const mod: any = (cvModule as any) instanceof Promise ? await (cvModule as any) : cvModule

  if (!mod.Mat) {
    await new Promise<void>((resolve) => {
      mod.onRuntimeInitialized = resolve
    })
  }

  return mod
}

export async function initOpenCV(): Promise<void> {
  if (cv) return

  cv = await getOpenCv()

  kernel = cv.getStructuringElement(cv.MORPH_RECT, new cv.Size(3, 3))
  blurSize = new cv.Size(BLUR_KSIZE, BLUR_KSIZE)
}

function releaseMats(): void {
  for (const m of [src, tmp, prev, blur, zero, alphaMat, rgba, planes]) m?.delete()
  src = tmp = prev = blur = zero = alphaMat = rgba = planes = null
  hasPrev = false
}

export function disposeOpenCV(): void {
  if (!cv) return
  releaseMats()
  kernel.delete()
  cv = kernel = blurSize = null
}

function ensureMats(w: number, h: number): void {
  if (src && matW === w && matH === h) return

  releaseMats()
  src = new cv.Mat(h, w, cv.CV_8UC1)
  tmp = new cv.Mat(h, w, cv.CV_8UC1)
  prev = new cv.Mat(h, w, cv.CV_8UC1)
  blur = new cv.Mat(h, w, cv.CV_8UC1)
  zero = cv.Mat.zeros(h, w, cv.CV_8UC1)
  alphaMat = new cv.Mat(h, w, cv.CV_8UC1)
  rgba = new cv.Mat(h, w, cv.CV_8UC4)
  planes = new cv.MatVector()
  planes.push_back(zero)
  planes.push_back(zero)
  planes.push_back(zero)
  planes.push_back(alphaMat)
  matW = w
  matH = h
}

export function drawBlurred(
    ctx: CanvasRenderingContext2D,
    blurCtx: CanvasRenderingContext2D,
    blurCanvas: HTMLCanvasElement,
    video: HTMLVideoElement,
    w: number,
    h: number,
    radius: number,
): void {
  const bw = Math.max(1, Math.round(w / BLUR_DOWNSCALE))
  const bh = Math.max(1, Math.round(h / BLUR_DOWNSCALE))

  if (blurCanvas.width !== bw || blurCanvas.height !== bh) {
    blurCanvas.width = bw
    blurCanvas.height = bh
  }

  if (SUPPORTS_CTX_FILTER) blurCtx.filter = `blur(${radius / BLUR_DOWNSCALE}px)`
  blurCtx.drawImage(video, 0, 0, bw, bh)
  if (SUPPORTS_CTX_FILTER) blurCtx.filter = 'none'

  ctx.imageSmoothingEnabled = true
  ctx.imageSmoothingQuality = 'high'
  ctx.drawImage(blurCanvas, 0, 0, w, h)
}

export function drawCover(
    ctx: CanvasRenderingContext2D,
    img: HTMLImageElement | ImageBitmap,
    w: number,
    h: number,
): void {
  const iw = img instanceof HTMLImageElement ? img.naturalWidth : img.width
  const ih = img instanceof HTMLImageElement ? img.naturalHeight : img.height
  if (!iw || !ih) return

  const scale = Math.max(w / iw, h / ih)
  const dw = iw * scale
  const dh = ih * scale
  ctx.drawImage(img, (w - dw) / 2, (h - dh) / 2, dw, dh)
}

export function writeSmoothedMask(
    labels: Uint8Array,
    out: Uint32Array,
    w: number,
    h: number,
): void {
  if (!cv) {
    for (let i = 0; i < labels.length; i++) out[i] = labels[i] === 0 ? MASK_ON : MASK_OFF
    return
  }

  ensureMats(w, h)

  src.data.set(labels)
  cv.threshold(src, src, 0, 255, cv.THRESH_BINARY_INV)

  cv.morphologyEx(src, tmp, cv.MORPH_OPEN, kernel)
  cv.GaussianBlur(tmp, blur, blurSize, 0, 0, cv.BORDER_DEFAULT)

  if (hasPrev) cv.addWeighted(blur, 1 - TEMPORAL, prev, TEMPORAL, 0, blur)
  ;[prev, blur] = [blur, prev]
  hasPrev = true

  prev.convertTo(alphaMat, -1, SHARPEN, 128 * (1 - SHARPEN))
  cv.merge(planes, rgba)

  const d = rgba.data
  out.set(new Uint32Array(d.buffer, d.byteOffset, w * h))
}
