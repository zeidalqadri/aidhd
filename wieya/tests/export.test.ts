import { describe, expect, it } from 'vitest'
import {
  buildContentsJson,
  makePackIdentifier,
  validatePack,
  WHATSAPP_LIMITS,
} from '../src/lib/export/whatsapp'
import { buildCutSvg } from '../src/lib/export/svgCut'
import { effectiveDpi, isDpiAdequate, mmToPt, SHEETS } from '../src/lib/units'

describe('whatsapp pack', () => {
  const meta = { identifier: 'wieya_test', name: 'Test', publisher: 'Me' }

  it('builds spec-shaped contents.json', () => {
    const json = buildContentsJson(meta, 'tray.png', [
      { imageFile: 's0.webp', emojis: ['😀'] },
    ])
    expect(json.sticker_packs).toHaveLength(1)
    const pack = json.sticker_packs[0]
    expect(pack.tray_image_file).toBe('tray.png')
    expect(pack.animated_sticker_pack).toBe(false)
    expect(pack.stickers[0]).toMatchObject({ image_file: 's0.webp', emojis: ['😀'] })
  })

  it('rejects fewer than the minimum stickers', () => {
    const r = validatePack({
      trayBytes: 1000,
      stickers: [{ bytes: 1000, emojis: ['a'] }],
    })
    expect(r.ok).toBe(false)
    expect(r.errors.join(' ')).toMatch(/at least 3/)
  })

  it('flags oversize sticker, oversize tray, and missing emoji', () => {
    const big = WHATSAPP_LIMITS.maxStickerBytes + 1
    const r = validatePack({
      trayBytes: WHATSAPP_LIMITS.maxTrayBytes + 1,
      stickers: [
        { bytes: big, emojis: ['x'] },
        { bytes: 1000, emojis: [] },
        { bytes: 1000, emojis: ['y'] },
      ],
    })
    expect(r.ok).toBe(false)
    expect(r.errors.join(' ')).toMatch(/Sticker 1/)
    expect(r.errors.join(' ')).toMatch(/Tray/)
    expect(r.errors.join(' ')).toMatch(/Sticker 2 needs at least one emoji/)
  })

  it('accepts a valid pack', () => {
    const r = validatePack({
      trayBytes: 2000,
      stickers: Array.from({ length: 3 }, () => ({ bytes: 50000, emojis: ['😀'] })),
    })
    expect(r.ok).toBe(true)
    expect(r.errors).toHaveLength(0)
  })

  it('makes a safe identifier', () => {
    const id = makePackIdentifier('My Cool Pack!!')
    expect(id).toMatch(/^wieya_my_cool_pack_[a-z0-9]+$/)
  })
})

describe('svg cut export', () => {
  it('embeds physical dimensions and a cut path under a CutContour layer', () => {
    const svg = buildCutSvg(
      [
        [
          { x: 0, y: 0 },
          { x: 10, y: 0 },
          { x: 10, y: 10 },
          { x: 0, y: 10 },
        ],
      ],
      { width: 50, height: 30, unit: 'mm' },
    )
    expect(svg).toContain('width="50mm"')
    expect(svg).toContain('height="30mm"')
    expect(svg).toContain('id="CutContour"')
    expect(svg).toContain('<path d="M0,0')
  })

  it('uses a dash array for perforation', () => {
    const svg = buildCutSvg([[{ x: 0, y: 0 }, { x: 1, y: 1 }]], {
      width: 10,
      height: 10,
      unit: 'in',
      perforation: true,
    })
    expect(svg).toContain('stroke-dasharray')
  })
})

describe('units', () => {
  it('computes dpi and adequacy', () => {
    expect(effectiveDpi(512, 2)).toBe(256)
    expect(isDpiAdequate(512, 2)).toBe(false)
    expect(isDpiAdequate(1024, 2)).toBe(true)
  })

  it('A4 is ~595x842 pt', () => {
    expect(SHEETS.A4.widthPt).toBeCloseTo(595.27, 1)
    expect(mmToPt(210)).toBeCloseTo(595.27, 1)
  })
})
