export interface SampleDef {
  name: string
  url: string
}

export const SAMPLES: SampleDef[] = [
  { name: 'star-buddy.png', url: `${import.meta.env.BASE_URL}samples/star-buddy.svg` },
  { name: 'coffee-cup.png', url: `${import.meta.env.BASE_URL}samples/coffee-cup.svg` },
  { name: 'rocket.png', url: `${import.meta.env.BASE_URL}samples/rocket.svg` },
]

/** Rasterize a bundled SVG sample into a PNG File so it flows through the same
 * upload pipeline as user images. */
export async function loadSampleFile(sample: SampleDef): Promise<File> {
  const img = new Image()
  img.crossOrigin = 'anonymous'
  await new Promise<void>((resolve, reject) => {
    img.onload = () => resolve()
    img.onerror = () => reject(new Error(`Failed to load sample ${sample.name}`))
    img.src = sample.url
  })
  const w = img.naturalWidth || 768
  const h = img.naturalHeight || 768
  const canvas = document.createElement('canvas')
  canvas.width = w
  canvas.height = h
  const ctx = canvas.getContext('2d')
  if (!ctx) throw new Error('2D canvas context unavailable')
  ctx.drawImage(img, 0, 0, w, h)
  const blob = await new Promise<Blob | null>((resolve) => canvas.toBlob(resolve, 'image/png'))
  if (!blob) throw new Error('Failed to rasterize sample')
  return new File([blob], sample.name, { type: 'image/png' })
}
