import { useEffect, useRef } from 'react'
import type { StickerItem } from '../lib/types'
import { CUT_COLOR } from '../lib/export/svgCut'

interface Props {
  item: StickerItem
  showCut?: boolean
  perforation?: boolean
  size?: number
}

export function PreviewCanvas({ item, showCut = false, perforation = true, size = 240 }: Props) {
  const ref = useRef<HTMLCanvasElement>(null)

  useEffect(() => {
    const canvas = ref.current
    if (!canvas) return
    const ctx = canvas.getContext('2d')
    if (!ctx) return
    canvas.width = size
    canvas.height = size
    ctx.clearRect(0, 0, size, size)

    const p = item.processed
    if (!p) {
      // Original preview.
      const scale = Math.min(size / item.source.width, size / item.source.height)
      const w = item.source.width * scale
      const h = item.source.height * scale
      ctx.drawImage(item.source, (size - w) / 2, (size - h) / 2, w, h)
      return
    }

    const { fullRes } = p
    const tmp = document.createElement('canvas')
    tmp.width = fullRes.width
    tmp.height = fullRes.height
    tmp.getContext('2d')!.putImageData(fullRes, 0, 0)

    const scale = Math.min(size / fullRes.width, size / fullRes.height)
    const w = fullRes.width * scale
    const h = fullRes.height * scale
    const ox = (size - w) / 2
    const oy = (size - h) / 2
    ctx.drawImage(tmp, ox, oy, w, h)

    if (showCut && p.cut.length) {
      ctx.save()
      ctx.translate(ox, oy)
      ctx.scale(scale, scale)
      ctx.lineWidth = 2 / scale
      ctx.strokeStyle = CUT_COLOR
      if (perforation) ctx.setLineDash([8 / scale, 6 / scale])
      for (const ring of p.cut) {
        ctx.beginPath()
        ring.forEach((pt, i) => (i === 0 ? ctx.moveTo(pt.x, pt.y) : ctx.lineTo(pt.x, pt.y)))
        ctx.closePath()
        ctx.stroke()
      }
      ctx.restore()
    }
  }, [item, item.processed, showCut, perforation, size])

  return <canvas ref={ref} className="bg-checker rounded-lg" style={{ width: size, height: size }} />
}
