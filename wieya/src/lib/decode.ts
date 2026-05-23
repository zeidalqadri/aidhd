export const ACCEPTED_TYPES = ['image/jpeg', 'image/png', 'image/webp'] as const
export const MAX_UPLOAD_BYTES = 25 * 1024 * 1024

export function isAcceptedType(type: string): boolean {
  return (ACCEPTED_TYPES as readonly string[]).includes(type)
}

export interface DecodedImage {
  bitmap: ImageBitmap
  url: string
}

/** Decode an uploaded file to an EXIF-corrected ImageBitmap + preview URL. */
export async function decodeFile(file: File): Promise<DecodedImage> {
  if (!isAcceptedType(file.type)) {
    throw new Error(
      `Unsupported file type "${file.type || 'unknown'}". Use JPEG, PNG, or WebP.`,
    )
  }
  if (file.size > MAX_UPLOAD_BYTES) {
    throw new Error(`File is too large (max ${Math.round(MAX_UPLOAD_BYTES / 1024 / 1024)}MB).`)
  }
  const bitmap = await createImageBitmap(file, { imageOrientation: 'from-image' })
  const url = URL.createObjectURL(file)
  return { bitmap, url }
}

/** Draw a bitmap to an OffscreenCanvas (or canvas) and read back its pixels. */
export function bitmapToImageData(bitmap: ImageBitmap): ImageData {
  const canvas = makeCanvas(bitmap.width, bitmap.height)
  const ctx = canvas.getContext('2d', { willReadFrequently: true }) as
    | OffscreenCanvasRenderingContext2D
    | CanvasRenderingContext2D
    | null
  if (!ctx) throw new Error('2D canvas context unavailable')
  ctx.drawImage(bitmap, 0, 0)
  return ctx.getImageData(0, 0, bitmap.width, bitmap.height)
}

export function makeCanvas(width: number, height: number): OffscreenCanvas | HTMLCanvasElement {
  if (typeof OffscreenCanvas !== 'undefined') return new OffscreenCanvas(width, height)
  const c = document.createElement('canvas')
  c.width = width
  c.height = height
  return c
}
