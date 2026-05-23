import { useMemo, useState } from 'react'
import JSZip from 'jszip'
import { useWizard } from '../wizardStore'
import { PreviewCanvas } from '../../components/PreviewCanvas'
import { buildWhatsAppZip, makeTrayIcon, type PackSticker } from '../../lib/export/pack'
import { buildWastickersFile } from '../../lib/export/wastickers'
import { generatePrintSheet } from '../../lib/export/pdfSheet'
import { buildCutSvg } from '../../lib/export/svgCut'
import { makePackIdentifier, validatePack } from '../../lib/export/whatsapp'
import { downloadBlob, downloadBytes } from '../../lib/download'
import { pathsBounds, transformPaths } from '../../lib/geometry'

export function StepExport() {
  const {
    items,
    settings,
    sheet,
    stickerWidthIn,
    packName,
    setPackName,
    publisher,
    setPublisher,
    setEmojis,
    setStep,
  } = useWizard()

  const [busy, setBusy] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)

  const processed = useMemo(
    () => items.filter((i) => i.processed?.webp512),
    [items],
  )

  const validation = useMemo(
    () =>
      validatePack({
        trayBytes: 0,
        stickers: processed.map((i) => ({
          bytes: i.processed!.webp512Bytes ?? 0,
          emojis: i.emojis,
        })),
      }),
    [processed],
  )

  const packStickers = (): PackSticker[] =>
    processed.map((i) => ({
      webp: i.processed!.webp512!,
      emojis: i.emojis,
      accessibilityText: i.fileName.replace(/\.[^.]+$/, ''),
    }))

  const run = async (label: string, fn: () => Promise<void>) => {
    setBusy(label)
    setError(null)
    try {
      await fn()
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setBusy(null)
    }
  }

  const exportWhatsApp = () =>
    run('whatsapp', async () => {
      const meta = { identifier: makePackIdentifier(packName), name: packName, publisher }
      const stickers = packStickers()
      const tray = await makeTrayIcon(stickers[0].webp)
      const zip = await buildWhatsAppZip(meta, stickers, tray)
      downloadBlob(zip, `${meta.identifier}.zip`)
    })

  const exportWastickers = () =>
    run('wastickers', async () => {
      const meta = { identifier: makePackIdentifier(packName), name: packName, publisher }
      const stickers = packStickers()
      const tray = await makeTrayIcon(stickers[0].webp)
      const file = await buildWastickersFile(meta, stickers, tray)
      downloadBlob(file, `${meta.identifier}.wastickers`)
    })

  const exportPdf = () =>
    run('pdf', async () => {
      const bytes = await generatePrintSheet(
        processed.map((i) => ({ fullRes: i.processed!.fullRes, cut: i.processed!.cut })),
        { sheet, stickerWidthIn, perforation: settings.perforation },
      )
      downloadBytes(bytes, `wieya-print-${sheet.toLowerCase()}.pdf`, 'application/pdf')
    })

  const exportSvg = () =>
    run('svg', async () => {
      const zip = new JSZip()
      processed.forEach((i, idx) => {
        const cut = i.processed!.cut
        const b = pathsBounds(cut)
        const scale = stickerWidthIn / b.width
        const paths = transformPaths(cut, scale, -b.x * scale, -b.y * scale)
        const svg = buildCutSvg(paths, {
          width: stickerWidthIn,
          height: b.height * scale,
          unit: 'in',
          perforation: settings.perforation,
          title: `Wieya cut ${idx + 1}`,
        })
        zip.file(`cut_${idx}.svg`, svg)
      })
      downloadBlob(await zip.generateAsync({ type: 'blob' }), 'wieya-cut-files.zip')
    })

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-xl font-semibold">Export</h2>
        <p className="mt-1 text-sm text-black/60">
          Assign emojis, then download your WhatsApp pack and print files.
        </p>
      </div>

      <div className="grid gap-4 sm:grid-cols-2">
        <label className="block">
          <span className="text-sm font-medium">Pack name</span>
          <input
            value={packName}
            onChange={(e) => setPackName(e.target.value)}
            className="mt-1 w-full rounded-lg border border-black/15 px-3 py-2 text-sm"
          />
        </label>
        <label className="block">
          <span className="text-sm font-medium">Publisher</span>
          <input
            value={publisher}
            onChange={(e) => setPublisher(e.target.value)}
            className="mt-1 w-full rounded-lg border border-black/15 px-3 py-2 text-sm"
          />
        </label>
      </div>

      <div className="grid grid-cols-2 gap-4 sm:grid-cols-3">
        {processed.map((item) => (
          <div key={item.id} className="space-y-2 text-center">
            <PreviewCanvas item={item} size={150} />
            <input
              value={item.emojis.join(' ')}
              onChange={(e) =>
                setEmojis(
                  item.id,
                  e.target.value.split(/\s+/).filter(Boolean).slice(0, 3),
                )
              }
              placeholder="emoji (max 3)"
              className="w-full rounded-lg border border-black/15 px-2 py-1 text-center text-sm"
            />
          </div>
        ))}
      </div>

      {!validation.ok && (
        <ul className="rounded-lg border border-amber-300 bg-amber-50 p-3 text-sm text-amber-800">
          {validation.errors.map((e, i) => (
            <li key={i}>• {e}</li>
          ))}
        </ul>
      )}
      {error && <p className="text-sm text-red-600">{error}</p>}

      <div className="grid gap-4 md:grid-cols-2">
        <div className="space-y-3 rounded-2xl border border-black/10 bg-white p-5">
          <h3 className="font-semibold">WhatsApp pack</h3>
          <p className="text-sm text-black/60">
            512×512 WebP stickers, a 96×96 tray icon, and contents.json.
          </p>
          <div className="flex flex-wrap gap-2">
            <button
              type="button"
              disabled={!validation.ok || busy != null}
              onClick={exportWhatsApp}
              className="rounded-lg bg-brand px-4 py-2 text-sm font-medium text-white hover:bg-brand-dark disabled:opacity-40"
            >
              {busy === 'whatsapp' ? 'Building…' : 'Download .zip pack'}
            </button>
            <button
              type="button"
              disabled={!validation.ok || busy != null}
              onClick={exportWastickers}
              className="rounded-lg border border-black/15 px-4 py-2 text-sm font-medium hover:bg-black/5 disabled:opacity-40"
            >
              {busy === 'wastickers' ? 'Building…' : '.wastickers'}
            </button>
          </div>
          <p className="text-xs text-black/50">
            A web page can&apos;t add stickers to WhatsApp directly — import the pack with a
            sticker-importer app, then tap “Add to WhatsApp”.
          </p>
        </div>

        <div className="space-y-3 rounded-2xl border border-black/10 bg-white p-5">
          <h3 className="font-semibold">Print &amp; cut</h3>
          <p className="text-sm text-black/60">
            A {sheet} sheet with crop &amp; registration marks and a magenta cut line, plus
            SVG cut paths.
          </p>
          <div className="flex flex-wrap gap-2">
            <button
              type="button"
              disabled={processed.length === 0 || busy != null}
              onClick={exportPdf}
              className="rounded-lg bg-brand px-4 py-2 text-sm font-medium text-white hover:bg-brand-dark disabled:opacity-40"
            >
              {busy === 'pdf' ? 'Building…' : 'Download print PDF'}
            </button>
            <button
              type="button"
              disabled={processed.length === 0 || busy != null}
              onClick={exportSvg}
              className="rounded-lg border border-black/15 px-4 py-2 text-sm font-medium hover:bg-black/5 disabled:opacity-40"
            >
              {busy === 'svg' ? 'Building…' : 'SVG cut files'}
            </button>
          </div>
          <p className="text-xs text-black/50">
            SVG cut paths import into Silhouette Studio and pro die-cutters. For Cricut, print
            the PDF/PNG and use Print&nbsp;then&nbsp;Cut (Cricut adds its own registration).
          </p>
        </div>
      </div>

      <div className="flex justify-start">
        <button type="button" onClick={() => setStep('layout')} className="text-sm text-black/60 hover:text-black">
          ← Back
        </button>
      </div>
    </div>
  )
}
