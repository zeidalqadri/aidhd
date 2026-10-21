import type { PathD, PathsD } from 'clipper2-ts'

export interface Bounds {
  x: number
  y: number
  width: number
  height: number
}

/** Signed area via the shoelace formula. Sign encodes winding direction. */
export function signedArea(ring: PathD): number {
  let sum = 0
  for (let i = 0; i < ring.length; i++) {
    const a = ring[i]
    const b = ring[(i + 1) % ring.length]
    sum += a.x * b.y - b.x * a.y
  }
  return sum / 2
}

export function pathsBounds(paths: PathsD): Bounds {
  let minX = Infinity
  let minY = Infinity
  let maxX = -Infinity
  let maxY = -Infinity
  for (const ring of paths) {
    for (const p of ring) {
      if (p.x < minX) minX = p.x
      if (p.y < minY) minY = p.y
      if (p.x > maxX) maxX = p.x
      if (p.y > maxY) maxY = p.y
    }
  }
  if (!Number.isFinite(minX)) return { x: 0, y: 0, width: 0, height: 0 }
  return { x: minX, y: minY, width: maxX - minX, height: maxY - minY }
}

/** Convert clipper paths to SVG path data (absolute), one subpath per ring. */
export function pathsToSvgData(paths: PathsD, precision = 2): string {
  const f = (n: number) => Number(n.toFixed(precision)).toString()
  return paths
    .map((ring) => {
      if (ring.length === 0) return ''
      const [first, ...rest] = ring
      const head = `M${f(first.x)},${f(first.y)}`
      const body = rest.map((p) => `L${f(p.x)},${f(p.y)}`).join('')
      return `${head}${body}Z`
    })
    .filter(Boolean)
    .join(' ')
}

/** Translate + uniformly scale every point (maps pixel space to physical units). */
export function transformPaths(
  paths: PathsD,
  scale: number,
  offsetX = 0,
  offsetY = 0,
): PathsD {
  return paths.map((ring) =>
    ring.map((p) => ({ x: p.x * scale + offsetX, y: p.y * scale + offsetY })),
  )
}
