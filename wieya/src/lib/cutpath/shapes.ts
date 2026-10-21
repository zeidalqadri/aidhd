import type { PathD, PathsD } from 'clipper2-ts'
import type { Bounds } from '../geometry'

export type CutShape = 'contour' | 'roundedRect' | 'rect' | 'circle'

const ARC_STEPS = 16

function arc(
  cx: number,
  cy: number,
  r: number,
  startAngle: number,
  endAngle: number,
  steps = ARC_STEPS,
): PathD {
  const pts: PathD = []
  for (let i = 0; i <= steps; i++) {
    const t = startAngle + ((endAngle - startAngle) * i) / steps
    pts.push({ x: cx + r * Math.cos(t), y: cy + r * Math.sin(t) })
  }
  return pts
}

export function rectPath(b: Bounds): PathsD {
  return [
    [
      { x: b.x, y: b.y },
      { x: b.x + b.width, y: b.y },
      { x: b.x + b.width, y: b.y + b.height },
      { x: b.x, y: b.y + b.height },
    ],
  ]
}

export function circlePath(b: Bounds, steps = 64): PathsD {
  const cx = b.x + b.width / 2
  const cy = b.y + b.height / 2
  const r = Math.min(b.width, b.height) / 2
  const ring: PathD = []
  for (let i = 0; i < steps; i++) {
    const t = (Math.PI * 2 * i) / steps
    ring.push({ x: cx + r * Math.cos(t), y: cy + r * Math.sin(t) })
  }
  return [ring]
}

export function roundedRectPath(b: Bounds, radius?: number): PathsD {
  const r = Math.max(
    0,
    Math.min(radius ?? Math.min(b.width, b.height) * 0.18, b.width / 2, b.height / 2),
  )
  if (r === 0) return rectPath(b)
  const x0 = b.x
  const y0 = b.y
  const x1 = b.x + b.width
  const y1 = b.y + b.height
  const HALF = Math.PI / 2
  const ring: PathD = [
    ...arc(x1 - r, y0 + r, r, -HALF, 0), // top-right
    ...arc(x1 - r, y1 - r, r, 0, HALF), // bottom-right
    ...arc(x0 + r, y1 - r, r, HALF, Math.PI), // bottom-left
    ...arc(x0 + r, y0 + r, r, Math.PI, Math.PI + HALF), // top-left
  ]
  return [ring]
}

export function shapePath(shape: Exclude<CutShape, 'contour'>, b: Bounds): PathsD {
  switch (shape) {
    case 'rect':
      return rectPath(b)
    case 'circle':
      return circlePath(b)
    case 'roundedRect':
      return roundedRectPath(b)
  }
}
