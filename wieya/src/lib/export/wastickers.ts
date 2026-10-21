import type { PackMeta } from './whatsapp'
import type { PackSticker } from './pack'

async function blobToBase64(blob: Blob): Promise<string> {
  const buf = new Uint8Array(await blob.arrayBuffer())
  let binary = ''
  for (let i = 0; i < buf.length; i++) binary += String.fromCharCode(buf[i])
  return btoa(binary)
}

/**
 * Build a best-effort `.wastickers` bundle (base64-JSON) for third-party importer
 * apps. NOTE: there is no single official schema for this format; the guaranteed
 * path is the spec-compliant zip from buildWhatsAppZip. Verify against a target
 * importer before relying on this.
 */
export async function buildWastickersFile(
  meta: PackMeta,
  stickers: PackSticker[],
  tray: Blob,
): Promise<Blob> {
  const payload = {
    identifier: meta.identifier,
    name: meta.name,
    publisher: meta.publisher,
    tray_image: await blobToBase64(tray),
    stickers: await Promise.all(
      stickers.map(async (s) => ({
        image_data: await blobToBase64(s.webp),
        emojis: s.emojis,
      })),
    ),
  }
  return new Blob([JSON.stringify(payload)], { type: 'application/octet-stream' })
}
