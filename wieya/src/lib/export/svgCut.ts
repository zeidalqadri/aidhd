import type { PathsD } from 'clipper2-ts'
import { pathsToSvgData } from '../geometry'

export const CUT_COLOR = '#ED008C' // magenta "CutContour"-style guide

export interface SvgCutOptions {
  /** Physical sheet/sticker size the paths are expressed in. */
  width: number
  height: number
  unit: 'mm' | 'in'
  /** Stroke width in the same unit (default 0.25mm-ish). */
  strokeWidth?: number
  /** Dashed perforation vs solid kiss-cut. */
  perforation?: boolean
  title?: string
}

/**
 * Emit an SVG whose only content is the cut path(s), sized in real-world units so
 * cutting software (Silhouette Studio, pro die-cutters) imports them at scale.
 */
export function buildCutSvg(paths: PathsD, opts: SvgCutOptions): string {
  const stroke = opts.strokeWidth ?? (opts.unit === 'mm' ? 0.25 : 0.01)
  const dash = opts.perforation
    ? ` stroke-dasharray="${stroke * 8},${stroke * 5}"`
    : ''
  const d = pathsToSvgData(paths, 3)
  const u = opts.unit
  return `<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="${opts.width}${u}" height="${opts.height}${u}" viewBox="0 0 ${opts.width} ${opts.height}">
  <title>${escapeXml(opts.title ?? 'Wieya cut path')}</title>
  <g id="CutContour" fill="none" stroke="${CUT_COLOR}" stroke-width="${stroke}"${dash}>
    <path d="${d}" />
  </g>
</svg>
`
}

function escapeXml(s: string): string {
  return s.replace(/[<>&'"]/g, (c) =>
    c === '<' ? '&lt;' : c === '>' ? '&gt;' : c === '&' ? '&amp;' : c === "'" ? '&apos;' : '&quot;',
  )
}
