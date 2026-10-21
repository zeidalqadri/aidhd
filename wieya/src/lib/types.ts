import type { PathsD } from 'clipper2-ts'
import type { CutShape } from './cutpath'

export type { CutShape }

export type WizardStep = 'upload' | 'process' | 'layout' | 'export'

export interface Settings {
  removeBackground: boolean
  cutShape: CutShape
  borderPx: number
  borderColor: string
  cutOffsetPx: number
  perforation: boolean // dashed perforation vs solid kiss-cut
}

export const DEFAULT_SETTINGS: Settings = {
  removeBackground: true,
  cutShape: 'contour',
  borderPx: 14,
  borderColor: '#ffffff',
  cutOffsetPx: 18,
  perforation: true,
}

/** Result of processing one source image into a sticker. */
export interface ProcessedSticker {
  /** Full-resolution RGBA with background removed (if enabled) + white border. */
  fullRes: ImageData
  /** Cut/border geometry in fullRes pixel coordinates. */
  cut: PathsD
  border: PathsD
  silhouette: PathsD
  /** 512x512 transparent WebP for WhatsApp. */
  webp512?: Blob
  webp512Bytes?: number
}

export interface StickerItem {
  id: string
  fileName: string
  /** Decoded original image. */
  source: ImageBitmap
  /** Object URL for previewing the original. */
  sourceUrl: string
  emojis: string[]
  processed?: ProcessedSticker
  /** Set when processing fails so the UI can surface it. */
  error?: string
}
