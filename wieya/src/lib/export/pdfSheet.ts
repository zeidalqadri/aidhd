import { PDFDocument, rgb, type PDFPage } from 'pdf-lib'
import type { PathsD } from 'clipper2-ts'
import { pathsBounds } from '../geometry'
import { inchToPt, SHEETS } from '../units'
import { makeCanvas } from '../decode'
import { canvasToBlob } from '../raster/resize'

const CUT = rgb(0.929, 0, 0.549) // magenta CutContour guide
const MARK = rgb(0, 0, 0)

export interface PrintStickerInput {
  fullRes: ImageData
  cut: PathsD
}

export interface PrintOptions {
  sheet: 'A4' | 'Letter'
  stickerWidthIn: number
  marginPt?: number
  gapPt?: number
  bleedPt?: number
  perforation: boolean
}

async function imageDataToPng(img: ImageData): Promise<Uint8Array> {
  const c = makeCanvas(img.width, img.height)
  const ctx = c.getContext('2d') as
    | OffscreenCanvasRenderingContext2D
    | CanvasRenderingContext2D
    | null
  if (!ctx) throw new Error('2D canvas context unavailable')
  ctx.putImageData(img, 0, 0)
  const blob = await canvasToBlob(c, 'image/png')
  return new Uint8Array(await blob.arrayBuffer())
}

/** Generate a print-ready sheet PDF: artwork at physical size with dashed cut
 * lines, crop marks, and registration crosshairs. */
export async function generatePrintSheet(
  stickers: PrintStickerInput[],
  opts: PrintOptions,
): Promise<Uint8Array> {
  const sheet = SHEETS[opts.sheet]
  const margin = opts.marginPt ?? 28
  const gap = opts.gapPt ?? 14
  const dash = opts.perforation ? [4, 3] : undefined

  const doc = await PDFDocument.create()
  let page = doc.addPage([sheet.widthPt, sheet.heightPt])
  drawRegistrationMarks(page, sheet.widthPt, sheet.heightPt, margin)

  let cursorX = margin
  let cursorY = margin // top-down
  let rowHeight = 0

  for (const sticker of stickers) {
    const cutB = pathsBounds(sticker.cut)
    if (cutB.width === 0 || cutB.height === 0) continue
    const wPt = inchToPt(opts.stickerWidthIn)
    const scale = wPt / cutB.width
    const hPt = cutB.height * scale

    // Wrap to next row / page.
    if (cursorX + wPt > sheet.widthPt - margin) {
      cursorX = margin
      cursorY += rowHeight + gap
      rowHeight = 0
    }
    if (cursorY + hPt > sheet.heightPt - margin) {
      page = doc.addPage([sheet.widthPt, sheet.heightPt])
      drawRegistrationMarks(page, sheet.widthPt, sheet.heightPt, margin)
      cursorX = margin
      cursorY = margin
      rowHeight = 0
    }

    const cellX = cursorX
    const cellTop = cursorY
    const toY = (yTop: number) => sheet.heightPt - yTop

    // Embed the full-res artwork, aligning the cut bounds to the cell.
    const png = await doc.embedPng(await imageDataToPng(sticker.fullRes))
    const imgW = sticker.fullRes.width * scale
    const imgH = sticker.fullRes.height * scale
    const imgLeft = cellX - cutB.x * scale
    const imgTop = cellTop - cutB.y * scale
    page.drawImage(png, {
      x: imgLeft,
      y: toY(imgTop + imgH),
      width: imgW,
      height: imgH,
    })

    // Cut / perforation path.
    for (const ring of sticker.cut) {
      for (let i = 0; i < ring.length; i++) {
        const a = ring[i]
        const b = ring[(i + 1) % ring.length]
        page.drawLine({
          start: { x: cellX + (a.x - cutB.x) * scale, y: toY(cellTop + (a.y - cutB.y) * scale) },
          end: { x: cellX + (b.x - cutB.x) * scale, y: toY(cellTop + (b.y - cutB.y) * scale) },
          thickness: 0.75,
          color: CUT,
          dashArray: dash,
        })
      }
    }

    drawCropMarks(page, cellX, cellTop, wPt, hPt, toY)

    cursorX += wPt + gap
    rowHeight = Math.max(rowHeight, hPt)
  }

  return doc.save()
}

function drawCropMarks(
  page: PDFPage,
  x: number,
  top: number,
  w: number,
  h: number,
  toY: (y: number) => number,
) {
  const len = 8
  const off = 3
  const corners: [number, number][] = [
    [x, top],
    [x + w, top],
    [x, top + h],
    [x + w, top + h],
  ]
  for (const [cx, cy] of corners) {
    const sx = cx < x + w / 2 ? -1 : 1
    const sy = cy < top + h / 2 ? -1 : 1
    page.drawLine({
      start: { x: cx + sx * off, y: toY(cy) },
      end: { x: cx + sx * (off + len), y: toY(cy) },
      thickness: 0.5,
      color: MARK,
    })
    page.drawLine({
      start: { x: cx, y: toY(cy + sy * off) },
      end: { x: cx, y: toY(cy + sy * (off + len)) },
      thickness: 0.5,
      color: MARK,
    })
  }
}

function drawRegistrationMarks(page: PDFPage, w: number, h: number, margin: number) {
  const r = 6
  const spots: [number, number][] = [
    [margin / 2, h - margin / 2],
    [w - margin / 2, margin / 2],
  ]
  for (const [cx, cy] of spots) {
    page.drawCircle({ x: cx, y: cy, size: r, borderColor: MARK, borderWidth: 0.5 })
    page.drawLine({ start: { x: cx - r - 2, y: cy }, end: { x: cx + r + 2, y: cy }, thickness: 0.5, color: MARK })
    page.drawLine({ start: { x: cx, y: cy - r - 2 }, end: { x: cx, y: cy + r + 2 }, thickness: 0.5, color: MARK })
  }
}
