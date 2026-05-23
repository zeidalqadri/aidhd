import { useWizard } from '../wizardStore'
import { PreviewCanvas } from '../../components/PreviewCanvas'
import { pathsBounds } from '../../lib/geometry'
import { effectiveDpi, MIN_PRINT_DPI } from '../../lib/units'

export function StepLayout() {
  const {
    items,
    settings,
    sheet,
    setSheet,
    stickerWidthIn,
    setStickerWidthIn,
    setStep,
  } = useWizard()

  const processed = items.filter((i) => i.processed)

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-xl font-semibold">Layout &amp; perforation</h2>
        <p className="mt-1 text-sm text-black/60">
          Set the print sheet and physical sticker size. The dashed magenta line is the
          cut / perforation path.
        </p>
      </div>

      <div className="flex flex-wrap items-end gap-6 rounded-2xl border border-black/10 bg-white p-5">
        <label className="block">
          <span className="text-sm font-medium">Sheet</span>
          <select
            value={sheet}
            onChange={(e) => setSheet(e.target.value as 'A4' | 'Letter')}
            className="mt-1 block rounded-lg border border-black/15 px-3 py-2 text-sm"
          >
            <option value="A4">A4 (210 × 297 mm)</option>
            <option value="Letter">Letter (8.5 × 11 in)</option>
          </select>
        </label>

        <label className="block">
          <span className="flex justify-between text-sm font-medium">
            Sticker width <span className="text-black/50">{stickerWidthIn}″</span>
          </span>
          <input
            type="range"
            min={1}
            max={4}
            step={0.25}
            value={stickerWidthIn}
            onChange={(e) => setStickerWidthIn(Number(e.target.value))}
            className="mt-2 w-48 accent-[var(--color-brand)]"
          />
        </label>

        <div className="text-sm text-black/60">
          Cut style: <strong>{settings.perforation ? 'Perforation (dashed)' : 'Kiss-cut (solid)'}</strong>
          <span className="ml-1 text-black/40">— change on the Process step</span>
        </div>
      </div>

      <div className="grid grid-cols-2 gap-4 sm:grid-cols-3">
        {processed.map((item) => {
          const cutB = pathsBounds(item.processed!.cut)
          const dpi = Math.round(effectiveDpi(cutB.width, stickerWidthIn))
          const low = dpi < MIN_PRINT_DPI
          return (
            <div key={item.id} className="space-y-1 text-center">
              <PreviewCanvas item={item} size={180} showCut perforation={settings.perforation} />
              <p className={['text-xs', low ? 'text-amber-600' : 'text-black/50'].join(' ')}>
                ~{dpi} DPI {low && '· low for print'}
              </p>
            </div>
          )
        })}
      </div>

      <div className="flex justify-between">
        <button type="button" onClick={() => setStep('process')} className="text-sm text-black/60 hover:text-black">
          ← Back
        </button>
        <button
          type="button"
          onClick={() => setStep('export')}
          className="rounded-lg bg-brand px-5 py-2.5 font-medium text-white hover:bg-brand-dark"
        >
          Next: Export →
        </button>
      </div>
    </div>
  )
}
