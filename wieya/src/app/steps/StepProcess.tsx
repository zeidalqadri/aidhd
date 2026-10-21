import { useWizard } from '../wizardStore'
import { PreviewCanvas } from '../../components/PreviewCanvas'
import type { CutShape } from '../../lib/types'
import { WHATSAPP_LIMITS } from '../../lib/export/whatsapp'

const SHAPES: { id: CutShape; label: string }[] = [
  { id: 'contour', label: 'Die-cut (trace subject)' },
  { id: 'roundedRect', label: 'Rounded rectangle' },
  { id: 'rect', label: 'Rectangle' },
  { id: 'circle', label: 'Circle' },
]

export function StepProcess() {
  const {
    items,
    settings,
    updateSettings,
    processAll,
    processing,
    modelStatus,
    modelProgress,
    setStep,
  } = useWizard()

  const anyProcessed = items.some((i) => i.processed)

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-xl font-semibold">Process into stickers</h2>
        <p className="mt-1 text-sm text-black/60">
          Choose how Wieya cuts each sticker, then process. Background removal runs a
          model in your browser the first time (one-time download).
        </p>
      </div>

      <div className="grid gap-6 md:grid-cols-[320px_1fr]">
        <div className="space-y-5 rounded-2xl border border-black/10 bg-white p-5">
          <label className="flex items-center justify-between gap-3">
            <span className="text-sm font-medium">Remove background</span>
            <input
              type="checkbox"
              checked={settings.removeBackground}
              onChange={(e) => updateSettings({ removeBackground: e.target.checked })}
              className="h-5 w-5 accent-[var(--color-brand)]"
            />
          </label>

          <div>
            <span className="text-sm font-medium">Cut shape</span>
            <div className="mt-2 grid gap-2">
              {SHAPES.map((s) => (
                <label
                  key={s.id}
                  className={[
                    'flex cursor-pointer items-center gap-2 rounded-lg border px-3 py-2 text-sm',
                    settings.cutShape === s.id ? 'border-brand bg-brand/5' : 'border-black/10',
                    s.id === 'contour' && !settings.removeBackground ? 'opacity-50' : '',
                  ].join(' ')}
                >
                  <input
                    type="radio"
                    name="shape"
                    checked={settings.cutShape === s.id}
                    disabled={s.id === 'contour' && !settings.removeBackground}
                    onChange={() => updateSettings({ cutShape: s.id })}
                    className="accent-[var(--color-brand)]"
                  />
                  {s.label}
                </label>
              ))}
            </div>
            {!settings.removeBackground && (
              <p className="mt-1 text-xs text-black/50">
                Die-cut needs background removal; use a shape for full-bleed designs.
              </p>
            )}
          </div>

          <Slider
            label="Border width"
            value={settings.borderPx}
            min={0}
            max={48}
            onChange={(v) => updateSettings({ borderPx: v, cutOffsetPx: Math.max(v, settings.cutOffsetPx) })}
          />
          <label className="flex items-center justify-between gap-3">
            <span className="text-sm font-medium">Border color</span>
            <input
              type="color"
              value={settings.borderColor}
              onChange={(e) => updateSettings({ borderColor: e.target.value })}
              className="h-8 w-12 rounded border border-black/10"
            />
          </label>
          <Slider
            label="Cut offset"
            value={settings.cutOffsetPx}
            min={settings.borderPx}
            max={64}
            onChange={(v) => updateSettings({ cutOffsetPx: v })}
          />
          <label className="flex items-center justify-between gap-3">
            <span className="text-sm font-medium">Perforation (dashed)</span>
            <input
              type="checkbox"
              checked={settings.perforation}
              onChange={(e) => updateSettings({ perforation: e.target.checked })}
              className="h-5 w-5 accent-[var(--color-brand)]"
            />
          </label>

          <button
            type="button"
            onClick={() => void processAll()}
            disabled={processing}
            className="w-full rounded-lg bg-brand px-4 py-2.5 font-medium text-white hover:bg-brand-dark disabled:opacity-50"
          >
            {processing ? 'Processing…' : anyProcessed ? 'Re-process all' : 'Process all'}
          </button>
          {processing && (
            <div className="text-xs text-black/60">
              {modelStatus ?? 'working'}
              {modelProgress != null && (
                <div className="mt-1 h-1.5 w-full overflow-hidden rounded bg-black/10">
                  <div
                    className="h-full bg-brand transition-all"
                    style={{ width: `${Math.round(modelProgress * 100)}%` }}
                  />
                </div>
              )}
            </div>
          )}
        </div>

        <div className="grid grid-cols-2 gap-4 sm:grid-cols-3">
          {items.map((item) => (
            <div key={item.id} className="space-y-1 text-center">
              <PreviewCanvas item={item} size={180} />
              <p className="truncate text-xs text-black/50">{item.fileName}</p>
              {item.error ? (
                <p className="text-xs text-red-600">{item.error}</p>
              ) : item.processed?.webp512Bytes != null ? (
                <p
                  className={[
                    'text-xs',
                    item.processed.webp512Bytes <= WHATSAPP_LIMITS.maxStickerBytes
                      ? 'text-emerald-600'
                      : 'text-red-600',
                  ].join(' ')}
                >
                  {Math.round(item.processed.webp512Bytes / 1024)}KB WebP
                </p>
              ) : (
                <p className="text-xs text-black/40">not processed</p>
              )}
            </div>
          ))}
        </div>
      </div>

      <div className="flex justify-between">
        <button type="button" onClick={() => setStep('upload')} className="text-sm text-black/60 hover:text-black">
          ← Back
        </button>
        <button
          type="button"
          disabled={!anyProcessed}
          onClick={() => setStep('layout')}
          className="rounded-lg bg-brand px-5 py-2.5 font-medium text-white hover:bg-brand-dark disabled:opacity-40"
        >
          Next: Layout →
        </button>
      </div>
    </div>
  )
}

function Slider({
  label,
  value,
  min,
  max,
  onChange,
}: {
  label: string
  value: number
  min: number
  max: number
  onChange: (v: number) => void
}) {
  return (
    <label className="block">
      <span className="flex justify-between text-sm font-medium">
        {label}
        <span className="text-black/50">{value}px</span>
      </span>
      <input
        type="range"
        min={min}
        max={max}
        value={value}
        onChange={(e) => onChange(Number(e.target.value))}
        className="mt-1 w-full accent-[var(--color-brand)]"
      />
    </label>
  )
}
