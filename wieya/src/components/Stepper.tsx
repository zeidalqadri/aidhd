import type { WizardStep } from '../lib/types'

const STEPS: { id: WizardStep; label: string }[] = [
  { id: 'upload', label: 'Upload' },
  { id: 'process', label: 'Process' },
  { id: 'layout', label: 'Layout' },
  { id: 'export', label: 'Export' },
]

interface Props {
  current: WizardStep
  maxReached: WizardStep
  onGo: (step: WizardStep) => void
}

const ORDER: WizardStep[] = ['upload', 'process', 'layout', 'export']

export function Stepper({ current, maxReached, onGo }: Props) {
  const maxIdx = ORDER.indexOf(maxReached)
  return (
    <ol className="flex items-center gap-2 sm:gap-4">
      {STEPS.map((s, i) => {
        const idx = ORDER.indexOf(s.id)
        const active = s.id === current
        const reachable = idx <= maxIdx
        return (
          <li key={s.id} className="flex items-center gap-2 sm:gap-4">
            <button
              type="button"
              disabled={!reachable}
              onClick={() => reachable && onGo(s.id)}
              className={[
                'flex items-center gap-2 rounded-full px-3 py-1.5 text-sm font-medium transition',
                active
                  ? 'bg-brand text-white'
                  : reachable
                    ? 'text-ink hover:bg-black/5'
                    : 'cursor-not-allowed text-black/30',
              ].join(' ')}
            >
              <span
                className={[
                  'flex h-6 w-6 items-center justify-center rounded-full text-xs',
                  active ? 'bg-white/25' : 'bg-black/10',
                ].join(' ')}
              >
                {i + 1}
              </span>
              {s.label}
            </button>
            {i < STEPS.length - 1 && <span className="h-px w-4 bg-black/15 sm:w-8" />}
          </li>
        )
      })}
    </ol>
  )
}
