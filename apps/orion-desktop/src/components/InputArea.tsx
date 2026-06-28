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
  { id: 'build', label: 'Build', desc: 'Direct response, no tools' },
  { id: 'plan', label: 'Plan', desc: 'Read-only, no edits' },
  { id: 'agent', label: 'Agent', desc: 'ORION uses tools (read, write, bash, MCP)' },
]

const COMMANDS: { id: string; label: string; desc: string }[] = [
  { id: '/clear',     label: 'Clear chat',    desc: 'Delete all messages' },
  { id: '/help',      label: 'Help',          desc: 'Show available commands' },
  { id: '/providers', label: 'Providers',     desc: 'List connected providers' },
  { id: '/model',     label: 'Current model', desc: 'Show the model and available ones' },
  { id: '/sync',      label: 'Sync',          desc: 'Sync models for the active provider' },
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
    // Tab autocompletes the top command suggestion.
    if (hintsVisible && e.key === 'Tab') {
      e.preventDefault()
      applyCommand(filteredCommands[0].id)
      return
    }
    if (e.key === 'Escape' && hintsVisible) {
      e.preventDefault()
      setShowCommandHints(false)
      return
    }
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      // If a partial command is typed, Enter completes it instead of sending.
      const typed = input.trim()
      if (hintsVisible && !COMMANDS.some(c => c.id === typed)) {
        applyCommand(filteredCommands[0].id)
        return
      }
      if (isStreaming) {
        handleStop()
      } else if (canSend) {
        submit()
      }
    }
  }

  // Show command hints while typing a slash command name (before any space).
  const commandQuery = input.startsWith('/') && !input.includes(' ') ? input : null
  const filteredCommands = commandQuery
    ? COMMANDS.filter(c => c.id.startsWith(commandQuery))
    : []
  const hintsVisible = showCommandHints && filteredCommands.length > 0

  const handleChange = (val: string) => {
    setInput(val)
    autoResize()
    setShowCommandHints(val.startsWith('/') && !val.includes(' '))
  }

  const applyCommand = (id: string) => {
    setInput(id + ' ')
    setShowCommandHints(false)
    taRef.current?.focus()
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
            placeholder={disabled ? 'Connect a provider first...' : 'Type a message or / for commands...'}
            disabled={disabled || sending}
            rows={1}
          />
          {isStreaming ? (
            <button
              className="stop-btn"
              onClick={handleStop}
              aria-label="Stop"
              title="Stop"
            >
              ■
            </button>
          ) : (
            <button
              className="send-btn"
              disabled={!canSend}
              onClick={submit}
              aria-label="Send"
              title="Send (Enter)"
            >
              ↑
            </button>
          )}

          {hintsVisible && (
            <div className="command-hints">
              {filteredCommands.map(c => (
                <div
                  key={c.id}
                  className="command-hint-item"
                  onMouseDown={(e) => {
                    e.preventDefault()
                    applyCommand(c.id)
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
          <button className="add-context-btn" disabled>+ Context</button>
          <span className="session-info">{sessionId.slice(0, 8)}</span>
        </div>
      </div>
    </div>
  )
}
