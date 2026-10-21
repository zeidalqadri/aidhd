# Wieya — Sticker Studio

Wieya turns ordinary images into **WhatsApp-ready stickers** and **print-ready,
perforated sticker sheets** — entirely in your browser. Your images never leave
your device.

## What it does

1. **Upload** JPEG / PNG / WebP images (drag-and-drop, or try the bundled samples).
2. **Process** each image into a sticker:
   - optional in-browser background removal (a salient-object model runs locally),
   - a classic white **die-cut border**, and
   - a choice of cut shape — trace the subject (die-cut), rounded rectangle,
     rectangle, or circle (great for full-bleed designs).
3. **Layout & perforate** — pick an A4/Letter sheet and physical sticker size,
   and Wieya draws an accurate **cut / perforation path** offset from each
   sticker's outline (dashed perforation or solid kiss-cut).
4. **Export**:
   - a **WhatsApp pack** — 512×512 WebP stickers (≤100 KB), a 96×96 tray icon,
     and a spec-compliant `contents.json`, bundled as a `.zip` (plus a best-effort
     `.wastickers` file);
   - a **print PDF** with bleed-aware layout, crop & registration marks, and the
     magenta cut line; and
   - **SVG cut files** for cutting machines.

## Example workflow

| Step | Input | Result |
| --- | --- | --- |
| Upload | a photo or illustration (e.g. `public/samples/star-buddy.svg`) | image staged for processing |
| Process | background removed + white border + die-cut outline | a clean sticker on transparency |
| Layout | A4 sheet, 2″ stickers | dashed magenta perforation around each sticker |
| Export | — | `*.zip` WhatsApp pack + `wieya-print-a4.pdf` + `wieya-cut-files.zip` |

The three SVGs under `public/samples/` are original, license-clean art used by the
in-app **“Try a sample set”** button — load them to see the full pipeline end to
end without uploading anything.

## Honest constraints

- **WhatsApp:** a web page cannot add stickers to WhatsApp directly. Wieya
  produces a correctly-structured pack; you import it with a third-party
  sticker-importer app and tap “Add to WhatsApp”. The spec-compliant `.zip` is the
  guaranteed-correct artifact; the `.wastickers` file is a best-effort convenience
  for importer apps and should be verified against your target app.
- **Cricut** “Print then Cut” ingests a flattened PNG and adds *its own*
  registration marks — it does not import the SVG cut path. The SVG cut files are
  for **Silhouette Studio** and professional die-cutters; for Cricut, print the PDF
  and use Print-then-Cut.
- The cut line is a **magenta “CutContour”-style guide** (accepted by most local
  print/cut shops), not a guaranteed Roland/Esko spot-color separation. Wieya does
  not do CMYK/ICC color management.
- Static stickers only (no animated WebP).

## Privacy

All processing — decoding, background removal, border/cut generation, WebP
encoding, PDF/SVG export — runs in your browser. The only network request is a
one-time download of the background-removal model weights (from the Hugging Face
CDN by default). To run fully offline, place the model under `public/models/` and
set `SELF_HOST = true` in `src/lib/segmentation/engine.ts`.

## Tech

Vite + React + TypeScript + Tailwind, Zustand for the wizard state. Key libraries:
`@huggingface/transformers` (BiRefNet-lite, MIT), `clipper2-ts` (polygon offset),
`marchingsquares` (contour tracing), `@jsquash/webp` (deterministic WebP encoding,
incl. Safari), `pdf-lib` (print PDF), `jszip` (pack bundling).

> Deploy behind COOP/COEP headers (`Cross-Origin-Opener-Policy: same-origin`,
> `Cross-Origin-Embedder-Policy: require-corp`) to enable threaded WASM. The dev
> server already sets these.

## Develop

```bash
npm install
npm run dev      # http://localhost:5173
npm run test     # Vitest unit tests (geometry, exports, units)
npm run build    # type-check + production build
npm run lint
```

The image-processing logic lives in `src/lib` as pure, framework-agnostic modules
(`cutpath/`, `raster/`, `export/`); React components in `src/app` and
`src/components` only orchestrate.
