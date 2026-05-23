import { buildCutGeometry, type Mask } from './cutpath'
import type { Bounds } from './geometry'
import { prepareWorkingImage, frameToSquare } from './raster/resize'
import { renderSticker } from './raster/render'
import { encodeWebpUnderBudget } from './raster/webp'
import { opaqueMask, removeBackgroundMask, type SegmentProgress } from './segmentation/engine'
import type { ProcessedSticker, Settings } from './types'

/** Bounding box of pixels whose mask alpha exceeds the threshold. */
function maskBounds(mask: Mask, threshold = 0.5): Bounds {
  const norm = mask.scale255 ? 1 / 255 : 1
  let minX = mask.width
  let minY = mask.height
  let maxX = 0
  let maxY = 0
  let found = false
  for (let y = 0; y < mask.height; y++) {
    for (let x = 0; x < mask.width; x++) {
      if (mask.data[y * mask.width + x] * norm >= threshold) {
        found = true
        if (x < minX) minX = x
        if (y < minY) minY = y
        if (x > maxX) maxX = x
        if (y > maxY) maxY = y
      }
    }
  }
  if (!found) return { x: 0, y: 0, width: mask.width, height: mask.height }
  return { x: minX, y: minY, width: maxX - minX + 1, height: maxY - minY + 1 }
}

export async function processSticker(
  source: ImageBitmap,
  settings: Settings,
  onProgress?: SegmentProgress,
): Promise<ProcessedSticker> {
  const img = prepareWorkingImage(source)

  const mask = settings.removeBackground
    ? await removeBackgroundMask(img, onProgress)
    : opaqueMask(img.width, img.height)

  const bounds = maskBounds(mask)

  const geometry = buildCutGeometry({
    shape: settings.cutShape,
    bounds,
    mask: settings.cutShape === 'contour' ? mask : undefined,
    borderPx: settings.borderPx,
    cutOffsetPx: settings.cutOffsetPx,
  })

  const rendered = renderSticker({
    source: img,
    mask,
    silhouette: geometry.silhouette,
    border: geometry.border,
    cut: geometry.cut,
    borderColor: settings.borderColor,
  })

  const square = frameToSquare(rendered.canvas, rendered.contentBounds, 512, 8)
  const encoded = await encodeWebpUnderBudget(square)

  return {
    fullRes: rendered.fullRes,
    cut: geometry.cut,
    border: geometry.border,
    silhouette: geometry.silhouette,
    webp512: encoded.blob,
    webp512Bytes: encoded.bytes,
  }
}
