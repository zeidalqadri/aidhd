import { describe, expect, it } from 'vitest'
import { areaPathsD } from 'clipper2-ts'
import { maskToContours, type Mask } from '../src/lib/cutpath/contours'
import { outerBoundaries, offsetOutward } from '../src/lib/cutpath/offset'
import { buildCutGeometry } from '../src/lib/cutpath'
import { circlePath, rectPath, roundedRectPath } from '../src/lib/cutpath/shapes'
import { pathsBounds, signedArea } from '../src/lib/geometry'

/** Build a mask with a filled rectangle of 1s inside a field of 0s. */
function blockMask(w: number, h: number, bx: number, by: number, bw: number, bh: number): Mask {
  const data = new Float32Array(w * h)
  for (let y = by; y < by + bh; y++)
    for (let x = bx; x < bx + bw; x++) data[y * w + x] = 1
  return { data, width: w, height: h }
}

describe('shapes', () => {
  it('rect path matches bounds', () => {
    const b = { x: 10, y: 20, width: 30, height: 40 }
    const bounds = pathsBounds(rectPath(b))
    expect(bounds).toEqual(b)
  })

  it('circle fits within bounds and is round', () => {
    const b = { x: 0, y: 0, width: 100, height: 100 }
    const bounds = pathsBounds(circlePath(b))
    expect(bounds.width).toBeCloseTo(100, 0)
    expect(Math.abs(areaPathsD(circlePath(b)))).toBeCloseTo(Math.PI * 50 * 50, -2)
  })

  it('rounded rect stays within bounds', () => {
    const b = { x: 0, y: 0, width: 80, height: 60 }
    const bounds = pathsBounds(roundedRectPath(b, 12))
    expect(bounds.width).toBeLessThanOrEqual(80.001)
    expect(bounds.height).toBeLessThanOrEqual(60.001)
  })
})

describe('maskToContours', () => {
  it('drops the domain frame and traces the subject only', () => {
    const mask = blockMask(40, 40, 10, 10, 20, 20)
    const rings = maskToContours(mask)
    expect(rings.length).toBe(1)
    const b = pathsBounds(rings)
    // Marching squares traces the 0.5 isovalue ~half a pixel outside the block.
    expect(b.x).toBeCloseTo(9.5, 1)
    expect(b.width).toBeCloseTo(20, 1)
  })

  it('returns nothing for an empty mask', () => {
    expect(maskToContours(blockMask(20, 20, 0, 0, 0, 0)).length).toBe(0)
  })
})

describe('offset', () => {
  it('drops holes and specks, keeps the outer ring', () => {
    const mask = blockMask(60, 60, 15, 15, 30, 30)
    const raw = maskToContours(mask)
    const outer = outerBoundaries(raw)
    expect(outer.length).toBe(1)
    expect(signedArea(outer[0])).toBeGreaterThan(0)
  })

  it('offset grows area monotonically', () => {
    const sq = [
      [
        { x: 0, y: 0 },
        { x: 100, y: 0 },
        { x: 100, y: 100 },
        { x: 0, y: 100 },
      ],
    ]
    const a0 = Math.abs(areaPathsD(sq))
    const a1 = Math.abs(areaPathsD(offsetOutward(sq, 5)))
    const a2 = Math.abs(areaPathsD(offsetOutward(sq, 10)))
    expect(a1).toBeGreaterThan(a0)
    expect(a2).toBeGreaterThan(a1)
  })
})

describe('buildCutGeometry', () => {
  it('contour mode: cut area >= border area >= silhouette area', () => {
    const mask = blockMask(80, 80, 20, 20, 40, 40)
    const g = buildCutGeometry({
      shape: 'contour',
      bounds: { x: 20, y: 20, width: 40, height: 40 },
      mask,
      borderPx: 6,
      cutOffsetPx: 10,
    })
    const sA = Math.abs(areaPathsD(g.silhouette))
    const bA = Math.abs(areaPathsD(g.border))
    const cA = Math.abs(areaPathsD(g.cut))
    expect(sA).toBeGreaterThan(0)
    expect(bA).toBeGreaterThanOrEqual(sA)
    expect(cA).toBeGreaterThanOrEqual(bA)
  })

  it('shape mode: rounded rect produces a valid cut', () => {
    const g = buildCutGeometry({
      shape: 'roundedRect',
      bounds: { x: 0, y: 0, width: 200, height: 120 },
      borderPx: 0,
      cutOffsetPx: 8,
    })
    expect(g.silhouette.length).toBe(1)
    expect(Math.abs(areaPathsD(g.cut))).toBeGreaterThan(Math.abs(areaPathsD(g.silhouette)))
  })

  it('contour mode without a mask throws', () => {
    expect(() =>
      buildCutGeometry({
        shape: 'contour',
        bounds: { x: 0, y: 0, width: 10, height: 10 },
        borderPx: 2,
        cutOffsetPx: 4,
      }),
    ).toThrow()
  })
})
