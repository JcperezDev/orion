import { useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { ArrowUp, ExternalLink, Check, X, Loader2 } from 'lucide-react'

interface Props {
  onConnected: () => void
}

const PROVIDERS = [
  { id: 'openrouter', name: 'OpenRouter', website: 'https://openrouter.ai', recommended: true, description: 'Aggregates multiple AI providers', color: 'var(--accent-blue)' },
  { id: 'openai', name: 'OpenAI', website: 'https://platform.openai.com', description: 'GPT-4, GPT-4o, GPT-4o-mini', color: 'var(--accent-green)' },
  { id: 'anthropic', name: 'Anthropic', website: 'https://console.anthropic.com', description: 'Claude 3.5 Sonnet, Opus, Haiku', color: 'var(--accent-amber)' },
  { id: 'google', name: 'Google Gemini', website: 'https://aistudio.google.com', description: 'Gemini 1.5, Gemini 2.0', color: 'var(--accent-purple)' },
  { id: 'deepseek', name: 'DeepSeek', website: 'https://platform.deepseek.com', description: 'DeepSeek V3, DeepSeek Coder', color: 'var(--accent-blue)' },
  { id: 'groq', name: 'Groq', website: 'https://console.groq.com', description: 'Fast inference with Llama, Mixtral', color: 'var(--accent-purple)' },
  { id: 'mistral', name: 'Mistral', website: 'https://console.mistral.ai', description: 'Mistral Large, Codestral', color: 'var(--accent-amber)' },
  { id: 'together', name: 'Together AI', website: 'https://together.ai', description: 'Llama, Qwen, DeepSeek models', color: 'var(--accent-green)' },
  { id: 'perplexity', name: 'Perplexity', website: 'https://perplexity.ai', description: 'Online AI with web search', color: 'var(--accent-blue)' },
  { id: 'minimax', name: 'MiniMax', website: 'https://platform.minimax.chat', description: 'Abab6.5s, Hailuo AI', color: 'var(--accent-purple)' },
]

function Pill({ label, tone }: { label: string; tone: 'blue' | 'green' | 'purple' | 'amber' }) {
  const map = {
    blue: 'text-accent-blue bg-accent-blue-bg border-accent-blue/30',
    green: 'text-accent-green bg-accent-green-bg border-accent-green/30',
    purple: 'text-accent-purple bg-accent-purple-bg border-accent-purple/30',
    amber: 'text-accent-amber bg-accent-amber-bg border-accent-amber/30',
  }
  return (
    <span className={`inline-flex items-center gap-1.5 rounded-full border px-2.5 py-0.5 text-[10px] tracking-[0.12em] uppercase ${map[tone]}`}>
      <span className="size-1.5 rounded-full bg-current" />
      {label}
    </span>
  )
}

export default function ProviderConnect({ onConnected }: Props) {
  const [selectedProvider, setSelectedProvider] = useState<string | null>(null)
  const [apiKey, setApiKey] = useState('')
  const [testing, setTesting] = useState(false)
  const [syncing, setSyncing] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [success, setSuccess] = useState(false)

  async function handleConnect() {
    if (!selectedProvider || !apiKey.trim()) return

    setTesting(true)
    setError(null)
    setSuccess(false)

    try {
      const result = await invoke<{ success: boolean; error?: string }>('test_provider_connection', {
        providerId: selectedProvider,
        apiKey: apiKey.trim()
      })

      if (result.success) {
        setSuccess(true)
        setSyncing(true)
        try {
          await invoke('save_provider_api_key', { 
            providerId: selectedProvider, 
            apiKey: apiKey.trim() 
          })
          await invoke('reload_registry')
          
          const defaultModels: Record<string, string> = {
            openai: 'openai:gpt-4o',
            anthropic: 'anthropic:claude-3-5-sonnet-20241022',
            google: 'google:gemini-1.5-pro',
            deepseek: 'deepseek:deepseek-chat',
            groq: 'groq:llama-3.1-70b-versatile',
            mistral: 'mistral:mistral-large-latest',
            together: 'together:meta-llama/Llama-3-70b-chat-hf',
            perplexity: 'perplexity:llama-3.1-sonar-large-128k-online',
            minimax: 'minimax:abab6.5s',
            openrouter: 'openrouter:anthropic/claude-3.5-sonnet',
          }
          
          const defaultModel = defaultModels[selectedProvider]
          if (defaultModel) {
            await invoke('set_default_model', { modelId: defaultModel })
          }
        } catch (saveErr) {
          console.warn('Failed to setup:', saveErr)
        }
        setSyncing(false)
        onConnected()
      } else {
        setError(result.error || 'Connection failed. Check your API key.')
      }
    } catch (err) {
      setError(String(err))
    } finally {
      setTesting(false)
    }
  }

  const selected = PROVIDERS.find(p => p.id === selectedProvider)

  return (
    <div className="relative min-h-screen w-full overflow-hidden bg-background text-foreground font-mono">
      <div className="grid-bg pointer-events-none absolute inset-0" />
      <div className="pointer-events-none absolute inset-0 bg-[radial-gradient(ellipse_at_top,oklch(0.3_0.1_260/0.25),transparent_60%)]" />

      <div className="relative mx-auto flex min-h-screen max-w-[900px] flex-col px-6 py-12">
        {/* Header */}
        <header className="mb-10">
          <div className="flex items-center gap-3 mb-4">
            <div className="relative grid size-10 place-items-center">
              <span className="absolute inset-0 rounded-md border border-accent-blue/40" />
              <span className="absolute inset-2 rounded-sm border border-accent-blue/20 rotate-45" />
              <span className="relative size-2 rounded-full bg-accent-blue" />
            </div>
            <div className="text-xl font-semibold tracking-[0.35em] text-text">ORION</div>
          </div>
          <p className="text-subtle text-sm">Bring your own keys. Route across providers. Run MCPs locally.</p>
        </header>

        {/* Step 1: Choose provider */}
        <div className="mb-8">
          <div className="mb-3 flex items-center gap-2">
            <span className="text-[10px] tracking-[0.2em] uppercase text-accent-blue">01</span>
            <span className="text-[11px] tracking-[0.15em] uppercase text-text">Choose your AI provider</span>
          </div>
          <div className="grid grid-cols-2 gap-3">
            {PROVIDERS.map((provider) => (
              <button
                key={provider.id}
                onClick={() => {
                  setSelectedProvider(provider.id)
                  setError(null)
                  setSuccess(false)
                }}
                className={`flex items-start gap-3 rounded-lg border p-4 text-left transition ${
                  selectedProvider === provider.id
                    ? 'border-accent-blue/50 bg-accent-blue-bg/30'
                    : 'border-border bg-surface/40 hover:bg-surface-2/40'
                }`}
              >
                <span className="mt-0.5 size-2 rounded-full" style={{ background: provider.color }} />
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2">
                    <span className="text-xs font-medium text-text">{provider.name}</span>
                    {provider.recommended && <Pill label="recommended" tone="blue" />}
                  </div>
                  <p className="mt-1 text-[10px] text-subtle line-clamp-1">{provider.description}</p>
                </div>
              </button>
            ))}
          </div>
        </div>

        {/* Step 2: API Key */}
        {selectedProvider && (
          <div className="mb-8">
            <div className="mb-3 flex items-center gap-2">
              <span className="text-[10px] tracking-[0.2em] uppercase text-accent-blue">02</span>
              <span className="text-[11px] tracking-[0.15em] uppercase text-text">Enter your API key</span>
            </div>
            <div className="rounded-lg border border-border bg-surface/40 p-4">
              <div className="mb-3 flex items-center justify-between">
                <span className="text-xs text-text-dim">{selected?.name} API Key</span>
                <a
                  href={selected?.website}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="flex items-center gap-1 text-[10px] text-accent-blue hover:underline"
                >
                  Get API key <ExternalLink className="size-3" />
                </a>
              </div>
              <input
                type="password"
                value={apiKey}
                onChange={(e) => {
                  setApiKey(e.target.value)
                  setError(null)
                  setSuccess(false)
                }}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') {
                    e.preventDefault()
                    handleConnect()
                  }
                }}
                placeholder="sk-..."
                className="w-full rounded border border-border bg-[var(--surface)] px-4 py-3 text-xs text-[var(--foreground)] placeholder:text-[var(--muted)] focus:border-accent-blue/60 focus:outline-none font-mono"
              />
              <p className="mt-2 text-[10px] text-subtle">Your API key is stored locally and never sent to our servers.</p>
            </div>
          </div>
        )}

        {/* Error */}
        {error && (
          <div className="mb-6 rounded-lg border border-accent-red/30 bg-accent-red/10 px-4 py-3">
            <div className="flex items-center gap-2 text-xs text-accent-red">
              <X className="size-4" />
              {error}
            </div>
          </div>
        )}

        {/* Success */}
        {success && (
          <div className="mb-6 rounded-lg border border-accent-green/30 bg-accent-green/10 px-4 py-3">
            <div className="flex items-center gap-2 text-xs text-accent-green">
              <Check className="size-4" />
              Connection successful! Syncing models...
            </div>
          </div>
        )}

        {/* Step 3: Connect */}
        {selectedProvider && (
          <div className="mb-8">
            <div className="mb-3 flex items-center gap-2">
              <span className="text-[10px] tracking-[0.2em] uppercase text-accent-blue">03</span>
              <span className="text-[11px] tracking-[0.15em] uppercase text-text">Test connection</span>
            </div>
            <button
              onClick={handleConnect}
              disabled={!apiKey.trim() || testing || syncing}
              className="flex w-full items-center justify-center gap-2 rounded-lg border border-accent-blue/40 bg-accent-blue-bg px-6 py-3 text-xs font-medium text-accent-blue transition hover:border-accent-blue/70 disabled:opacity-50"
            >
              {testing ? (
                <>
                  <Loader2 className="size-4 animate-spin" />
                  Testing connection...
                </>
              ) : syncing ? (
                <>
                  <Loader2 className="size-4 animate-spin" />
                  Syncing models...
                </>
              ) : (
                <>
                  <ArrowUp className="size-4" />
                  Connect
                </>
              )}
            </button>
          </div>
        )}


      </div>
    </div>
  )
}
