import { isoContours } from 'marchingsquares'
import type { PathsD } from 'clipper2-ts'

export interface Mask {
  /** Normalized alpha in [0,1], row-major, length = width * height. */
  data: Float32Array | Uint8ClampedArray
  width: number
  height: number
  /** If data is 0..255 (Uint8) rather than 0..1. */
  scale255?: boolean
}

/**
 * Trace the foreground silhouette of an alpha mask into polygon rings (pixel
 * coords). The mask is padded with a 1px background border so the subject never
 * touches the domain edge; isoContours always emits one spurious contour that
 * traces the whole (background) domain, which we drop by removing any contour
 * that touches the padded boundary.
 */
export function maskToContours(mask: Mask, threshold = 0.5): PathsD {
  const { width: w, height: h } = mask
  const norm = mask.scale255 ? 1 / 255 : 1
  const pad = 1
  const pw = w + pad * 2
  const ph = h + pad * 2

  const grid: number[][] = new Array(ph)
  for (let y = 0; y < ph; y++) {
    const row = new Array<number>(pw).fill(0)
    grid[y] = row
  }
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      grid[y + pad][x + pad] = mask.data[y * w + x] * norm
    }
  }

  const contours = isoContours(grid, threshold)
  const touchesEdge = (ring: number[][]) =>
    ring.some(([x, y]) => x <= 0 || y <= 0 || x >= pw - 1 || y >= ph - 1)

  const out: PathsD = []
  for (const ring of contours) {
    if (ring.length < 4) continue
    if (touchesEdge(ring)) continue // drops the domain-frame contour
    out.push(ring.map(([x, y]) => ({ x: x - pad, y: y - pad })))
  }
  return out
}
