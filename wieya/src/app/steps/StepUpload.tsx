import { useCallback, useState } from 'react'
import { useDropzone } from 'react-dropzone'
import { useWizard } from '../wizardStore'
import { ACCEPTED_TYPES } from '../../lib/decode'
import { SAMPLES, loadSampleFile } from '../../lib/samples'

export function StepUpload() {
  const { items, addFiles, removeItem, setStep, addError } = useWizard()
  const [loadingSample, setLoadingSample] = useState(false)

  const onDrop = useCallback((accepted: File[]) => void addFiles(accepted), [addFiles])
  const { getRootProps, getInputProps, isDragActive } = useDropzone({
    onDrop,
    accept: Object.fromEntries(ACCEPTED_TYPES.map((t) => [t, []])),
  })

  const tryAllSamples = async () => {
    setLoadingSample(true)
    try {
      const files = await Promise.all(SAMPLES.map(loadSampleFile))
      await addFiles(files)
    } finally {
      setLoadingSample(false)
    }
  }

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-xl font-semibold">Upload your images</h2>
        <p className="mt-1 text-sm text-black/60">
          Drop JPEG, PNG, or WebP files. Add at least 3 for a WhatsApp pack. Everything is
          processed on your device — nothing is uploaded.
        </p>
      </div>

      <div
        {...getRootProps()}
        className={[
          'flex cursor-pointer flex-col items-center justify-center rounded-2xl border-2 border-dashed p-10 text-center transition',
          isDragActive ? 'border-brand bg-brand/5' : 'border-black/15 hover:border-brand/60',
        ].join(' ')}
      >
        <input {...getInputProps()} />
        <p className="text-base font-medium">
          {isDragActive ? 'Drop them here…' : 'Drag images here, or click to browse'}
        </p>
        <p className="mt-1 text-xs text-black/50">JPEG · PNG · WebP · up to 25MB each</p>
      </div>

      <div className="flex flex-wrap items-center gap-3">
        <button
          type="button"
          onClick={tryAllSamples}
          disabled={loadingSample}
          className="rounded-lg border border-black/15 px-3 py-2 text-sm font-medium hover:bg-black/5 disabled:opacity-50"
        >
          {loadingSample ? 'Loading…' : 'Try a sample set'}
        </button>
        {items.length > 0 && (
          <button
            type="button"
            onClick={() => useWizard.getState().clearItems()}
            className="text-sm text-black/50 hover:text-black"
          >
            Clear all
          </button>
        )}
      </div>

      {addError && <p className="text-sm text-red-600">{addError}</p>}

      {items.length > 0 && (
        <div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
          {items.map((item) => (
            <div key={item.id} className="group relative">
              <img
                src={item.sourceUrl}
                alt={item.fileName}
                className="bg-checker aspect-square w-full rounded-lg object-contain"
              />
              <button
                type="button"
                onClick={() => removeItem(item.id)}
                className="absolute right-1 top-1 rounded-full bg-black/60 px-2 text-xs text-white opacity-0 transition group-hover:opacity-100"
              >
                Remove
              </button>
              <p className="mt-1 truncate text-xs text-black/50">{item.fileName}</p>
            </div>
          ))}
        </div>
      )}

      <div className="flex justify-end">
        <button
          type="button"
          disabled={items.length === 0}
          onClick={() => setStep('process')}
          className="rounded-lg bg-brand px-5 py-2.5 font-medium text-white transition hover:bg-brand-dark disabled:opacity-40"
        >
          Next: Process →
        </button>
      </div>
    </div>
  )
}
