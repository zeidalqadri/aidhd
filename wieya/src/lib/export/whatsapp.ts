export const WHATSAPP_LIMITS = {
  stickerPx: 512,
  trayPx: 96,
  maxStickerBytes: 100 * 1024,
  maxTrayBytes: 50 * 1024,
  minStickers: 3,
  maxStickers: 30,
  maxEmojisPerSticker: 3,
} as const

export interface StickerMeta {
  imageFile: string
  emojis: string[]
  accessibilityText?: string
}

export interface PackMeta {
  identifier: string
  name: string
  publisher: string
  publisherEmail?: string
  publisherWebsite?: string
  privacyPolicyWebsite?: string
  licenseAgreementWebsite?: string
}

/** Build the WAStickerApps `contents.json` payload (file-reference form). */
export function buildContentsJson(
  meta: PackMeta,
  trayImageFile: string,
  stickers: StickerMeta[],
) {
  return {
    android_play_store_link: '',
    ios_app_store_link: '',
    sticker_packs: [
      {
        identifier: meta.identifier,
        name: meta.name,
        publisher: meta.publisher,
        tray_image_file: trayImageFile,
        image_data_version: '1',
        publisher_email: meta.publisherEmail ?? '',
        publisher_website: meta.publisherWebsite ?? '',
        privacy_policy_website: meta.privacyPolicyWebsite ?? '',
        license_agreement_website: meta.licenseAgreementWebsite ?? '',
        animated_sticker_pack: false,
        stickers: stickers.map((s) => ({
          image_file: s.imageFile,
          emojis: s.emojis,
          accessibility_text: s.accessibilityText ?? '',
        })),
      },
    ],
  }
}

export interface PackValidationInput {
  stickers: { bytes: number; emojis: string[] }[]
  trayBytes: number
}

/** Validate a pack against WhatsApp's published constraints. */
export function validatePack(input: PackValidationInput): {
  ok: boolean
  errors: string[]
} {
  const errors: string[] = []
  const n = input.stickers.length
  if (n < WHATSAPP_LIMITS.minStickers)
    errors.push(`A pack needs at least ${WHATSAPP_LIMITS.minStickers} stickers (have ${n}).`)
  if (n > WHATSAPP_LIMITS.maxStickers)
    errors.push(`A pack allows at most ${WHATSAPP_LIMITS.maxStickers} stickers (have ${n}).`)
  if (input.trayBytes > WHATSAPP_LIMITS.maxTrayBytes)
    errors.push(`Tray icon is ${fmtKB(input.trayBytes)} (max ${fmtKB(WHATSAPP_LIMITS.maxTrayBytes)}).`)
  input.stickers.forEach((s, i) => {
    if (s.bytes > WHATSAPP_LIMITS.maxStickerBytes)
      errors.push(`Sticker ${i + 1} is ${fmtKB(s.bytes)} (max ${fmtKB(WHATSAPP_LIMITS.maxStickerBytes)}).`)
    if (s.emojis.length === 0) errors.push(`Sticker ${i + 1} needs at least one emoji.`)
    if (s.emojis.length > WHATSAPP_LIMITS.maxEmojisPerSticker)
      errors.push(`Sticker ${i + 1} has too many emojis (max ${WHATSAPP_LIMITS.maxEmojisPerSticker}).`)
  })
  return { ok: errors.length === 0, errors }
}

export function makePackIdentifier(name: string): string {
  const slug = name
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '_')
    .replace(/^_+|_+$/g, '')
    .slice(0, 96)
  const rand = Math.random().toString(36).slice(2, 8)
  return `wieya_${slug || 'pack'}_${rand}`
}

const fmtKB = (bytes: number) => `${Math.round(bytes / 1024)}KB`
