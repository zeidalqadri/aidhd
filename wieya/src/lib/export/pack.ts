import JSZip from 'jszip'
import { makeCanvas } from '../decode'
import { canvasToBlob } from '../raster/resize'
import {
  buildContentsJson,
  type PackMeta,
  type StickerMeta,
  WHATSAPP_LIMITS,
} from './whatsapp'

export interface PackSticker {
  webp: Blob
  emojis: string[]
  accessibilityText?: string
}

/** Render a square sticker blob down to a 96x96 PNG tray icon under budget. */
export async function makeTrayIcon(stickerWebp: Blob): Promise<Blob> {
  const bitmap = await createImageBitmap(stickerWebp)
  const c = makeCanvas(WHATSAPP_LIMITS.trayPx, WHATSAPP_LIMITS.trayPx)
  const ctx = c.getContext('2d') as
    | OffscreenCanvasRenderingContext2D
    | CanvasRenderingContext2D
    | null
  if (!ctx) throw new Error('2D canvas context unavailable')
  ctx.imageSmoothingEnabled = true
  ctx.imageSmoothingQuality = 'high'
  ctx.drawImage(bitmap, 0, 0, WHATSAPP_LIMITS.trayPx, WHATSAPP_LIMITS.trayPx)
  bitmap.close?.()
  return canvasToBlob(c, 'image/png')
}

/** Bundle a spec-compliant WAStickerApps pack (contents.json + tray + webp). */
export async function buildWhatsAppZip(
  meta: PackMeta,
  stickers: PackSticker[],
  tray: Blob,
): Promise<Blob> {
  const zip = new JSZip()
  const trayName = 'tray.png'
  const stickerMetas: StickerMeta[] = []

  stickers.forEach((s, i) => {
    const file = `sticker_${i}.webp`
    zip.file(file, s.webp)
    stickerMetas.push({
      imageFile: file,
      emojis: s.emojis,
      accessibilityText: s.accessibilityText,
    })
  })
  zip.file(trayName, tray)
  zip.file('contents.json', JSON.stringify(buildContentsJson(meta, trayName, stickerMetas), null, 2))
  zip.file('README.txt', IMPORT_README)

  return zip.generateAsync({ type: 'blob' })
}

const IMPORT_README = `Wieya sticker pack
==================

This zip is a WhatsApp sticker pack in the official WAStickerApps layout
(contents.json + tray.png + sticker_*.webp).

A web page cannot add stickers to WhatsApp directly. To install:
1. Open a sticker-importer app that reads WAStickerApps packs (or use the
   accompanying .wastickers file with a compatible importer).
2. Point it at this pack's contents.json / files.
3. Tap "Add to WhatsApp".

Each sticker is 512x512 WebP (<=100KB); the tray icon is 96x96 PNG.
`
