import type { Provider } from '../App'

interface Props {
  providers: Provider[]
  onSync: (providerId?: string) => void
  loading: boolean
}

const PROVIDER_INFO: Record<string, { name: string; website: string; description: string }> = {
  openai: { name: 'OpenAI', website: 'https://platform.openai.com', description: 'GPT-4, GPT-4o, GPT-4o-mini' },
  anthropic: { name: 'Anthropic', website: 'https://console.anthropic.com', description: 'Claude 3.5 Sonnet, Opus, Haiku' },
  google: { name: 'Google Gemini', website: 'https://aistudio.google.com', description: 'Gemini 1.5, Gemini 2.0' },
  deepseek: { name: 'DeepSeek', website: 'https://platform.deepseek.com', description: 'DeepSeek V3, DeepSeek Coder' },
  groq: { name: 'Groq', website: 'https://console.groq.com', description: 'Fast inference with Llama, Mixtral' },
  mistral: { name: 'Mistral', website: 'https://console.mistral.ai', description: 'Mistral Large, Codestral' },
  together: { name: 'Together AI', website: 'https://together.ai', description: 'Llama, Qwen, DeepSeek models' },
  perplexity: { name: 'Perplexity', website: 'https://perplexity.ai', description: 'Online AI with web search' },
  minimax: { name: 'MiniMax', website: 'https://platform.minimax.chat', description: 'Abab6.5s, Hailuo AI' },
  openrouter: { name: 'OpenRouter', website: 'https://openrouter.ai', description: 'Aggregates multiple AI providers' },
}

const STATUS_LABELS: Record<string, { label: string; color: string }> = {
  not_configured: { label: 'Not configured', color: 'text-text-subtle' },
  missing_key: { label: 'Missing API key', color: 'text-warning' },
  testing: { label: 'Testing...', color: 'text-text-muted' },
  connected: { label: 'Connected', color: 'text-success' },
  invalid_key: { label: 'Invalid API key', color: 'text-error' },
  offline: { label: 'Offline', color: 'text-error' },
  disabled: { label: 'Disabled', color: 'text-text-subtle' },
}

export default function ProvidersView({ providers, onSync, loading }: Props) {
  return (
    <div className="h-full overflow-y-auto p-6">
      <div className="flex items-center justify-between mb-6">
        <h2 className="text-xl font-bold">AI Providers</h2>
        <button
          onClick={() => onSync()}
          disabled={loading}
          className="px-4 py-2 bg-primary hover:bg-primary-hover text-white rounded-lg text-sm font-medium disabled:opacity-50"
        >
          {loading ? 'Syncing...' : 'Sync Models'}
        </button>
      </div>

      <div className="grid grid-cols-2 gap-4">
        {providers.map((provider) => {
          const info = PROVIDER_INFO[provider.id] || { name: provider.name, website: '', description: '' }
          const status = STATUS_LABELS[provider.status] || STATUS_LABELS.not_configured

          return (
            <div key={provider.id} className="bg-surface border border-border-subtle rounded-xl p-5">
              <div className="flex items-start justify-between mb-3">
                <div>
                  <h3 className="font-semibold">{info.name}</h3>
                  <a
                    href={info.website}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-xs text-accent hover:underline"
                  >
                    {info.website}
                  </a>
                </div>
                <span className={`text-xs font-medium ${status.color}`}>
                  {status.label}
                </span>
              </div>

              <p className="text-sm text-text-muted mb-3">{info.description}</p>

              <div className="flex items-center justify-between">
                <span className="text-xs text-text-subtle">
                  {provider.models_count} models
                </span>
                <button
                  onClick={() => onSync(provider.id)}
                  disabled={loading || provider.status !== 'connected'}
                  className="text-xs px-3 py-1 bg-surface-raised hover:bg-border rounded transition-colors disabled:opacity-50"
                >
                  Sync
                </button>
              </div>

              {provider.error && (
                <div className="mt-2 text-xs text-error">
                  {provider.error}
                </div>
              )}
            </div>
          )
        })}
      </div>

      {providers.length === 0 && !loading && (
        <div className="text-center text-text-subtle py-12">
          No providers configured. Go to the onboarding flow to connect a provider.
        </div>
      )}
    </div>
  )
}
