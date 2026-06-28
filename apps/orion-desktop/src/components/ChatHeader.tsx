import { useEffect, useRef, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

interface ModelInfo {
  id: string
  provider: string
  name: string
  context_window?: number
  supports_vision: boolean
  supports_tools: boolean
  is_free: boolean
  is_local: boolean
  is_available: boolean
}

interface ActiveConfig {
  provider_name: string
  model_id: string
  model_name: string
}

interface Props {
  sessionTitle: string
  sessionId: string
  onTitleChange: (newTitle: string) => void
  totalTokens: number
  tokenLimit: number
  onModelChange?: () => void
}

export default function ChatHeader({ sessionTitle, sessionId, onTitleChange, totalTokens, tokenLimit, onModelChange }: Props) {
  const [editing, setEditing] = useState(false)
  const [title, setTitle] = useState(sessionTitle)
  const [active, setActive] = useState<ActiveConfig | null>(null)
  const [models, setModels] = useState<ModelInfo[]>([])
  const [open, setOpen] = useState(false)
  const wrapRef = useRef<HTMLDivElement>(null)
  const inputRef = useRef<HTMLInputElement>(null)

  useEffect(() => {
    setTitle(sessionTitle)
  }, [sessionTitle])

  useEffect(() => {
    if (editing) inputRef.current?.select()
  }, [editing])

  async function refreshActive() {
    try {
      const modelId = await invoke<string | null>('get_default_model')
      if (!modelId) {
        setActive(null)
        return
      }
      const [providerId] = modelId.split(':')
      const providerList = await invoke<Array<{ id: string; name: string }>>('list_providers')
      const provider = providerList.find(p => p.id === providerId)
      const modelList = await invoke<ModelInfo[]>('list_models', { provider: providerId })
      // `x.id` is already the full `provider:model` id.
      const m = modelList.find(x => x.id === modelId)
      setActive({
        provider_name: provider?.name ?? providerId ?? '',
        model_id: modelId,
        model_name: m?.name ?? modelId.split(':')[1] ?? modelId,
      })
      setModels(modelList)
    } catch (e) {
      console.error('Failed to load active model:', e)
    }
  }

  useEffect(() => {
    refreshActive()
    const onClick = (e: MouseEvent) => {
      if (wrapRef.current && !wrapRef.current.contains(e.target as Node)) setOpen(false)
    }
    document.addEventListener('mousedown', onClick)
    // The backend may auto-select a model on first message — refresh the badge.
    const unlisten = listen('orion://model_changed', () => {
      refreshActive()
      onModelChange?.()
    })
    return () => {
      document.removeEventListener('mousedown', onClick)
      unlisten.then(fn => fn())
    }
  }, [])

  async function commitTitle() {
    const trimmed = title.trim()
    if (trimmed && trimmed !== sessionTitle) {
      try {
        await invoke('rename_session', { id: sessionId, title: trimmed })
        onTitleChange(trimmed)
      } catch (e) {
        console.error('Rename failed:', e)
        setTitle(sessionTitle)
      }
    } else {
      setTitle(sessionTitle)
    }
    setEditing(false)
  }

  async function selectModel(modelId: string) {
    try {
      await invoke('set_active_model', { modelId })
      setOpen(false)
      await refreshActive()
      onModelChange?.()
    } catch (e) {
      console.error('Set model failed:', e)
    }
  }

  const pct = tokenLimit > 0 ? Math.min(100, Math.round((totalTokens / tokenLimit) * 100)) : 0
  const tokenClass = pct >= 90 ? 'danger' : pct >= 75 ? 'warn' : ''
  const fmtTokens = (n: number) => {
    if (n >= 1000) return `${(n / 1000).toFixed(1)}K`
    return n.toString()
  }

  return (
    <div className="chat-header">
      <div className="chat-header-left">
        {editing ? (
          <input
            ref={inputRef}
            className="session-title editing"
            value={title}
            onChange={e => setTitle(e.target.value)}
            onBlur={commitTitle}
            onKeyDown={e => {
              if (e.key === 'Enter') {
                e.preventDefault()
                ;(e.target as HTMLInputElement).blur()
              } else if (e.key === 'Escape') {
                setTitle(sessionTitle)
                setEditing(false)
              }
            }}
            autoFocus
          />
        ) : (
          <span
            className="session-title"
            onDoubleClick={() => setEditing(true)}
            title="Doble click para renombrar"
          >
            {sessionTitle || 'New session'}
          </span>
        )}
      </div>

      <div className="chat-header-right">
        <div ref={wrapRef} style={{ position: 'relative' }}>
          <button
            className={`model-badge${active ? '' : ' model-badge-empty'}`}
            onClick={() => setOpen(o => !o)}
            title={active ? `${active.provider_name} · ${active.model_name}` : 'No model selected'}
          >
            {active
              ? `${active.provider_name} · ${active.model_name} ▾`
              : 'Select model ▾'}
          </button>
          {open && (
            <div className="model-dropdown">
              {models.length === 0 && (
                <div className="model-dropdown-empty">No models. Connect a provider.</div>
              )}
              {models.map(m => {
                const fullId = `${m.provider}:${m.id}`
                return (
                  <div
                    key={fullId}
                    className={`model-dropdown-item${fullId === active?.model_id ? ' active' : ''}`}
                    onClick={() => selectModel(fullId)}
                  >
                    <span className="model-dropdown-item-name">{m.name || m.id}</span>
                    <span className="model-dropdown-provider">{m.provider}</span>
                  </div>
                )
              })}
            </div>
          )}
        </div>

        <span className={`token-badge ${tokenClass}`}>
          {fmtTokens(totalTokens)}{tokenLimit > 0 ? ` · ${pct}%` : ''}
        </span>
      </div>
    </div>
  )
}
