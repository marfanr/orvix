const BLUR_DOWNSCALE = 4
const TEMPORAL = 0.6
const SHARPEN = 1.5

const MASK_OFF = 0

const SUPPORTS_CTX_FILTER =
  typeof OffscreenCanvasRenderingContext2D !== 'undefined' &&
  'filter' in OffscreenCanvasRenderingContext2D.prototype

type Ctx2D = OffscreenCanvasRenderingContext2D

const KSIZE = 7
const HALF = (KSIZE - 1) >> 1
const SIGMA = 0.3 * ((KSIZE - 1) * 0.5 - 1) + 0.8
const KERNEL = (() => {
  const k = new Float32Array(KSIZE)
  let sum = 0
  for (let i = 0; i < KSIZE; i++) {
    const x = i - HALF
    k[i] = Math.exp(-(x * x) / (2 * SIGMA * SIGMA))
    sum += k[i]!
  }
  for (let i = 0; i < KSIZE; i++) k[i]! /= sum
  return k
})()

// --- buffer mask (dialokasi ulang hanya kalau ukuran berubah) ---
let bufW = 0
let bufH = 0
let a = new Uint8Array(0)
let b = new Uint8Array(0)
let f = new Float32Array(0)
let blurred = new Uint8Array(0)
let prev = new Uint8Array(0)
let hasPrev = false

function ensureBuffers(w: number, h: number): void {
  if (bufW === w && bufH === h) return
  const n = w * h
  a = new Uint8Array(n)
  b = new Uint8Array(n)
  f = new Float32Array(n)
  blurred = new Uint8Array(n)
  prev = new Uint8Array(n)
  bufW = w
  bufH = h
  hasPrev = false
}

// Erosi/dilatasi 3x3 separable, satu sumbu per pemanggilan. Piksel di luar batas diabaikan.
function morph3(
  src: Uint8Array,
  dst: Uint8Array,
  w: number,
  h: number,
  horizontal: boolean,
  useMin: boolean,
): void {
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      const i = y * w + x
      let v = src[i]!
      if (horizontal) {
        if (x > 0) {
          const n = src[i - 1]!
          v = useMin ? (n < v ? n : v) : n > v ? n : v
        }
        if (x < w - 1) {
          const n = src[i + 1]!
          v = useMin ? (n < v ? n : v) : n > v ? n : v
        }
      } else {
        if (y > 0) {
          const n = src[i - w]!
          v = useMin ? (n < v ? n : v) : n > v ? n : v
        }
        if (y < h - 1) {
          const n = src[i + w]!
          v = useMin ? (n < v ? n : v) : n > v ? n : v
        }
      }
      dst[i] = v
    }
  }
}

// BORDER_REFLECT_101
function reflect(i: number, n: number): number {
  if (i < 0) return -i
  if (i >= n) return 2 * n - 2 - i
  return i
}

function gaussian(src: Uint8Array, dst: Uint8Array, w: number, h: number): void {
  // horizontal: src -> f
  for (let y = 0; y < h; y++) {
    const row = y * w
    for (let x = 0; x < w; x++) {
      let s = 0
      for (let k = -HALF; k <= HALF; k++) {
        s += src[row + reflect(x + k, w)]! * KERNEL[k + HALF]!
      }
      f[row + x] = s
    }
  }
  // vertical: f -> dst
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      let s = 0
      for (let k = -HALF; k <= HALF; k++) {
        s += f[reflect(y + k, h) * w + x]! * KERNEL[k + HALF]!
      }
      dst[y * w + x] = s + 0.5
    }
  }
}

// API lama dipertahankan sebagai no-op supaya import di worker tidak perlu diubah.
export function initOpenCV(): Promise<void> {
  return Promise.resolve()
}

export function isOpenCvReady(): boolean {
  return true
}

export function disposeOpenCV(): void {
  hasPrev = false
}

export function drawBlurred(
  ctx: Ctx2D,
  blurCtx: Ctx2D,
  blurCanvas: OffscreenCanvas,
  source: CanvasImageSource,
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
  blurCtx.drawImage(source, 0, 0, bw, bh)
  if (SUPPORTS_CTX_FILTER) blurCtx.filter = 'none'

  ctx.imageSmoothingEnabled = true
  ctx.imageSmoothingQuality = 'high'
  ctx.drawImage(blurCanvas, 0, 0, w, h)
}

export function drawCover(ctx: Ctx2D, img: ImageBitmap, w: number, h: number): void {
  const iw = img.width
  const ih = img.height
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
  ensureBuffers(w, h)
  const n = w * h

  // label 0 = orang -> 255
  for (let i = 0; i < n; i++) a[i] = labels[i] === 0 ? 255 : 0

  // morphological open (erode lalu dilate), hasil akhir di `a`
  morph3(a, b, w, h, true, true)
  morph3(b, a, w, h, false, true)
  morph3(a, b, w, h, true, false)
  morph3(b, a, w, h, false, false)

  gaussian(a, blurred, w, h)

  // blend temporal
  if (hasPrev) {
    for (let i = 0; i < n; i++) {
      blurred[i] = blurred[i]! * (1 - TEMPORAL) + prev[i]! * TEMPORAL + 0.5
    }
  }
  prev.set(blurred)
  hasPrev = true

  // kontras di sekitar 128, lalu tulis ke alpha (little-endian: byte tertinggi)
  const offset = 128 * (1 - SHARPEN)
  for (let i = 0; i < n; i++) {
    let v = blurred[i]! * SHARPEN + offset
    v = v < 0 ? 0 : v > 255 ? 255 : v
    out[i] = ((v + 0.5) << 24) >>> 0 || MASK_OFF
  }
}
