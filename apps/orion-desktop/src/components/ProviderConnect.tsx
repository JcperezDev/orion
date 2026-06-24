import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { ArrowUp, ExternalLink, Check, X, Loader2, Eye, EyeOff } from 'lucide-react'

interface Props {
  onConnected: () => void
}

interface ProviderDef {
  id: string
  name: string
  description: string
  color: string
  recommended?: boolean
  requiresKey: boolean
  dashboard?: string
}

const PROVIDERS: ProviderDef[] = [
  { id: 'openrouter', name: 'OpenRouter', description: 'Aggregates multiple AI providers', color: '#534AB7', recommended: true, requiresKey: true, dashboard: 'https://openrouter.ai/keys' },
  { id: 'openai', name: 'OpenAI', description: 'GPT-4, GPT-4o, GPT-4o-mini', color: '#10a37f', requiresKey: true, dashboard: 'https://platform.openai.com/api-keys' },
  { id: 'anthropic', name: 'Anthropic', description: 'Claude 3.5 Sonnet, Opus, Haiku', color: '#d4a574', requiresKey: true, dashboard: 'https://console.anthropic.com/keys' },
  { id: 'google', name: 'Google', description: 'Gemini 1.5, Gemini 2.0', color: '#4285f4', requiresKey: true, dashboard: 'https://aistudio.google.com/app/apikey' },
  { id: 'deepseek', name: 'DeepSeek', description: 'DeepSeek V3, DeepSeek Coder', color: '#4d9eff', requiresKey: true, dashboard: 'https://platform.deepseek.com/api_keys' },
  { id: 'groq', name: 'Groq', description: 'Fast inference with Llama, Mixtral', color: '#f55036', requiresKey: true, dashboard: 'https://console.groq.com/keys' },
  { id: 'mistral', name: 'Mistral', description: 'Mistral Large, Codestral', color: '#f7a800', requiresKey: true, dashboard: 'https://console.mistral.ai/api-keys/' },
  { id: 'together', name: 'Together AI', description: 'Llama, Qwen, DeepSeek models', color: '#10b981', requiresKey: true, dashboard: 'https://api.together.xyz/settings/api-keys' },
  { id: 'perplexity', name: 'Perplexity', description: 'Online AI with web search', color: '#20b2aa', requiresKey: true, dashboard: 'https://www.perplexity.ai/settings/api' },
  { id: 'minimax', name: 'MiniMax', description: 'MiniMax-M3, MiniMax-Text-01', color: '#a855f7', requiresKey: true, dashboard: 'https://www.minimax.io/' },
  { id: 'ollama', name: 'Ollama', description: 'Local — sin API key', color: '#1D9E75', requiresKey: false },
  { id: 'custom', name: 'Custom', description: 'OpenAI-compatible endpoint', color: '#888888', requiresKey: true },
]

const DEFAULT_MODELS: Record<string, string> = {
  openai: 'openai:gpt-4o',
  anthropic: 'anthropic:claude-3-5-sonnet-20241022',
  google: 'google:gemini-1.5-pro',
  deepseek: 'deepseek:deepseek-chat',
  groq: 'groq:llama-3.1-70b-versatile',
  mistral: 'mistral:mistral-large-latest',
  together: 'together:meta-llama/Llama-3-70b-chat-hf',
  perplexity: 'perplexity:llama-3.1-sonar-large-128k-online',
  minimax: 'minimax:MiniMax-M3',
  openrouter: 'openrouter:anthropic/claude-3.5-sonnet',
  ollama: 'ollama:llama3.1',
  custom: 'custom:default',
}

interface TestResult {
  success: boolean
  models: string[]
  error?: string
  latency_ms: number
}

export default function ProviderConnect({ onConnected }: Props) {
  const [selectedProvider, setSelectedProvider] = useState<string | null>(null)
  const [apiKey, setApiKey] = useState('')
  const [showKey, setShowKey] = useState(false)
  const [testing, setTesting] = useState(false)
  const [testResult, setTestResult] = useState<TestResult | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [saving, setSaving] = useState(false)

  useEffect(() => {
    setApiKey('')
    setTestResult(null)
    setError(null)
  }, [selectedProvider])

  const selected = PROVIDERS.find(p => p.id === selectedProvider)
  const needsKey = selected?.requiresKey ?? true

  async function runTest(): Promise<TestResult | null> {
    if (!selectedProvider) return null
    if (needsKey && !apiKey.trim()) return null

    setTesting(true)
    setError(null)
    setTestResult(null)

    try {
      const result = await invoke<TestResult>('test_provider_connection', {
        providerId: selectedProvider,
        apiKey: apiKey.trim(),
      })
      setTestResult(result)
      if (!result.success) {
        setError(result.error || 'Connection failed')
      }
      return result
    } catch (err) {
      const fail: TestResult = { success: false, models: [], error: String(err), latency_ms: 0 }
      setError(String(err))
      setTestResult(fail)
      return fail
    } finally {
      setTesting(false)
    }
  }

  async function handleConnect() {
    if (!selectedProvider) return
    if (needsKey && !apiKey.trim()) return

    let result = testResult
    if (!result?.success) {
      result = await runTest()
    }

    if (!result?.success) {
      setError(result?.error || 'Connection failed')
      return
    }

    setSaving(true)
    setError(null)

    try {
      if (needsKey) {
        await invoke('save_provider_api_key', {
          providerId: selectedProvider,
          apiKey: apiKey.trim(),
        })
      } else {
        await invoke('set_provider_enabled', { providerId: selectedProvider, enabled: true }).catch(() => {})
      }
      await invoke('reload_registry')

      const defaultModel = DEFAULT_MODELS[selectedProvider]
      if (defaultModel) {
        try {
          await invoke('set_default_model', { modelId: defaultModel })
        } catch {}
      }

      onConnected()
    } catch (err) {
      setError(String(err))
    } finally {
      setSaving(false)
    }
  }

  function handleTryAgain() {
    setTestResult(null)
    setError(null)
    setApiKey('')
  }

  return (
    <div className="relative min-h-screen w-full overflow-hidden bg-bg-primary text-text-primary" style={{ fontFamily: 'system-ui, -apple-system, sans-serif' }}>
      <div className="grid-bg pointer-events-none absolute inset-0" />
      <div className="pointer-events-none absolute inset-0 bg-[radial-gradient(ellipse_at_top,rgba(83,74,183,0.18),transparent_60%)]" />

      <div className="relative mx-auto flex min-h-screen max-w-[900px] flex-col px-6 py-12">
        {/* Header */}
        <header className="mb-10">
          <div className="flex items-center gap-3 mb-3">
            <span className="size-2 rounded-full" style={{ background: 'var(--accent)' }} />
            <h1
              className="text-text-primary"
              style={{
                fontFamily: "'JetBrains Mono', monospace",
                letterSpacing: '0.15em',
                fontWeight: 700,
                fontSize: '24px',
              }}
            >
              ORION
            </h1>
          </div>
          <p
            className="text-text-secondary"
            style={{
              fontFamily: "'JetBrains Mono', monospace",
              fontSize: '13px',
            }}
          >
            Bring your own keys. Route across providers. Run MCPs locally.
          </p>
        </header>

        {/* Step 01 — Choose provider */}
        <section className="mb-8">
          <div className="mb-3 flex items-center gap-2">
            <span
              style={{
                color: 'var(--accent)',
                fontFamily: "'JetBrains Mono', monospace",
                fontSize: '11px',
                letterSpacing: '0.08em',
                fontWeight: 600,
              }}
            >
              01
            </span>
            <span
              className="text-text-primary"
              style={{
                fontFamily: "'JetBrains Mono', monospace",
                fontSize: '11px',
                letterSpacing: '0.06em',
                textTransform: 'uppercase',
                fontWeight: 600,
              }}
            >
              Choose your AI provider
            </span>
          </div>

          <div className="grid grid-cols-2 gap-3">
            {PROVIDERS.map((provider) => {
              const isSelected = selectedProvider === provider.id
              return (
                <button
                  key={provider.id}
                  type="button"
                  onClick={() => setSelectedProvider(provider.id)}
                  className="relative flex items-start gap-3 text-left transition"
                  style={{
                    padding: '14px 16px',
                    borderRadius: '8px',
                    background: isSelected ? 'var(--accent-muted)' : 'rgba(17,17,22,0.4)',
                    border: isSelected
                      ? '0.5px solid var(--accent)'
                      : '0.5px solid var(--border-subtle)',
                    transitionDuration: '150ms',
                    cursor: 'pointer',
                  }}
                  onMouseEnter={(e) => {
                    if (!isSelected) e.currentTarget.style.background = 'var(--bg-tertiary)'
                  }}
                  onMouseLeave={(e) => {
                    if (!isSelected) e.currentTarget.style.background = 'rgba(17,17,22,0.4)'
                  }}
                >
                  {isSelected && (
                    <span
                      className="absolute right-2 top-2 grid place-items-center"
                      style={{
                        width: '16px',
                        height: '16px',
                        borderRadius: '50%',
                        background: 'var(--accent)',
                      }}
                    >
                      <Check className="size-3 text-white" />
                    </span>
                  )}
                  <span
                    className="mt-0.5 rounded-full flex-shrink-0"
                    style={{ width: '8px', height: '8px', background: provider.color }}
                  />
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2 mb-1">
                      <span
                        className="text-text-primary"
                        style={{ fontSize: '13px', fontWeight: 500 }}
                      >
                        {provider.name}
                      </span>
                      {provider.recommended && (
                        <span
                          className="text-accent-text"
                          style={{
                            background: 'var(--accent-muted)',
                            border: '0.5px solid var(--accent)',
                            fontSize: '10px',
                            letterSpacing: '0.06em',
                            padding: '2px 8px',
                            borderRadius: '20px',
                            textTransform: 'uppercase',
                            fontWeight: 600,
                            fontFamily: "'JetBrains Mono', monospace",
                          }}
                        >
                          Recommended
                        </span>
                      )}
                    </div>
                    <p
                      className="text-text-secondary truncate"
                      style={{ fontSize: '11px' }}
                    >
                      {provider.description}
                    </p>
                  </div>
                </button>
              )
            })}
          </div>
        </section>

        {/* Step 02 — API Key */}
        {selectedProvider && needsKey && (
          <section className="mb-8">
            <div className="mb-3 flex items-center gap-2">
              <span
                style={{
                  color: 'var(--accent)',
                  fontFamily: "'JetBrains Mono', monospace",
                  fontSize: '11px',
                  letterSpacing: '0.08em',
                  fontWeight: 600,
                }}
              >
                02
              </span>
              <span
                className="text-text-primary"
                style={{
                  fontFamily: "'JetBrains Mono', monospace",
                  fontSize: '11px',
                  letterSpacing: '0.06em',
                  textTransform: 'uppercase',
                  fontWeight: 600,
                }}
              >
                Enter your API key
              </span>
            </div>
            <div
              style={{
                borderRadius: '8px',
                border: '0.5px solid var(--border-subtle)',
                background: 'rgba(17,17,22,0.4)',
                padding: '16px',
              }}
            >
              <div className="mb-3 flex items-center justify-between">
                <span
                  className="text-text-secondary"
                  style={{ fontSize: '12px' }}
                >
                  {selected?.name} API Key
                </span>
                {selected?.dashboard && (
                  <a
                    href={selected.dashboard}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="flex items-center gap-1 hover:underline"
                    style={{ color: 'var(--accent)', fontSize: '11px' }}
                  >
                    Get API key
                    <ExternalLink className="size-3" />
                  </a>
                )}
              </div>
              <div className="relative">
                <input
                  type={showKey ? 'text' : 'password'}
                  value={apiKey}
                  onChange={(e) => setApiKey(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') {
                      e.preventDefault()
                      handleConnect()
                    }
                  }}
                  placeholder="sk-..."
                  autoFocus
                  className="w-full focus:outline-none"
                  style={{
                    background: 'var(--bg-primary)',
                    border: '0.5px solid var(--border-subtle)',
                    borderRadius: '6px',
                    padding: '12px 44px 12px 14px',
                    color: 'var(--text-primary)',
                    fontFamily: "'JetBrains Mono', monospace",
                    fontSize: '12px',
                  }}
                  onFocus={(e) => {
                    e.currentTarget.style.borderColor = 'var(--accent)'
                  }}
                  onBlur={(e) => {
                    e.currentTarget.style.borderColor = 'var(--border-subtle)'
                  }}
                />
                <button
                  type="button"
                  onClick={() => setShowKey(s => !s)}
                  className="absolute right-3 top-1/2 -translate-y-1/2 text-text-secondary hover:text-text-primary"
                  aria-label="Toggle visibility"
                >
                  {showKey ? <EyeOff className="size-4" /> : <Eye className="size-4" />}
                </button>
              </div>
              <p
                className="mt-3 text-text-tertiary"
                style={{ fontSize: '11px' }}
              >
                Your API key is stored locally and never sent to our servers.
              </p>
            </div>
          </section>
        )}

        {/* Test result */}
        {testResult && (
          <div
            className="mb-6"
            style={{
              borderRadius: '8px',
              border: `0.5px solid ${testResult.success ? 'var(--green)' : 'var(--red)'}`,
              background: testResult.success ? 'var(--green-bg)' : 'var(--red-bg)',
              padding: '12px 16px',
            }}
          >
            <div
              className="flex items-center gap-2"
              style={{
                color: testResult.success ? 'var(--green-text)' : 'var(--red-text)',
                fontSize: '12px',
                fontWeight: 500,
              }}
            >
              {testResult.success ? <Check className="size-4" /> : <X className="size-4" />}
              {testResult.success
                ? `Connected — ${testResult.models.length} models available (${testResult.latency_ms}ms)`
                : (testResult.error || 'Connection failed')}
            </div>
            {testResult.success && testResult.models.length > 0 && (
              <div className="mt-2 ml-6 space-y-1">
                {testResult.models.slice(0, 3).map((m) => (
                  <div
                    key={m}
                    className="text-text-secondary"
                    style={{ fontFamily: "'JetBrains Mono', monospace", fontSize: '11px' }}
                  >
                    {m}
                  </div>
                ))}
              </div>
            )}
          </div>
        )}

        {/* Error */}
        {error && !testResult && (
          <div
            className="mb-6"
            style={{
              borderRadius: '8px',
              border: '0.5px solid var(--red)',
              background: 'var(--red-bg)',
              padding: '12px 16px',
            }}
          >
            <div
              className="flex items-center gap-2"
              style={{ color: 'var(--red-text)', fontSize: '12px' }}
            >
              <X className="size-4" />
              {error}
            </div>
          </div>
        )}

        {/* Step 03 — Test connection */}
        {selectedProvider && (
          <section className="mb-8">
            <div className="mb-3 flex items-center gap-2">
              <span
                style={{
                  color: 'var(--accent)',
                  fontFamily: "'JetBrains Mono', monospace",
                  fontSize: '11px',
                  letterSpacing: '0.08em',
                  fontWeight: 600,
                }}
              >
                03
              </span>
              <span
                className="text-text-primary"
                style={{
                  fontFamily: "'JetBrains Mono', monospace",
                  fontSize: '11px',
                  letterSpacing: '0.06em',
                  textTransform: 'uppercase',
                  fontWeight: 600,
                }}
              >
                Test connection
              </span>
            </div>
            <div className="flex gap-3">
              {testResult?.success ? (
                <>
                  <button
                    type="button"
                    onClick={handleConnect}
                    disabled={saving}
                    className="flex-1 flex items-center justify-center gap-2 transition"
                    style={{
                      background: 'var(--accent)',
                      color: 'white',
                      borderRadius: '8px',
                      padding: '12px 24px',
                      fontSize: '13px',
                      fontWeight: 500,
                      border: 'none',
                      cursor: saving ? 'wait' : 'pointer',
                      opacity: saving ? 0.6 : 1,
                    }}
                  >
                    {saving ? (
                      <>
                        <Loader2 className="size-4 animate-spin" />
                        Saving...
                      </>
                    ) : (
                      <>
                        Continue
                        <ArrowUp className="size-4" />
                      </>
                    )}
                  </button>
                  <button
                    type="button"
                    onClick={handleTryAgain}
                    className="px-6 transition"
                    style={{
                      background: 'transparent',
                      color: 'var(--text-secondary)',
                      border: '0.5px solid var(--border-mid)',
                      borderRadius: '8px',
                      padding: '12px 24px',
                      fontSize: '13px',
                      cursor: 'pointer',
                    }}
                  >
                    Try again
                  </button>
                </>
              ) : (
                <button
                  type="button"
                  onClick={handleConnect}
                  disabled={testing || saving || (needsKey && !apiKey.trim())}
                  className="flex-1 flex items-center justify-center gap-2 transition"
                  style={{
                    background: 'var(--accent-muted)',
                    color: 'var(--accent-text)',
                    borderRadius: '8px',
                    padding: '12px 24px',
                    fontSize: '13px',
                    fontWeight: 500,
                    border: '0.5px solid var(--accent)',
                    cursor: (testing || saving || (needsKey && !apiKey.trim())) ? 'not-allowed' : 'pointer',
                    opacity: (testing || saving || (needsKey && !apiKey.trim())) ? 0.5 : 1,
                  }}
                >
                  {testing ? (
                    <>
                      <Loader2 className="size-4 animate-spin" />
                      Testing connection...
                    </>
                  ) : (
                    <>
                      Connect
                      <ArrowUp className="size-4" />
                    </>
                  )}
                </button>
              )}
            </div>
          </section>
        )}
      </div>
    </div>
  )
}
