import { useCallback, useEffect, useRef, useState } from 'react'
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

interface ToolCallEvent {
  id: string
  name: string
  arguments: unknown
}

interface ToolResultEvent {
  tool_call_id: string
  content: string
  is_error: boolean
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
  const [undoable, setUndoable] = useState<{ id: string; summary: string; paths: string[] } | null>(null)
  const [limitInfo, setLimitInfo] = useState<{ message: string; retryAfter: number | null } | null>(null)
  const [retryInfo, setRetryInfo] = useState<{ attempt: number; delaySecs: number } | null>(null)
  const lastSubmitRef = useRef<{ text: string; mode?: string } | null>(null)
  // Mirror of `messages` so handlers can read the latest history without being
  // re-created on every message.
  const messagesRef = useRef<ChatMessage[]>([])
  useEffect(() => { messagesRef.current = messages }, [messages])

  // Prior user/assistant turns (with text) for multi-turn context.
  const buildHistory = () =>
    messagesRef.current
      .filter(m => (m.role === 'user' || m.role === 'assistant') && !!m.content && !m.isStreaming)
      .map(m => ({ role: m.role, content: m.content }))
      .slice(-40) // cap to recent turns to avoid context overflow

  // Load active session on mount
  useEffect(() => {
    invoke<Session | null>('get_active_session')
      .then(s => { if (s) setActiveSession(s) })
      .catch(e => console.error('Failed to load active session:', e))
  }, [])

  // Load persisted messages whenever the active session changes.
  useEffect(() => {
    if (!activeSession) { setMessages([]); return }
    invoke<Array<{ role: string; content: string; created_at: string }>>('get_messages', { sessionId: activeSession.id })
      .then(rows => {
        setMessages(rows.map(m => ({
          id: genId(),
          role: m.role as ChatMessage['role'],
          content: m.content,
          timestamp: m.created_at,
        })))
      })
      .catch(() => {})
  }, [activeSession?.id])

  // React to session switches coming from the Sidebar.
  useEffect(() => {
    const onSessionChange = (e: Event) => {
      const id = (e as CustomEvent).detail as string | null
      if (!id) { setActiveSession(null); return }
      invoke<Session | null>('get_active_session')
        .then(s => { if (s) setActiveSession(s) })
        .catch(() => {})
    }
    window.addEventListener('orion:session', onSessionChange)
    return () => window.removeEventListener('orion:session', onSessionChange)
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
      setRetryInfo(null)
      appendToken(e.payload)
    }).then(fn => { if (mounted) unlistens.push(fn); else fn() })

    listen<ToolCallEvent>('orion://tool_call', e => {
      pushToolCall(e.payload)
    }).then(fn => { if (mounted) unlistens.push(fn); else fn() })

    listen<ToolResultEvent>('orion://tool_result', e => {
      completeToolCall(e.payload)
    }).then(fn => { if (mounted) unlistens.push(fn); else fn() })

    listen<string>('orion://error', e => {
      pushError(e.payload)
      markStreamEnd()
    }).then(fn => { if (mounted) unlistens.push(fn); else fn() })

    listen<void>('orion://done', () => {
      setRetryInfo(null)
      markStreamEnd()
    }).then(fn => { if (mounted) unlistens.push(fn); else fn() })

    listen<{ tool_call_id: string; summary: string; paths: string[] }>('orion://undoable', e => {
      setUndoable({ id: e.payload.tool_call_id, summary: e.payload.summary, paths: e.payload.paths })
    }).then(fn => { if (mounted) unlistens.push(fn); else fn() })

    listen<{ attempt: number; delay_secs: number; reason: string }>('orion://retrying', e => {
      setRetryInfo({ attempt: e.payload.attempt, delaySecs: e.payload.delay_secs })
    }).then(fn => { if (mounted) unlistens.push(fn); else fn() })

    listen<{ retry_after_secs: number | null; message: string }>('orion://limit_reached', e => {
      setRetryInfo(null)
      setLimitInfo({ message: e.payload.message, retryAfter: e.payload.retry_after_secs })
      markStreamEnd()
    }).then(fn => { if (mounted) unlistens.push(fn); else fn() })

    return () => {
      mounted = false
      unlistens.forEach(fn => fn())
    }
  }, [])

  const handleUndo = useCallback(async () => {
    if (!undoable) return
    try {
      await invoke<string[]>('undo_changes', { toolCallId: undoable.id })
    } catch (err) {
      console.error('undo failed:', err)
    } finally {
      setUndoable(null)
    }
  }, [undoable])

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

  const pushToolCall = useCallback((ev: ToolCallEvent) => {
    const input = typeof ev.arguments === 'string'
      ? ev.arguments
      : JSON.stringify(ev.arguments, null, 2)
    pushMessage({
      id: genId(),
      role: 'tool_call',
      timestamp: nowIso(),
      tool: { id: ev.id, name: ev.name, input, status: 'running' },
    })
  }, [pushMessage])

  const completeToolCall = useCallback((ev: ToolResultEvent) => {
    setMessages(prev => {
      // Update the most recent running tool_call that has a matching id (or no id yet).
      for (let i = prev.length - 1; i >= 0; i--) {
        const m = prev[i]
        if (m.role === 'tool_call' && m.tool && m.tool.status === 'running') {
          if (!m.tool.id || m.tool.id === ev.tool_call_id) {
            const next = prev.slice()
            next[i] = {
              ...m,
              tool: {
                ...m.tool,
                status: ev.is_error ? 'error' : 'done',
                output: ev.content,
                isError: ev.is_error,
              },
            }
            return next
          }
        }
      }
      // Fallback: append a new tool_call message with the result.
      return [
        ...prev,
        {
          id: genId(),
          role: 'tool_call',
          timestamp: nowIso(),
          tool: {
            id: ev.tool_call_id,
            name: 'tool',
            input: '',
            output: ev.content,
            status: ev.is_error ? 'error' : 'done',
            isError: ev.is_error,
          },
        },
      ]
    })
  }, [])

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
    // Clear the streaming flag on ANY in-flight assistant bubble, not just the
    // last message — an error/system message may have been appended after it,
    // which would otherwise leave the input disabled forever.
    setMessages(prev => {
      if (!prev.some(m => m.isStreaming)) return prev
      return prev
        // Drop placeholder assistant bubbles that never received any content.
        .filter(m => !(m.role === 'assistant' && m.isStreaming && !m.content))
        .map(m => (m.isStreaming ? { ...m, isStreaming: false } : m))
    })
  }, [])

  // --- slash command handlers ---
  async function handleSlashCommand(cmd: string) {
    const command = cmd.trim().split(/\s+/)[0]
    switch (command) {
      case '/clear':
        setMessages([])
        setTotalTokens(0)
        pushSystem('Chat cleared.')
        break

      case '/help':
        pushSystem([
          'Available commands:',
          '  /clear       — clear all messages',
          '  /help        — show this help',
          '  /providers   — list connected providers',
          '  /model       — show the active model',
          '  /sync        — sync models for the active provider',
        ].join('\n'))
        break

      case '/providers':
        try {
          const providers = await invoke<Array<{ id: string; name: string; has_api_key: boolean; models_count: number }>>('get_connected_providers')
          const lines = providers.map(p =>
            `  ${p.has_api_key ? '●' : '○'} ${p.name.padEnd(20)} ${p.models_count} models`
          )
          pushSystem(`Providers (${providers.length}):\n${lines.join('\n') || '  (none)'}`)
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
          pushSystem(`Active model: ${modelId ?? 'none'}\nAvailable models (${models.length}, showing 8):\n${list}`)
        } catch (e) {
          pushError(String(e))
        }
        break
      }

      case '/sync': {
        try {
          const modelId = await invoke<string | null>('get_default_model')
          const targetProvider = modelId?.split(':')[0] ?? 'openrouter'
          pushSystem(`Syncing ${targetProvider}...`)
          await invoke('sync_provider_models', { providerId: targetProvider })
          pushSystem('✓ Models synced.')
        } catch (e) {
          pushError(`Sync failed: ${e}`)
        }
        break
      }

      default:
        pushSystem(`Unknown command: ${command}. Type /help.`)
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

    // Capture prior conversation BEFORE pushing the new bubbles.
    const history = buildHistory()
    // Regular LLM message — echo user + create empty assistant bubble for streaming
    setLimitInfo(null)
    lastSubmitRef.current = { text, mode: payload.mode }
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
        history,
      })
    } catch (e) {
      pushError(String(e))
      markStreamEnd()
    }
  }, [activeSession, markStreamEnd, pushError, pushMessage])

  // Resume the checkpointed work after a usage limit: re-run the last prompt.
  const handleResume = useCallback(async () => {
    const last = lastSubmitRef.current
    if (!activeSession || !last) return
    // History up to the failed turn — drop the trailing user message we resend.
    const history = buildHistory()
    if (history.length && history[history.length - 1].role === 'user' && history[history.length - 1].content === last.text) {
      history.pop()
    }
    setLimitInfo(null)
    pushMessage({
      id: genId(), role: 'assistant', content: '', timestamp: nowIso(), isStreaming: true,
    })
    try {
      await invoke<string>('send_message', {
        sessionId: activeSession.id,
        content: last.text,
        mode: last.mode,
        history,
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
            <span className="session-title" style={{ cursor: 'default' }}>Loading session…</span>
          </div>
        </div>
      )}
      <MessageList messages={messages} />
      {undoable && (
        <div className="undo-bar">
          <span className="undo-bar-text">
            Edited <code>{undoable.paths.join(', ') || undoable.summary}</code>
          </span>
          <button className="undo-bar-btn" onClick={handleUndo}>Undo</button>
          <button className="undo-bar-dismiss" onClick={() => setUndoable(null)} aria-label="Dismiss">✕</button>
        </div>
      )}
      {retryInfo && !limitInfo && (
        <div className="limit-bar retry">
          <span className="limit-bar-text">
            ⏳ Provider busy — retrying (attempt {retryInfo.attempt}) in {retryInfo.delaySecs}s…
          </span>
        </div>
      )}
      {limitInfo && (
        <div className="limit-bar">
          <span className="limit-bar-text">
            🛑 Usage limit reached.
            {limitInfo.retryAfter != null ? ` Resets in ~${limitInfo.retryAfter}s.` : ''}{' '}
            {limitInfo.message || ''}
            {limitInfo.retryAfter == null
              ? ' Switch to another model in the top bar, or add credits to this provider.'
              : ' Your work is saved — resume when it resets.'}
          </span>
          {/* Resume only helps for a temporary limit that resets; for a hard
              quota/credit limit it would just hit the same error again. */}
          {limitInfo.retryAfter != null && (
            <button className="limit-bar-btn" onClick={handleResume}>Resume</button>
          )}
          <button className="undo-bar-dismiss" onClick={() => setLimitInfo(null)} aria-label="Dismiss">✕</button>
        </div>
      )}
      <InputArea
        sessionId={activeSession?.id ?? ''}
        disabled={!activeSession}
        isStreaming={messages.some(m => m.isStreaming)}
        onSubmit={handleSubmit}
        onUserMessage={pushMessage}
        onSendingChange={() => {}}
      />
    </div>
  )
}