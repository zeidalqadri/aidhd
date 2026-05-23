import { canvasToBlob } from './resize'

type AnyCanvas = OffscreenCanvas | HTMLCanvasElement

const QUALITY_STEPS = [0.92, 0.85, 0.78, 0.7, 0.6, 0.5, 0.4, 0.3]

let canvasWebpSupport: boolean | null = null

async function supportsCanvasWebp(): Promise<boolean> {
  if (canvasWebpSupport !== null) return canvasWebpSupport
  try {
    const c =
      typeof OffscreenCanvas !== 'undefined'
        ? new OffscreenCanvas(2, 2)
        : Object.assign(document.createElement('canvas'), { width: 2, height: 2 })
    const blob = await canvasToBlob(c, 'image/webp', 0.8)
    canvasWebpSupport = blob.type === 'image/webp'
  } catch {
    canvasWebpSupport = false
  }
  return canvasWebpSupport
}

function getImageData(canvas: AnyCanvas): ImageData {
  const ctx = canvas.getContext('2d') as
    | OffscreenCanvasRenderingContext2D
    | CanvasRenderingContext2D
    | null
  if (!ctx) throw new Error('2D canvas context unavailable')
  return ctx.getImageData(0, 0, canvas.width, canvas.height)
}

async function encodeWithJsquash(canvas: AnyCanvas, quality: number): Promise<Blob> {
  const { default: encode } = await import('@jsquash/webp/encode')
  const buf = await encode(getImageData(canvas), { quality: Math.round(quality * 100) })
  return new Blob([buf], { type: 'image/webp' })
}

export interface EncodeResult {
  blob: Blob
  bytes: number
  quality: number
  withinBudget: boolean
}

/**
 * Encode a canvas to WebP under a byte budget by stepping quality down. Uses the
 * native canvas encoder where available (Chrome/Firefox) and the WASM libwebp
 * encoder otherwise (Safari, which cannot encode WebP via canvas).
 */
export async function encodeWebpUnderBudget(
  canvas: AnyCanvas,
  maxBytes = 100 * 1024,
): Promise<EncodeResult> {
  const native = await supportsCanvasWebp()
  const encodeAt = (q: number) =>
    native ? canvasToBlob(canvas, 'image/webp', q) : encodeWithJsquash(canvas, q)

  let last: Blob | null = null
  let lastQ = QUALITY_STEPS[QUALITY_STEPS.length - 1]
  for (const q of QUALITY_STEPS) {
    const blob = await encodeAt(q)
    last = blob
    lastQ = q
    if (blob.size <= maxBytes) {
      return { blob, bytes: blob.size, quality: q, withinBudget: true }
    }
  }
  return {
    blob: last!,
    bytes: last!.size,
    quality: lastQ,
    withinBudget: false,
  }
}
