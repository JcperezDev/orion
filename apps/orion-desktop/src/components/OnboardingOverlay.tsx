import { useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { ProviderView } from './Sidebar'

interface ProviderDef {
  id: string
  name: string
  dot: string
  desc: string
  recommended?: boolean
  needsKey: boolean
  keyUrl: string | null
}

const PROVIDERS: ProviderDef[] = [
  { id: 'openrouter',  name: 'OpenRouter',  dot: '#534AB7', desc: 'Access to 100+ models',            recommended: true,  needsKey: true,  keyUrl: 'https://openrouter.ai/keys' },
  { id: 'openai',      name: 'OpenAI',      dot: '#10a37f', desc: 'GPT-4o, GPT-4o-mini, o3',         recommended: false, needsKey: true,  keyUrl: 'https://platform.openai.com/api-keys' },
  { id: 'anthropic',   name: 'Anthropic',   dot: '#d4a574', desc: 'Claude Sonnet, Opus, Haiku',     recommended: false, needsKey: true,  keyUrl: 'https://console.anthropic.com/keys' },
  { id: 'google',      name: 'Google',      dot: '#4285f4', desc: 'Gemini 2.0, Gemini 1.5 Pro',      recommended: false, needsKey: true,  keyUrl: 'https://aistudio.google.com/app/apikey' },
  { id: 'deepseek',    name: 'DeepSeek',    dot: '#4d9eff', desc: 'DeepSeek V3, DeepSeek Coder',     recommended: false, needsKey: true,  keyUrl: 'https://platform.deepseek.com/api_keys' },
  { id: 'groq',        name: 'Groq',        dot: '#f55036', desc: 'Llama 3, Mixtral — ultra fast',    recommended: false, needsKey: true,  keyUrl: 'https://console.groq.com/keys' },
  { id: 'mistral',     name: 'Mistral',     dot: '#f7a800', desc: 'Mistral Large, Codestral',        recommended: false, needsKey: true,  keyUrl: 'https://console.mistral.ai/api-keys/' },
  { id: 'together',    name: 'Together AI', dot: '#10b981', desc: 'Llama, Qwen, DeepSeek models',    recommended: false, needsKey: true,  keyUrl: 'https://api.together.xyz/settings/api-keys' },
  { id: 'perplexity',  name: 'Perplexity',  dot: '#20b2aa', desc: 'Online AI with web search',       recommended: false, needsKey: true,  keyUrl: 'https://www.perplexity.ai/settings/api' },
  { id: 'minimax',     name: 'MiniMax',     dot: '#a855f7', desc: 'Abab6.5s, Hailuo AI',             recommended: false, needsKey: true,  keyUrl: 'https://www.minimax.io/' },
  { id: 'ollama',      name: 'Ollama',      dot: '#1D9E75', desc: 'Modelos locales — sin API key',   recommended: false, needsKey: false, keyUrl: null },
  { id: 'custom',      name: 'Custom',      dot: '#888888', desc: 'Cualquier API OpenAI-compatible', recommended: false, needsKey: true,  keyUrl: null },
]

interface TestModel {
  id: string
  name: string
  is_recommended?: boolean
  badges?: string[]
}

interface TestResult {
  success: boolean
  models: TestModel[]
  error?: string | null
  latency_ms: number
}

type Step = 'provider' | 'apikey' | 'testing' | 'model'

interface Props {
  onClose: () => void
}

function mapError(code: number | undefined, msg: string | undefined): string {
  if (code === 401) return 'Invalid API key. Check that it is correct.'
  if (code === 403) return 'No access. Check your plan or permissions.'
  if (code === 429) return 'Request limit reached. Wait a moment.'
  if (code === 500 || code === 502 || code === 503) return 'Server error. Try again later.'
  return msg ?? 'Unknown error.'
}

export default function OnboardingOverlay({ onClose }: Props) {
  const [step, setStep] = useState<Step>('provider')
  const [selected, setSelected] = useState<ProviderDef | null>(null)
  const [apiKey, setApiKey] = useState('')
  const [showKey, setShowKey] = useState(false)
  const [customUrl, setCustomUrl] = useState('')
  const [testResult, setTestResult] = useState<TestResult | null>(null)
  const [testError, setTestError] = useState<string | null>(null)
  const [selectedModel, setSelectedModel] = useState<string | null>(null)
  const [connectedProviders, setConnectedProviders] = useState<ProviderView[]>([])

  useEffect(() => {
    invoke<ProviderView[]>('get_connected_providers')
      .then(setConnectedProviders)
      .catch(() => {})
  }, [])

  // Close on Esc
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose()
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [onClose])

  const isConnected = (id: string) => connectedProviders.some(p => p.id === id && p.has_api_key)

  const handleTest = async () => {
    if (!selected) return
    setStep('testing')
    setTestError(null)

    try {
      const result = await invoke<TestResult>('test_provider_connection', {
        providerId: selected.id,
        apiKey: selected.needsKey ? apiKey : '',
        baseUrl: selected.id === 'custom' ? (customUrl || null) : null,
      })
      setTestResult(result)
      if (result.success) {
        const recommended = result.models.find(m => m.is_recommended)
        setSelectedModel(recommended?.id ?? result.models[0]?.id ?? null)
        setStep('model')
      } else {
        setTestError(mapError(undefined, result.error ?? undefined))
        setStep('apikey')
      }
    } catch (e: any) {
      const msg = typeof e === 'string' ? e : (e?.message ?? String(e))
      setTestError(mapError(undefined, msg))
      setStep('apikey')
    }
  }

  const handleConnect = async () => {
    if (!selected || !selectedModel) return
    try {
      await invoke('save_provider', {
        providerId: selected.id,
        apiKey: selected.needsKey ? apiKey : '',
        baseUrl: selected.id === 'custom' ? (customUrl || null) : null,
      })
      await invoke('set_active_model', { modelId: selectedModel })
      onClose()
    } catch (e) {
      setTestError(String(e))
      setStep('apikey')
    }
  }

  const onProviderClick = (p: ProviderDef) => {
    setSelected(p)
    if (!p.needsKey) {
      // Ollama — skip API key, go straight to test
      setApiKey('')
      setTimeout(() => handleTest(), 50)
    }
  }

  const onContinueFromProvider = () => {
    if (!selected) return
    if (selected.needsKey) {
      setStep('apikey')
    } else {
      handleTest()
    }
  }

  return (
    <div className="onboarding-overlay" onClick={e => { if (e.target === e.currentTarget) onClose() }}>
      <div className="onboarding-card">
        <div className="onboarding-logo">
          <span className="logo-dot" />
          ORION
        </div>
        <div className="onboarding-sub">AI Coding Agent — Connect your first provider to get started</div>

        {step === 'provider' && (
          <>
            <div className="step-label">01  Choose your AI provider</div>
            <div className="provider-grid">
              {PROVIDERS.map(p => (
                <div
                  key={p.id}
                  className={`provider-card${selected?.id === p.id ? ' selected' : ''}`}
                  onClick={() => onProviderClick(p)}
                >
                  <div className="provider-dot" style={{ background: p.dot }} />
                  <div style={{ flex: 1, minWidth: 0 }}>
                    <div className="provider-name">
                      {p.name}
                      {p.recommended && <span className="recommended-badge">Recommended</span>}
                      {isConnected(p.id) && <span className="recommended-badge" style={{ color: 'var(--green-text)', borderColor: 'var(--green)', background: 'var(--green-bg)' }}>connected</span>}
                    </div>
                    <div className="provider-desc">{p.desc}</div>
                  </div>
                </div>
              ))}
            </div>
            <div className="onboarding-actions">
              <span style={{ fontSize: 11, color: 'var(--text-tertiary, #4a4866)', fontFamily: 'JetBrains Mono, monospace' }}>
                {selected ? `Selected: ${selected.name}` : 'Select a provider'}
              </span>
              <button className="next-btn" disabled={!selected} onClick={onContinueFromProvider}>
                Continue →
              </button>
            </div>
          </>
        )}

        {step === 'apikey' && selected && (
          <>
            <div className="step-label">02  Enter your API key</div>
            <div className="api-key-section">
              <div className="api-key-label">
                <span className="api-key-label-text">
                  <span className="provider-dot" style={{ background: selected.dot }} />
                  {selected.name} API Key
                </span>
                {selected.keyUrl && (
                  <a
                    className="get-key-link"
                    onClick={e => {
                      e.preventDefault()
                      invoke('plugin:shell|open', { path: selected.keyUrl }).catch(() => {
                        window.open(selected.keyUrl!, '_blank', 'noopener,noreferrer')
                      })
                    }}
                    href={selected.keyUrl}
                    target="_blank"
                    rel="noopener noreferrer"
                  >
                    Get API key ↗
                  </a>
                )}
              </div>
              <div className="api-key-input-wrap">
                <input
                  className="api-key-input"
                  type={showKey ? 'text' : 'password'}
                  placeholder="sk-..."
                  value={apiKey}
                  onChange={e => setApiKey(e.target.value)}
                  onKeyDown={e => {
                    if (e.key === 'Enter' && apiKey.trim()) {
                      e.preventDefault()
                      handleTest()
                    }
                  }}
                  autoFocus
                />
                <button className="toggle-key-btn" onClick={() => setShowKey(s => !s)} title={showKey ? 'Hide' : 'Show'}>
                  {showKey ? '🙈' : '👁'}
                </button>
              </div>
              {selected.id === 'custom' && (
                <input
                  className="api-key-input api-key-input-wrap"
                  style={{ marginTop: 8, padding: '10px 12px', background: 'var(--bg-tertiary)', border: '0.5px solid var(--border-mid)', borderRadius: 8 }}
                  placeholder="Base URL: https://api.your-provider.com/v1"
                  value={customUrl}
                  onChange={e => setCustomUrl(e.target.value)}
                />
              )}
              <p className="api-key-note">
                Your API key is stored locally and never leaves your device.
              </p>
              {testError && <div className="error-msg">✗ {testError}</div>}
            </div>
            <div className="onboarding-actions">
              <button className="back-btn" onClick={() => setStep('provider')}>← Back</button>
              <button
                className="next-btn"
                disabled={!apiKey.trim()}
                onClick={handleTest}
              >
                Test connection →
              </button>
            </div>
          </>
        )}

        {step === 'testing' && selected && (
          <>
            <div className="step-label">03  Testing connection</div>
            <div className="testing-state">
              <span className="spinner">◌</span>
              Connecting to {selected.name}...
            </div>
          </>
        )}

        {step === 'model' && selected && testResult && (
          <>
            <div className="step-label">04  Select your model</div>
            <div className="test-success">
              ✓ Connected — {testResult.models.length} models available · {testResult.latency_ms}ms
            </div>
            <div className="model-list">
              {testResult.models.map(m => (
                <div
                  key={m.id}
                  className={`model-item${selectedModel === m.id ? ' selected' : ''}`}
                  onClick={() => setSelectedModel(m.id)}
                >
                  <span className="model-name">{m.name || m.id}</span>
                  <div className="model-badges">
                    {m.is_recommended && <span className="model-badge-tag recommended">★ Recommended</span>}
                    {m.badges?.map((b, i) => (
                      <span key={i} className="model-badge-tag">{b}</span>
                    ))}
                  </div>
                </div>
              ))}
              {testResult.models.length === 0 && (
                <div style={{ padding: 16, fontSize: 12, color: 'var(--text-tertiary)', textAlign: 'center', fontStyle: 'italic' }}>
                  No models detected. You can continue and configure the model manually from Settings.
                </div>
              )}
            </div>
            <div className="onboarding-actions">
              <button className="back-btn" onClick={() => setStep('apikey')}>← Change key</button>
              <button
                className="next-btn"
                disabled={!selectedModel}
                onClick={handleConnect}
              >
                Start →
              </button>
            </div>
          </>
        )}
      </div>
    </div>
  )
}
