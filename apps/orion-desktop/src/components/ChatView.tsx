import { useCallback, useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import ChatHeader from './ChatHeader'
import MessageList, { ChatMessage } from './MessageList'
import InputArea, { type SubmitPayload } from './InputArea'

interface Session {
  id: string
  title: string
  created_at: string
  updated_at: string
  message_count: number
  active_model: string | null
}

interface ModelInfo {
  id: string
  provider: string
  name: string
  context_window?: number
}

function genId(): string {
  return (Date.now().toString(36) + Math.random().toString(36).slice(2, 8))
}

function nowIso() {
  return new Date().toISOString()
}

export default function ChatView() {
  const [activeSession, setActiveSession] = useState<Session | null>(null)
  const [messages, setMessages] = useState<ChatMessage[]>([])
  const [totalTokens, setTotalTokens] = useState(0)
  const [contextWindow, setContextWindow] = useState<number>(0)

  // Load active session on mount
  useEffect(() => {
    invoke<Session | null>('get_active_session')
      .then(s => { if (s) setActiveSession(s) })
      .catch(e => console.error('Failed to load active session:', e))
  }, [])

  // Load active model + context_window whenever session becomes available or model changes
  useEffect(() => {
    if (!activeSession) return
    refreshContextWindow()
  }, [activeSession?.id, activeSession?.active_model])

  // Listen to streaming + error events from the backend
  useEffect(() => {
    const unlistens: UnlistenFn[] = []
    let mounted = true

    listen<string>('orion://token', e => {
      appendToken(e.payload)
    }).then(fn => { if (mounted) unlistens.push(fn); else fn() })

    listen<string>('orion://error', e => {
      pushError(e.payload)
      markStreamEnd()
    }).then(fn => { if (mounted) unlistens.push(fn); else fn() })

    listen<void>('orion://done', () => {
      markStreamEnd()
    }).then(fn => { if (mounted) unlistens.push(fn); else fn() })

    return () => {
      mounted = false
      unlistens.forEach(fn => fn())
    }
  }, [])

  async function refreshContextWindow() {
    try {
      const modelId = await invoke<string | null>('get_default_model')
      if (!modelId) {
        setContextWindow(0)
        return
      }
      const [providerId] = modelId.split(':')
      const models = await invoke<ModelInfo[]>('list_models', { provider: providerId })
      const m = models.find(x => `${x.provider}:${x.id}` === modelId)
      setContextWindow(m?.context_window ?? 0)
    } catch (e) {
      console.error('Failed to load context window:', e)
    }
  }

  // --- message helpers ---
  const pushMessage = useCallback((msg: ChatMessage) => {
    setMessages(prev => [...prev, msg])
  }, [])

  const pushSystem = useCallback((content: string) => {
    pushMessage({ id: genId(), role: 'system', content, timestamp: nowIso() })
  }, [pushMessage])

  const pushError = useCallback((content: string) => {
    pushMessage({ id: genId(), role: 'error', content, timestamp: nowIso() })
  }, [pushMessage])

  const appendToken = useCallback((token: string) => {
    setMessages(prev => {
      if (prev.length === 0) return prev
      const next = prev.slice()
      const last = next[next.length - 1]
      if (last.role === 'assistant' && last.isStreaming) {
        next[next.length - 1] = { ...last, content: last.content + token }
      } else {
        next.push({
          id: genId(), role: 'assistant', content: token,
          timestamp: nowIso(), isStreaming: true,
        })
      }
      return next
    })
    setTotalTokens(t => t + Math.ceil(token.length / 4))
  }, [])

  const markStreamEnd = useCallback(() => {
    setMessages(prev => {
      if (prev.length === 0) return prev
      const last = prev[prev.length - 1]
      if (last.role === 'assistant' && last.isStreaming) {
        const next = prev.slice()
        next[next.length - 1] = { ...last, isStreaming: false }
        return next
      }
      return prev
    })
  }, [])

  // --- slash command handlers ---
  async function handleSlashCommand(cmd: string) {
    const command = cmd.trim().split(/\s+/)[0]
    switch (command) {
      case '/clear':
        setMessages([])
        setTotalTokens(0)
        pushSystem('Chat limpiado.')
        break

      case '/help':
        pushSystem([
          'Comandos disponibles:',
          '  /clear       — borra todos los mensajes',
          '  /help        — muestra esta ayuda',
          '  /providers   — lista providers conectados',
          '  /model       — muestra el modelo activo',
          '  /sync        — sincroniza modelos del provider activo',
        ].join('\n'))
        break

      case '/providers':
        try {
          const providers = await invoke<Array<{ id: string; name: string; has_api_key: boolean; models_count: number }>>('get_connected_providers')
          const lines = providers.map(p =>
            `  ${p.has_api_key ? '●' : '○'} ${p.name.padEnd(20)} ${p.models_count} modelos`
          )
          pushSystem(`Providers (${providers.length}):\n${lines.join('\n') || '  (ninguno)'}`)
        } catch (e) {
          pushError(String(e))
        }
        break

      case '/model': {
        try {
          const [modelId, models] = await Promise.all([
            invoke<string | null>('get_default_model'),
            invoke<Array<{ provider: string; id: string; name: string; context_window?: number }>>('list_models', { provider: null }),
          ])
          const list = models.slice(0, 8).map(m =>
            `  ${m.provider}:${m.id.padEnd(28)} ${m.context_window ? m.context_window.toLocaleString() + ' ctx' : ''}`
          ).join('\n')
          pushSystem(`Modelo actual: ${modelId ?? 'ninguno'}\nModelos disponibles (${models.length}, mostrando 8):\n${list}`)
        } catch (e) {
          pushError(String(e))
        }
        break
      }

      case '/sync': {
        try {
          const modelId = await invoke<string | null>('get_default_model')
          const targetProvider = modelId?.split(':')[0] ?? 'openrouter'
          pushSystem(`Sincronizando ${targetProvider}...`)
          await invoke('sync_provider_models', { providerId: targetProvider })
          pushSystem('✓ Modelos sincronizados.')
        } catch (e) {
          pushError(`Sync falló: ${e}`)
        }
        break
      }

      default:
        pushSystem(`Comando desconocido: ${command}. Escribe /help.`)
    }
  }

  // --- main submit handler ---
  const handleSubmit = useCallback(async (payload: SubmitPayload) => {
    if (!activeSession) return
    const { text, isCommand } = payload

    if (isCommand) {
      await handleSlashCommand(text)
      return
    }

    // Regular LLM message — echo user + create empty assistant bubble for streaming
    pushMessage({
      id: genId(), role: 'user', content: text, timestamp: nowIso(),
    })
    pushMessage({
      id: genId(), role: 'assistant', content: '', timestamp: nowIso(), isStreaming: true,
    })

    try {
      await invoke<string>('send_message', {
        sessionId: activeSession.id,
        content: text,
        mode: payload.mode,
      })
    } catch (e) {
      pushError(String(e))
      markStreamEnd()
    }
  }, [activeSession, markStreamEnd, pushError, pushMessage])

  const handleTitleChange = useCallback((newTitle: string) => {
    setActiveSession(prev => (prev ? { ...prev, title: newTitle } : prev))
  }, [])

  return (
    <div className="chat-shell">
      {activeSession ? (
        <ChatHeader
          sessionTitle={activeSession.title}
          sessionId={activeSession.id}
          onTitleChange={handleTitleChange}
          totalTokens={totalTokens}
          tokenLimit={contextWindow}
          onModelChange={refreshContextWindow}
        />
      ) : (
        <div className="chat-header">
          <div className="chat-header-left">
            <span className="session-title" style={{ cursor: 'default' }}>Cargando sesión…</span>
          </div>
        </div>
      )}
      <MessageList messages={messages} />
      <InputArea
        sessionId={activeSession?.id ?? ''}
        disabled={!activeSession}
        onSubmit={handleSubmit}
        onUserMessage={pushMessage}
        onSendingChange={() => {}}
      />
    </div>
  )
}