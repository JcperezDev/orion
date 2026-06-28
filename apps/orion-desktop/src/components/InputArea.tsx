import { useEffect, useRef, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { ChatMessage } from './MessageList'

export type BuildMode = 'build' | 'plan' | 'agent'

export interface SubmitPayload {
  text: string
  mode: BuildMode
  isCommand: boolean
}

interface Props {
  sessionId: string
  disabled?: boolean
  isStreaming?: boolean
  onSubmit: (payload: SubmitPayload) => void | Promise<void>
  onUserMessage?: (msg: ChatMessage) => void
  onSendingChange?: (sending: boolean) => void
}

function genId(): string {
  return (Date.now().toString(36) + Math.random().toString(36).slice(2, 8))
}

const MODES: { id: BuildMode; label: string; desc: string }[] = [
  { id: 'build', label: 'Build', desc: 'Respuesta directa, sin herramientas' },
  { id: 'plan', label: 'Plan', desc: 'Solo lectura, sin ediciones' },
  { id: 'agent', label: 'Agent', desc: 'ORION usa herramientas (read, write, bash, MCP)' },
]

export default function InputArea({
  sessionId,
  disabled,
  isStreaming,
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

  const canSend = !!input.trim() && !sending && !disabled && !isStreaming

  const submit = async () => {
    if (!canSend) return
    const text = input.trim()
    const isCommand = text.startsWith('/')
    setInput('')
    autoResize()
    setShowCommandHints(false)

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

  const handleStop = async () => {
    try {
      await invoke('cancel_generation')
    } catch (e) {
      console.error('Failed to cancel:', e)
    }
  }

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      if (isStreaming) {
        handleStop()
      } else if (canSend) {
        submit()
      }
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
          {isStreaming ? (
            <button
              className="stop-btn"
              onClick={handleStop}
              aria-label="Detener"
              title="Detener (Enter)"
            >
              ■
            </button>
          ) : (
            <button
              className="send-btn"
              disabled={!canSend}
              onClick={submit}
              aria-label="Enviar"
              title="Enviar (Enter)"
            >
              ↑
            </button>
          )}

          {showCommandHints && (
            <div className="command-hints">
              {[
                { id: '/clear',     label: 'Limpiar chat', desc: 'Borra todos los mensajes' },
                { id: '/help',      label: 'Ayuda',        desc: 'Muestra los comandos disponibles' },
                { id: '/providers', label: 'Ver providers', desc: 'Lista providers conectados' },
                { id: '/model',     label: 'Modelo actual', desc: 'Muestra el modelo y los disponibles' },
                { id: '/sync',      label: 'Sincronizar',  desc: 'Sincroniza modelos del provider activo' },
              ].map(c => (
                <div
                  key={c.id}
                  className="command-hint-item"
                  onMouseDown={(e) => {
                    e.preventDefault()
                    setInput(c.id + ' ')
                    setShowCommandHints(false)
                    taRef.current?.focus()
                  }}
                >
                  <span className="command-hint-key">{c.id}</span>
                  <span className="command-hint-label">{c.label}</span>
                  <span className="command-hint-desc">{c.desc}</span>
                </div>
              ))}
            </div>
          )}
        </div>
        <div className="input-controls">
          <div className="mode-pills">
            {MODES.map(m => (
              <button
                key={m.id}
                className={`mode-pill${mode === m.id ? ' active' : ''}`}
                onClick={() => setMode(m.id)}
                disabled={sending || isStreaming}
                title={m.desc}
              >
                {m.label}
              </button>
            ))}
          </div>
          <button className="add-context-btn" disabled>+ Contexto</button>
          <span className="session-info">{sessionId.slice(0, 8)}</span>
        </div>
      </div>
    </div>
  )
}
