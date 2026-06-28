import { useEffect, useMemo, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'

interface ProviderView {
  id: string
  name: string
  has_api_key: boolean
}

interface ModelView {
  id: string // full "provider:model"
  provider: string
  name: string
  context_window?: number
  supports_tools: boolean
  is_free: boolean
}

function fmtCtx(n?: number): string {
  if (!n) return ''
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(n % 1_000_000 ? 1 : 0)}M ctx`
  if (n >= 1000) return `${Math.round(n / 1000)}K ctx`
  return `${n} ctx`
}

export function ModelPicker({
  open,
  onClose,
  onSelected,
}: {
  open: boolean
  onClose: () => void
  onSelected: (modelId: string) => void
}) {
  const [providers, setProviders] = useState<ProviderView[]>([])
  const [models, setModels] = useState<ModelView[]>([])
  const [filter, setFilter] = useState('')
  const [current, setCurrent] = useState<string | null>(null)

  useEffect(() => {
    if (!open) return
    setFilter('')
    Promise.all([
      invoke<ProviderView[]>('list_providers'),
      invoke<ModelView[]>('list_models', { provider: null }),
      invoke<string | null>('get_default_model'),
    ])
      .then(([p, m, def]) => {
        setProviders(p)
        setModels(m)
        setCurrent(def)
      })
      .catch(() => {})
  }, [open])

  // Esc closes
  useEffect(() => {
    if (!open) return
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose()
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [open, onClose])

  const connected = useMemo(
    () => new Set(providers.filter(p => p.has_api_key || p.id === 'ollama').map(p => p.id)),
    [providers]
  )

  const groups = useMemo(() => {
    const q = filter.trim().toLowerCase()
    const byProvider = new Map<string, ModelView[]>()
    for (const m of models) {
      if (!connected.has(m.provider)) continue
      if (q && !(`${m.name} ${m.id}`.toLowerCase().includes(q))) continue
      const arr = byProvider.get(m.provider) ?? []
      arr.push(m)
      byProvider.set(m.provider, arr)
    }
    return byProvider
  }, [models, connected, filter])

  const unconnected = useMemo(
    () => providers.filter(p => !p.has_api_key && p.id !== 'ollama'),
    [providers]
  )

  if (!open) return null

  const providerName = (id: string) => providers.find(p => p.id === id)?.name ?? id

  async function pick(modelId: string) {
    try {
      await invoke('set_active_model', { modelId })
    } catch {
      /* ignore */
    }
    onSelected(modelId)
    onClose()
  }

  function openSettingsProviders() {
    window.dispatchEvent(new CustomEvent('orion:open-settings', { detail: 'providers' }))
    onClose()
  }

  return (
    <div className="picker-backdrop" onClick={onClose} role="dialog" aria-modal="true">
      <div className="picker" onClick={e => e.stopPropagation()}>
        <div className="picker-header">
          <span className="picker-title">Select a model</span>
          <button className="picker-close" onClick={onClose} aria-label="Close">✕</button>
        </div>
        <input
          className="picker-search"
          placeholder="Search models…"
          value={filter}
          onChange={e => setFilter(e.target.value)}
          autoFocus
        />
        <div className="picker-list">
          {groups.size === 0 && (
            <div className="picker-empty">
              {connected.size === 0
                ? 'No connected providers yet.'
                : 'No models match your search.'}
            </div>
          )}
          {Array.from(groups.entries()).map(([pid, ms]) => (
            <div key={pid} className="picker-group">
              <div className="picker-group-title">{providerName(pid)}</div>
              {ms.map(m => (
                <button
                  key={m.id}
                  className={`picker-item${current === m.id ? ' active' : ''}`}
                  onClick={() => pick(m.id)}
                >
                  <span className="picker-item-name">{m.name}</span>
                  <span className="picker-item-meta">
                    {m.supports_tools && <span className="picker-tag">tools</span>}
                    {m.is_free && <span className="picker-tag free">free</span>}
                    {fmtCtx(m.context_window) && <span className="picker-ctx">{fmtCtx(m.context_window)}</span>}
                    {current === m.id && <span className="picker-check">●</span>}
                  </span>
                </button>
              ))}
            </div>
          ))}

          {unconnected.length > 0 && (
            <div className="picker-group">
              <div className="picker-group-title">Not connected</div>
              {unconnected.map(p => (
                <div key={p.id} className="picker-item disabled">
                  <span className="picker-item-name">{p.name}</span>
                  <button className="picker-connect" onClick={openSettingsProviders}>Connect</button>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  )
}
