import type { PathsD } from 'clipper2-ts'
import type { Bounds } from '../geometry'
import { maskToContours, type Mask } from './contours'
import { normalizeOrientation, offsetOutward, outerBoundaries } from './offset'
import { type CutShape, shapePath } from './shapes'

export type { CutShape } from './shapes'
export type { Mask } from './contours'

export interface CutGeometry {
  /** Outer silhouette in pixel coords (subject edge or shape edge). */
  silhouette: PathsD
  /** Silhouette offset outward by the white border width. */
  border: PathsD
  /** Silhouette offset outward by the cut distance — the kiss-cut / perforation path. */
  cut: PathsD
}

export interface CutGeometryOptions {
  shape: CutShape
  /** Pixel bounds of the trimmed artwork (used for geometric shapes). */
  bounds: Bounds
  /** Alpha mask (required when shape === 'contour'). */
  mask?: Mask
  borderPx: number
  cutOffsetPx: number
  cornerRadiusPx?: number
  simplifyEpsilon?: number
}

/**
 * Produce the silhouette, white-border, and cut polygons for one sticker. For
 * 'contour' shapes the silhouette is traced from the alpha mask; otherwise it is
 * the chosen geometric primitive around the artwork bounds.
 */
export function buildCutGeometry(opts: CutGeometryOptions): CutGeometry {
  const cutOffset = Math.max(opts.cutOffsetPx, opts.borderPx)

  let silhouette: PathsD
  if (opts.shape === 'contour') {
    if (!opts.mask) throw new Error('contour shape requires an alpha mask')
    const raw = maskToContours(opts.mask)
    silhouette = normalizeOrientation(
      outerBoundaries(raw, { simplifyEpsilon: opts.simplifyEpsilon ?? 1.0 }),
    )
  } else {
    silhouette = normalizeOrientation(
      shapePath(opts.shape, opts.bounds),
    )
  }

  const border = offsetOutward(silhouette, opts.borderPx)
  const cut = offsetOutward(silhouette, cutOffset)
  return { silhouette, border, cut }
}
