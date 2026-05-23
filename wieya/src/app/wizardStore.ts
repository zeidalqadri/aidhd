import { create } from 'zustand'
import { decodeFile } from '../lib/decode'
import { processSticker } from '../lib/process'
import { DEFAULT_SETTINGS, type Settings, type StickerItem, type WizardStep } from '../lib/types'

const DEFAULT_EMOJI = '✨'
let idCounter = 0

interface WizardState {
  step: WizardStep
  items: StickerItem[]
  settings: Settings
  packName: string
  publisher: string
  sheet: 'A4' | 'Letter'
  stickerWidthIn: number
  perforation: boolean
  modelProgress: number | null
  modelStatus: string | null
  processing: boolean
  addError: string | null

  setStep: (step: WizardStep) => void
  addFiles: (files: File[]) => Promise<void>
  removeItem: (id: string) => void
  clearItems: () => void
  updateSettings: (patch: Partial<Settings>) => void
  setEmojis: (id: string, emojis: string[]) => void
  setPackName: (name: string) => void
  setPublisher: (publisher: string) => void
  setSheet: (sheet: 'A4' | 'Letter') => void
  setStickerWidthIn: (inches: number) => void
  processAll: () => Promise<void>
  reset: () => void
}

export const useWizard = create<WizardState>((set, get) => ({
  step: 'upload',
  items: [],
  settings: DEFAULT_SETTINGS,
  packName: 'My Wieya Pack',
  publisher: 'Wieya',
  sheet: 'A4',
  stickerWidthIn: 2,
  perforation: true,
  modelProgress: null,
  modelStatus: null,
  processing: false,
  addError: null,

  setStep: (step) => set({ step }),

  addFiles: async (files) => {
    set({ addError: null })
    for (const file of files) {
      try {
        const { bitmap, url } = await decodeFile(file)
        const item: StickerItem = {
          id: `s${idCounter++}`,
          fileName: file.name,
          source: bitmap,
          sourceUrl: url,
          emojis: [DEFAULT_EMOJI],
        }
        set((s) => ({ items: [...s.items, item] }))
      } catch (err) {
        set({ addError: err instanceof Error ? err.message : String(err) })
      }
    }
  },

  removeItem: (id) =>
    set((s) => {
      const item = s.items.find((i) => i.id === id)
      if (item) {
        item.source.close?.()
        URL.revokeObjectURL(item.sourceUrl)
      }
      return { items: s.items.filter((i) => i.id !== id) }
    }),

  clearItems: () =>
    set((s) => {
      s.items.forEach((i) => {
        i.source.close?.()
        URL.revokeObjectURL(i.sourceUrl)
      })
      return { items: [] }
    }),

  updateSettings: (patch) => set((s) => ({ settings: { ...s.settings, ...patch } })),

  setEmojis: (id, emojis) =>
    set((s) => ({
      items: s.items.map((i) => (i.id === id ? { ...i, emojis } : i)),
    })),

  setPackName: (packName) => set({ packName }),
  setPublisher: (publisher) => set({ publisher }),
  setSheet: (sheet) => set({ sheet }),
  setStickerWidthIn: (stickerWidthIn) => set({ stickerWidthIn }),

  processAll: async () => {
    const { items, settings } = get()
    if (items.length === 0) return
    set({ processing: true, modelStatus: settings.removeBackground ? 'loading model' : null })

    for (const item of items) {
      try {
        const processed = await processSticker(item.source, settings, (info) => {
          set({
            modelStatus: info.status,
            modelProgress: typeof info.progress === 'number' ? info.progress / 100 : get().modelProgress,
          })
        })
        set((s) => ({
          items: s.items.map((i) =>
            i.id === item.id ? { ...i, processed, error: undefined } : i,
          ),
        }))
      } catch (err) {
        set((s) => ({
          items: s.items.map((i) =>
            i.id === item.id
              ? { ...i, error: err instanceof Error ? err.message : String(err) }
              : i,
          ),
        }))
      }
    }
    set({ processing: false, modelProgress: null, modelStatus: null })
  },

  reset: () => {
    get().clearItems()
    set({ step: 'upload', settings: DEFAULT_SETTINGS, processing: false })
  },
}))
