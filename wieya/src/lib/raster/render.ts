import type { PathsD } from 'clipper2-ts'
import type { Mask } from '../cutpath'
import type { Bounds } from '../geometry'
import { pathsBounds } from '../geometry'
import { makeCanvas } from '../decode'

type AnyCanvas = OffscreenCanvas | HTMLCanvasElement
type Ctx2D = OffscreenCanvasRenderingContext2D | CanvasRenderingContext2D

function ctxOf(c: AnyCanvas): Ctx2D {
  const ctx = c.getContext('2d') as Ctx2D | null
  if (!ctx) throw new Error('2D canvas context unavailable')
  return ctx
}

export function pathsToPath2D(paths: PathsD): Path2D {
  const p = new Path2D()
  for (const ring of paths) {
    ring.forEach((pt, i) => (i === 0 ? p.moveTo(pt.x, pt.y) : p.lineTo(pt.x, pt.y)))
    p.closePath()
  }
  return p
}

/** Source RGB with alpha multiplied by the mask (background removed). */
function maskedSubjectCanvas(source: ImageData, mask: Mask): AnyCanvas {
  const out = new ImageData(source.width, source.height)
  const norm = mask.scale255 ? 1 / 255 : 1
  for (let i = 0; i < source.width * source.height; i++) {
    out.data[i * 4] = source.data[i * 4]
    out.data[i * 4 + 1] = source.data[i * 4 + 1]
    out.data[i * 4 + 2] = source.data[i * 4 + 2]
    const a = (source.data[i * 4 + 3] / 255) * (mask.data[i] * norm)
    out.data[i * 4 + 3] = Math.round(a * 255)
  }
  const c = makeCanvas(source.width, source.height)
  ctxOf(c).putImageData(out, 0, 0)
  return c
}

export interface RenderInput {
  source: ImageData
  mask: Mask
  silhouette: PathsD
  border: PathsD
  cut: PathsD
  borderColor: string
}

export interface RenderResult {
  canvas: AnyCanvas
  fullRes: ImageData
  /** Bounds of the cut polygon — the framing box for the 512 sticker. */
  contentBounds: Bounds
}

/**
 * Compose a sticker: fill the white die-cut border, then draw the subject clipped
 * to its silhouette on top. The cut line itself is not rasterized (it lives in the
 * print PDF/SVG); everything beyond the border stays transparent.
 */
export function renderSticker(input: RenderInput): RenderResult {
  const { width, height } = input.source
  const canvas = makeCanvas(width, height)
  const ctx = ctxOf(canvas)
  ctx.clearRect(0, 0, width, height)

  if (input.border.length > 0 && input.borderColor !== 'none') {
    ctx.fillStyle = input.borderColor
    ctx.fill(pathsToPath2D(input.border))
  }

  const subject = maskedSubjectCanvas(input.source, input.mask)
  ctx.save()
  if (input.silhouette.length > 0) ctx.clip(pathsToPath2D(input.silhouette))
  ctx.drawImage(subject, 0, 0)
  ctx.restore()

  const fullRes = ctx.getImageData(0, 0, width, height)
  const contentBounds = input.cut.length > 0
    ? pathsBounds(input.cut)
    : { x: 0, y: 0, width, height }
  return { canvas, fullRes, contentBounds }
}
