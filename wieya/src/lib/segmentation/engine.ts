import {
  AutoModel,
  AutoProcessor,
  RawImage,
  env,
  type PreTrainedModel,
  type Processor,
} from '@huggingface/transformers'
import type { Mask } from '../cutpath'

interface MaskTensor {
  data: { length: number; [i: number]: number }
  dims: number[]
  sigmoid(): MaskTensor
}

// Privacy note: user images never leave the device. The only network access is a
// one-time fetch of the static model weights (from the HF CDN by default). To run
// fully offline, drop the model under /public/models and set SELF_HOST below.
const MODEL_ID = 'onnx-community/BiRefNet_lite-ONNX'
const SELF_HOST = false

if (SELF_HOST) {
  env.allowRemoteModels = false
  env.localModelPath = '/models/'
}

export type SegmentProgress = (info: { status: string; progress?: number }) => void

let modelPromise: Promise<{ model: PreTrainedModel; processor: Processor }> | null = null

async function getDevice(): Promise<'webgpu' | 'wasm'> {
  const gpu = (navigator as unknown as { gpu?: unknown }).gpu
  if (!gpu) return 'wasm'
  try {
    const adapter = await (gpu as { requestAdapter: () => Promise<unknown> }).requestAdapter()
    return adapter ? 'webgpu' : 'wasm'
  } catch {
    return 'wasm'
  }
}

async function loadModel(onProgress?: SegmentProgress) {
  const device = await getDevice()
  const progress_callback = onProgress
    ? (p: { status: string; progress?: number }) => onProgress(p)
    : undefined
  try {
    const model = await AutoModel.from_pretrained(MODEL_ID, {
      device,
      dtype: device === 'webgpu' ? 'fp16' : 'q8',
      progress_callback,
    })
    const processor = await AutoProcessor.from_pretrained(MODEL_ID, { progress_callback })
    return { model, processor }
  } catch (err) {
    if (device === 'webgpu') {
      // Fall back to WASM if the WebGPU build fails to initialize.
      const model = await AutoModel.from_pretrained(MODEL_ID, { device: 'wasm', dtype: 'q8' })
      const processor = await AutoProcessor.from_pretrained(MODEL_ID)
      return { model, processor }
    }
    throw err
  }
}

export function preloadSegmenter(onProgress?: SegmentProgress) {
  if (!modelPromise) modelPromise = loadModel(onProgress)
  return modelPromise
}

/**
 * Run salient-object segmentation and return an alpha mask (0..1) sized to the
 * input image. Background removal composites this alpha over the source RGB.
 */
export async function removeBackgroundMask(
  source: ImageData,
  onProgress?: SegmentProgress,
): Promise<Mask> {
  const { model, processor } = await preloadSegmenter(onProgress)
  onProgress?.({ status: 'segmenting' })

  const image = new RawImage(
    new Uint8ClampedArray(source.data),
    source.width,
    source.height,
    4,
  )
  const inputs = await processor(image)
  const output = await model({ input: inputs.pixel_values })

  // BiRefNet emits a single-channel saliency map; take the first output tensor.
  const key = 'output' in output ? 'output' : Object.keys(output)[0]
  let tensor = (output as Record<string, MaskTensor>)[key]

  const dims = tensor.dims
  const mh = dims[dims.length - 2]
  const mw = dims[dims.length - 1]

  // Apply sigmoid only if the values look like logits (outside [0,1]).
  if (needsSigmoid(tensor.data)) tensor = tensor.sigmoid()

  return {
    data: resizeMask(tensor.data, mw, mh, source.width, source.height),
    width: source.width,
    height: source.height,
  }
}

function needsSigmoid(data: { length: number; [i: number]: number }): boolean {
  const step = Math.max(1, Math.floor(data.length / 256))
  for (let i = 0; i < data.length; i += step) {
    if (data[i] < 0 || data[i] > 1) return true
  }
  return false
}

/** Bilinear resize of a single-channel mask into a Float32Array of alpha (0..1). */
function resizeMask(
  src: { length: number; [i: number]: number },
  sw: number,
  sh: number,
  dw: number,
  dh: number,
): Float32Array {
  const out = new Float32Array(dw * dh)
  for (let y = 0; y < dh; y++) {
    const fy = ((y + 0.5) * sh) / dh - 0.5
    const y0 = Math.max(0, Math.floor(fy))
    const y1 = Math.min(sh - 1, y0 + 1)
    const wy = fy - y0
    for (let x = 0; x < dw; x++) {
      const fx = ((x + 0.5) * sw) / dw - 0.5
      const x0 = Math.max(0, Math.floor(fx))
      const x1 = Math.min(sw - 1, x0 + 1)
      const wx = fx - x0
      const a = src[y0 * sw + x0]
      const b = src[y0 * sw + x1]
      const c = src[y1 * sw + x0]
      const d = src[y1 * sw + x1]
      const top = a + (b - a) * wx
      const bot = c + (d - c) * wx
      out[y * dw + x] = Math.min(1, Math.max(0, top + (bot - top) * wy))
    }
  }
  return out
}

/** A fully opaque mask, used when background removal is disabled. */
export function opaqueMask(width: number, height: number): Mask {
  return { data: new Float32Array(width * height).fill(1), width, height }
}
