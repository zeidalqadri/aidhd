import type { Bounds } from '../geometry'
import { makeCanvas } from '../decode'

type AnyCanvas = OffscreenCanvas | HTMLCanvasElement
type Ctx2D = OffscreenCanvasRenderingContext2D | CanvasRenderingContext2D

function ctxOf(c: AnyCanvas, willRead = false): Ctx2D {
  const ctx = c.getContext('2d', willRead ? { willReadFrequently: true } : undefined) as Ctx2D | null
  if (!ctx) throw new Error('2D canvas context unavailable')
  return ctx
}

export const WORKING_MAX_EDGE = 2048

/** Downscale a bitmap to a memory-safe working size, returning its pixels. */
export function prepareWorkingImage(
  bitmap: ImageBitmap,
  maxEdge = WORKING_MAX_EDGE,
): ImageData {
  const longest = Math.max(bitmap.width, bitmap.height)
  const scale = longest > maxEdge ? maxEdge / longest : 1
  const w = Math.max(1, Math.round(bitmap.width * scale))
  const h = Math.max(1, Math.round(bitmap.height * scale))
  const canvas = makeCanvas(w, h)
  const ctx = ctxOf(canvas, true)
  ctx.imageSmoothingEnabled = true
  ctx.imageSmoothingQuality = 'high'
  ctx.drawImage(bitmap, 0, 0, w, h)
  return ctx.getImageData(0, 0, w, h)
}

/**
 * Frame the composed sticker into a square WhatsApp canvas: crop to the cut
 * bounds, scale to fit `size` minus a transparent margin, and center.
 */
export function frameToSquare(
  source: AnyCanvas,
  contentBounds: Bounds,
  size = 512,
  marginPx = 8,
): AnyCanvas {
  const out = makeCanvas(size, size)
  const ctx = ctxOf(out)
  ctx.imageSmoothingEnabled = true
  ctx.imageSmoothingQuality = 'high'

  const avail = size - marginPx * 2
  const b = clampBounds(contentBounds, source.width, source.height)
  const scale = Math.min(avail / b.width, avail / b.height)
  const dw = b.width * scale
  const dh = b.height * scale
  const dx = (size - dw) / 2
  const dy = (size - dh) / 2
  ctx.drawImage(source as CanvasImageSource, b.x, b.y, b.width, b.height, dx, dy, dw, dh)
  return out
}

function clampBounds(b: Bounds, w: number, h: number): Bounds {
  const x = Math.max(0, Math.floor(b.x))
  const y = Math.max(0, Math.floor(b.y))
  const width = Math.min(w - x, Math.ceil(b.width + (b.x - x)))
  const height = Math.min(h - y, Math.ceil(b.height + (b.y - y)))
  return { x, y, width: Math.max(1, width), height: Math.max(1, height) }
}

export async function canvasToBlob(
  canvas: AnyCanvas,
  type: string,
  quality?: number,
): Promise<Blob> {
  if ('convertToBlob' in canvas) {
    return canvas.convertToBlob({ type, quality })
  }
  return new Promise((resolve, reject) => {
    ;(canvas as HTMLCanvasElement).toBlob(
      (b) => (b ? resolve(b) : reject(new Error('toBlob failed'))),
      type,
      quality,
    )
  })
}
