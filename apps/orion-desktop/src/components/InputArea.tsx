import { useEffect, useRef, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen, UnlistenFn } from '@tauri-apps/api/event'
import type { ChatMessage } from './MessageList'

interface Props {
  sessionId: string
  disabled?: boolean
  onUserMessage: (msg: ChatMessage) => void
  onAssistantStart: (msg: ChatMessage) => void
  onToken: (token: string) => void
  onAssistantEnd: (fullText: string) => void
  onError: (message: string) => void
  onSendingChange: (sending: boolean) => void
}

type BuildMode = 'build' | 'plan'

function genId(): string {
  return (Date.now().toString(36) + Math.random().toString(36).slice(2, 8))
}

export default function InputArea({
  sessionId,
  disabled,
  onUserMessage,
  onAssistantStart,
  onToken,
  onAssistantEnd,
  onError,
  onSendingChange,
}: Props) {
  const [input, setInput] = useState('')
  const [mode, setMode] = useState<BuildMode>('build')
  const [sending, setSending] = useState(false)
  const taRef = useRef<HTMLTextAreaElement>(null)

  useEffect(() => {
    onSendingChange(sending)
  }, [sending, onSendingChange])

  useEffect(() => {
    const unlistens: UnlistenFn[] = []
    let mounted = true

    listen<string>('orion://token', e => {
      onToken(e.payload)
    }).then(fn => {
      if (mounted) unlistens.push(fn)
      else fn()
    })

    listen<string>('orion://error', e => {
      onError(e.payload)
      setSending(false)
    }).then(fn => {
      if (mounted) unlistens.push(fn)
      else fn()
    })

    return () => {
      mounted = false
      unlistens.forEach(fn => fn())
    }
  }, [onToken, onError])

  const autoResize = () => {
    const ta = taRef.current
    if (!ta) return
    ta.style.height = 'auto'
    ta.style.height = Math.min(ta.scrollHeight, 160) + 'px'
  }

  const canSend = !!input.trim() && !sending && !disabled

  const send = async () => {
    if (!canSend) return
    const text = input.trim()
    setInput('')
    autoResize()
    setSending(true)

    const userMsg: ChatMessage = {
      id: genId(),
      role: 'user',
      content: text,
      timestamp: new Date().toISOString(),
    }
    onUserMessage(userMsg)

    const assistantMsg: ChatMessage = {
      id: genId(),
      role: 'assistant',
      content: '',
      timestamp: new Date().toISOString(),
      isStreaming: true,
    }
    onAssistantStart(assistantMsg)

    try {
      const full = await invoke<string>('send_message', {
        sessionId,
        content: text,
        mode,
      })
      onAssistantEnd(full)
    } catch (e) {
      onError(String(e))
    } finally {
      setSending(false)
    }
  }

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      if (canSend) send()
    }
  }

  return (
    <div className="input-area">
      <div className="input-area-inner">
        <div className="input-box">
          <textarea
            ref={taRef}
            value={input}
            onChange={e => {
              setInput(e.target.value)
              autoResize()
            }}
            onKeyDown={handleKeyDown}
            placeholder={disabled ? 'Configura un provider primero...' : 'Escribe un mensaje o / para comandos...'}
            disabled={disabled || sending}
            rows={1}
          />
          <button
            className="send-btn"
            disabled={!canSend}
            onClick={send}
            aria-label="Enviar"
            title="Enviar (Enter)"
          >
            ↑
          </button>
        </div>
        <div className="input-controls">
          <select
            className="mode-selector"
            value={mode}
            onChange={e => setMode(e.target.value as BuildMode)}
            disabled={sending}
          >
            <option value="build">Build</option>
            <option value="plan">Plan</option>
          </select>
          <button className="add-context-btn" disabled>+ Contexto</button>
          <span className="session-info">{sessionId.slice(0, 8)}</span>
        </div>
      </div>
    </div>
  )
}
