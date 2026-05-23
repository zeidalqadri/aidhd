import { useWizard } from './app/wizardStore'
import { Stepper } from './components/Stepper'
import { StepUpload } from './app/steps/StepUpload'
import { StepProcess } from './app/steps/StepProcess'
import { StepLayout } from './app/steps/StepLayout'
import { StepExport } from './app/steps/StepExport'
import type { WizardStep } from './lib/types'

const ORDER: WizardStep[] = ['upload', 'process', 'layout', 'export']

export default function App() {
  const { step, items, setStep } = useWizard()
  const anyProcessed = items.some((i) => i.processed)

  // Furthest step the user is allowed to jump to.
  const maxReached: WizardStep = anyProcessed
    ? 'export'
    : items.length > 0
      ? 'process'
      : 'upload'

  return (
    <div className="mx-auto flex min-h-full max-w-4xl flex-col px-4 py-6">
      <header className="mb-6 flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
        <div className="flex items-baseline gap-2">
          <span className="text-2xl font-bold tracking-tight text-brand">Wieya</span>
          <span className="text-sm text-black/50">sticker studio</span>
        </div>
        <Stepper current={step} maxReached={maxReached} onGo={setStep} />
      </header>

      <main className="flex-1 rounded-3xl border border-black/10 bg-white/70 p-5 sm:p-8">
        {step === 'upload' && <StepUpload />}
        {step === 'process' && <StepProcess />}
        {step === 'layout' && <StepLayout />}
        {step === 'export' && <StepExport />}
      </main>

      <footer className="mt-6 text-center text-xs text-black/40">
        Private by design — your images are processed entirely in your browser and never
        uploaded. {ORDER.indexOf(step) + 1} / {ORDER.length}
      </footer>
    </div>
  )
}
