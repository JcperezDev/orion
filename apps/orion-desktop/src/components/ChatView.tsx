import { useCallback, useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import ChatHeader from './ChatHeader'
import MessageList, { ChatMessage } from './MessageList'
import InputArea from './InputArea'

interface Session {
  id: string
  title: string
  created_at: string
  updated_at: string
  message_count: number
  active_model: string | null
}

function genId(): string {
  return (Date.now().toString(36) + Math.random().toString(36).slice(2, 8))
}

export default function ChatView() {
  const [activeSession, setActiveSession] = useState<Session | null>(null)
  const [messages, setMessages] = useState<ChatMessage[]>([])
  const [totalTokens, setTotalTokens] = useState(0)
  const [tokenLimit] = useState(0) // 0 = no limit shown

  useEffect(() => {
    invoke<Session | null>('get_active_session')
      .then(s => {
        if (s) setActiveSession(s)
      })
      .catch(e => console.error('Failed to load active session:', e))
  }, [])

  const handleUserMessage = useCallback((msg: ChatMessage) => {
    setMessages(prev => [...prev, msg])
  }, [])

  const handleAssistantStart = useCallback((msg: ChatMessage) => {
    setMessages(prev => [...prev, msg])
  }, [])

  const handleToken = useCallback((token: string) => {
    setMessages(prev => {
      if (prev.length === 0) return prev
      const next = prev.slice()
      const last = next[next.length - 1]
      if (last.role === 'assistant' && last.isStreaming) {
        next[next.length - 1] = { ...last, content: last.content + token }
      } else {
        next.push({
          id: genId(),
          role: 'assistant',
          content: token,
          timestamp: new Date().toISOString(),
          isStreaming: true,
        })
      }
      return next
    })
    setTotalTokens(t => t + Math.ceil(token.length / 4))
  }, [])

  const handleAssistantEnd = useCallback((_fullText: string) => {
    setMessages(prev => {
      if (prev.length === 0) return prev
      const next = prev.slice()
      const last = next[next.length - 1]
      if (last.role === 'assistant' && last.isStreaming) {
        next[next.length - 1] = { ...last, isStreaming: false }
      }
      return next
    })
  }, [])

  const handleError = useCallback((message: string) => {
    setMessages(prev => [
      ...prev,
      {
        id: genId(),
        role: 'error',
        content: message,
        timestamp: new Date().toISOString(),
      },
    ])
  }, [])

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
          tokenLimit={tokenLimit}
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
        onUserMessage={handleUserMessage}
        onAssistantStart={handleAssistantStart}
        onToken={handleToken}
        onAssistantEnd={handleAssistantEnd}
        onError={handleError}
        onSendingChange={() => {}}
      />
    </div>
  )
}
