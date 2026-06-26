import { useEffect, useRef, useState } from 'react'
import type { ChatMessage } from './MessageList'

export type BuildMode = 'build' | 'plan' | 'agent'

export interface SubmitPayload {
  text: string
  mode: BuildMode
  /** True if the text starts with "/" — the parent can decide whether to handle as command. */
  isCommand: boolean
}

interface Props {
  sessionId: string
  disabled?: boolean
  onSubmit: (payload: SubmitPayload) => void | Promise<void>
  onUserMessage?: (msg: ChatMessage) => void
  onSendingChange?: (sending: boolean) => void
}

function genId(): string {
  return (Date.now().toString(36) + Math.random().toString(36).slice(2, 8))
}

export default function InputArea({
  sessionId,
  disabled,
  onSubmit,
  onUserMessage,
  onSendingChange,
}: Props) {
  const [input, setInput] = useState('')
  const [mode, setMode] = useState<BuildMode>('build')
  const [sending, setSending] = useState(false)
  const taRef = useRef<HTMLTextAreaElement>(null)
  const [showCommandHints, setShowCommandHints] = useState(false)

  useEffect(() => {
    onSendingChange?.(sending)
  }, [sending, onSendingChange])

  const autoResize = () => {
    const ta = taRef.current
    if (!ta) return
    ta.style.height = 'auto'
    ta.style.height = Math.min(ta.scrollHeight, 160) + 'px'
  }

  const canSend = !!input.trim() && !sending && !disabled

  const submit = async () => {
    if (!canSend) return
    const text = input.trim()
    const isCommand = text.startsWith('/')
    setInput('')
    autoResize()
    setShowCommandHints(false)

    // Echo the user's command/message into the chat for visual continuity.
    if (onUserMessage && isCommand) {
      onUserMessage({
        id: genId(),
        role: 'user',
        content: text,
        timestamp: new Date().toISOString(),
      })
    }

    setSending(true)
    try {
      await onSubmit({ text, mode, isCommand })
    } finally {
      setSending(false)
    }
  }

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      if (canSend) submit()
    }
  }

  const handleChange = (val: string) => {
    setInput(val)
    autoResize()
    setShowCommandHints(val === '/')
  }

  return (
    <div className="input-area">
      <div className="input-area-inner">
        <div className="input-box" style={{ position: 'relative' }}>
          <textarea
            ref={taRef}
            value={input}
            onChange={e => handleChange(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder={disabled ? 'Configura un provider primero...' : 'Escribe un mensaje o / para comandos...'}
            disabled={disabled || sending}
            rows={1}
          />
          <button
            className="send-btn"
            disabled={!canSend}
            onClick={submit}
            aria-label="Enviar"
            title="Enviar (Enter)"
          >
            ↑
          </button>

          {showCommandHints && (
            <div className="absolute bottom-full left-0 right-0 mb-1 max-h-64 overflow-y-auto rounded-lg border border-border bg-[var(--surface)] shadow-lg" style={{ background: 'var(--bg-secondary)', border: '0.5px solid var(--border-mid)', borderRadius: 8 }}>
              {[
                { id: '/clear',     label: 'Limpiar chat', desc: 'Borra todos los mensajes' },
                { id: '/help',      label: 'Ayuda',        desc: 'Muestra los comandos disponibles' },
                { id: '/providers', label: 'Ver providers', desc: 'Lista providers conectados' },
                { id: '/model',     label: 'Modelo actual', desc: 'Muestra el modelo y los disponibles' },
                { id: '/sync',      label: 'Sincronizar',  desc: 'Sincroniza modelos del provider activo' },
              ].map(c => (
                <div
                  key={c.id}
                  onMouseDown={(e) => {
                    e.preventDefault()
                    setInput(c.id + ' ')
                    setShowCommandHints(false)
                    taRef.current?.focus()
                  }}
                  className="command-hint-item"
                  style={{
                    padding: '7px 12px',
                    cursor: 'pointer',
                    display: 'flex',
                    alignItems: 'center',
                    gap: 10,
                    fontSize: 12,
                  }}
                >
                  <span style={{ color: 'var(--accent-text)', fontFamily: "'JetBrains Mono', monospace", minWidth: 80 }}>{c.id}</span>
                  <span style={{ color: 'var(--text-primary)', flex: 1 }}>{c.label}</span>
                  <span style={{ color: 'var(--text-tertiary)', fontSize: 10 }}>{c.desc}</span>
                </div>
              ))}
            </div>
          )}
        </div>
        <div className="input-controls">
          <select
            className="mode-selector"
            value={mode}
            onChange={e => setMode(e.target.value as BuildMode)}
            disabled={sending}
            title={
              mode === 'agent'
                ? 'Agent mode: ORION can use tools (read, write, bash, MCP) and may ask permission'
                : mode === 'plan'
                ? 'Plan mode: read-only research, no edits'
                : 'Build mode: direct execution, no tools'
            }
          >
            <option value="build">Build</option>
            <option value="plan">Plan</option>
            <option value="agent">Agent</option>
          </select>
          <button className="add-context-btn" disabled>+ Contexto</button>
          <span className="session-info">{sessionId.slice(0, 8)}</span>
        </div>
      </div>
    </div>
  )
}