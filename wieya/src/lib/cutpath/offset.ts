import {
  EndType,
  FillRule,
  JoinType,
  type PathsD,
  areaD,
  inflatePathsD,
  reversePathD,
  simplifyPathsD,
  unionD,
} from 'clipper2-ts'

/**
 * Clean raw rings into outer cut boundaries: union overlapping pieces, simplify,
 * drop holes (negative-area rings) and tiny specks, and normalize every kept ring
 * to positive orientation so a positive offset grows outward.
 */
export function outerBoundaries(
  raw: PathsD,
  opts: { simplifyEpsilon?: number; minArea?: number } = {},
): PathsD {
  if (raw.length === 0) return []
  const simplifyEpsilon = opts.simplifyEpsilon ?? 1.0
  const minArea = opts.minArea ?? 16

  const merged = unionD(raw, FillRule.NonZero)
  const simplified = simplifyPathsD(merged, simplifyEpsilon, true)

  const kept: PathsD = []
  for (const ring of simplified) {
    const a = areaD(ring)
    if (Math.abs(a) < minArea) continue // speck
    if (a < 0) continue // hole — not cut by default
    kept.push(ring)
  }
  return kept
}

/** Offset (dilate) closed polygons outward by `delta` pixels with round joins. */
export function offsetOutward(paths: PathsD, delta: number): PathsD {
  if (paths.length === 0 || delta <= 0) return paths
  return inflatePathsD(paths, delta, JoinType.Round, EndType.Polygon)
}

/** Force every ring to positive (outward) winding. */
export function normalizeOrientation(paths: PathsD): PathsD {
  return paths.map((ring) => (areaD(ring) < 0 ? reversePathD(ring) : ring))
}
