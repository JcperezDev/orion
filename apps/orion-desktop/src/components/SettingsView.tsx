import { Check } from 'lucide-react'
import type { Provider } from '../App'

interface Props {
  providers: Provider[]
  defaultModel: string | null
}

export default function SettingsView({ providers, defaultModel }: Props) {
  const connectedProviders = providers.filter(p => p.status === 'connected')

  return (
    <div className="h-full overflow-y-auto p-6">
      <h2 className="text-xl font-bold mb-6">Settings</h2>
      <div className="max-w-xl space-y-6">
        <div className="bg-surface border border-border-subtle rounded-xl p-5">
          <h3 className="font-semibold mb-4">Default Model</h3>
          <p className="text-sm text-text-muted mb-3">
            Current default: <span className="text-text font-mono">{defaultModel || 'Not set'}</span>
          </p>
          <p className="text-sm text-text-subtle">
            Go to the Models page to search and select a default model for your conversations.
          </p>
        </div>

        <div className="bg-surface border border-border-subtle rounded-xl p-5">
          <h3 className="font-semibold mb-4">Connected Providers</h3>
          {connectedProviders.length > 0 ? (
            <div className="space-y-2">
              {connectedProviders.map(p => (
                <div key={p.id} className="flex items-center justify-between text-sm">
                  <span>{p.name}</span>
                  <span className="text-success flex items-center gap-1"><Check className="size-3.5" /> Connected</span>
                </div>
              ))}
            </div>
          ) : (
            <p className="text-sm text-text-muted">No providers connected yet.</p>
          )}
        </div>

        <div className="bg-surface border border-border-subtle rounded-xl p-5">
          <h3 className="font-semibold mb-4">Local AI</h3>
          <label className="flex items-center gap-3 cursor-pointer">
            <input
              type="checkbox"
              disabled
              className="w-4 h-4 rounded border-border bg-surface-raised text-primary"
            />
            <div>
              <span className="text-sm">Enable local AI (Ollama)</span>
              <p className="text-xs text-text-subtle">Not enabled by default. Requires Ollama running locally.</p>
            </div>
          </label>
        </div>

        <div className="bg-surface border border-border-subtle rounded-xl p-5">
          <h3 className="font-semibold mb-4">About</h3>
          <div className="text-sm text-text-muted">
            <p>ORION v0.1.0</p>
            <p className="mt-1">AI Model Router with BYOK support</p>
            <p className="mt-2 text-xs text-text-subtle">
              Securely connect your own API keys. No local model downloads required.
            </p>
          </div>
        </div>
      </div>
    </div>
  )
}
