import { useEffect, useMemo, useRef, useState } from 'react'

export type MessageRole = 'system' | 'assistant' | 'user' | 'tool_call' | 'error'

export interface ToolInfo {
  id?: string
  name: string
  input: string
  output?: string
  status: 'running' | 'done' | 'error'
  isError?: boolean
}

export interface ChatMessage {
  id: string
  role: MessageRole
  content?: string
  timestamp: string
  model?: string
  tool?: ToolInfo
  isStreaming?: boolean
}

interface Props {
  messages: ChatMessage[]
}

function formatTime(iso: string): string {
  try {
    const d = new Date(iso)
    if (isNaN(d.getTime())) return ''
    return d.toLocaleTimeString('es', { hour: '2-digit', minute: '2-digit' })
  } catch {
    return ''
  }
}

/**
 * Lightweight markdown-like parser: extracts ```lang\n...\n``` code blocks and
 * renders the rest as plain text. Inline code (`...`) gets a styled span.
 * No external highlighter — keeps bundle small.
 */
function renderContent(content: string, isStreaming: boolean, id: string): React.ReactNode {
  const parts: React.ReactNode[] = []
  const re = /```(\w+)?\n([\s\S]*?)```/g
  let lastIndex = 0
  let match: RegExpExecArray | null
  let blockIdx = 0

  while ((match = re.exec(content)) !== null) {
    if (match.index > lastIndex) {
      parts.push(renderInline(content.slice(lastIndex, match.index), `${id}-text-${blockIdx}`))
    }
    const lang = (match[1] || 'text').trim()
    const code = match[2]
    parts.push(<CodeBlock key={`${id}-code-${blockIdx}`} lang={lang} code={code} />)
    lastIndex = re.lastIndex
    blockIdx++
  }

  if (lastIndex < content.length) {
    parts.push(renderInline(content.slice(lastIndex), `${id}-tail`))
  }

  if (isStreaming && parts.length > 0) {
    const last = parts[parts.length - 1]
    parts[parts.length - 1] = (
      <span key={`${id}-streaming`} className="streaming-cursor">
        {last}
      </span>
    )
  } else if (isStreaming) {
    parts.push(<span key={`${id}-empty-cursor`} className="streaming-cursor">{''}</span>)
  }

  return <>{parts}</>
}

function renderInline(text: string, keyPrefix: string): React.ReactNode {
  // Split on inline code: `...`
  const parts: React.ReactNode[] = []
  const re = /`([^`\n]+)`/g
  let lastIndex = 0
  let match: RegExpExecArray | null
  let idx = 0

  while ((match = re.exec(text)) !== null) {
    if (match.index > lastIndex) {
      parts.push(text.slice(lastIndex, match.index))
    }
    parts.push(<code key={`${keyPrefix}-ic-${idx}`}>{match[1]}</code>)
    lastIndex = re.lastIndex
    idx++
  }
  if (lastIndex < text.length) {
    parts.push(text.slice(lastIndex))
  }
  return <>{parts}</>
}

function CodeBlock({ lang, code }: { lang: string; code: string }) {
  const [copied, setCopied] = useState(false)
  const onCopy = () => {
    navigator.clipboard.writeText(code).then(
      () => {
        setCopied(true)
        setTimeout(() => setCopied(false), 1500)
      },
      () => {}
    )
  }
  return (
    <div className="code-block">
      <div className="code-block-header">
        <span>{lang}</span>
        <button className="code-copy-btn" onClick={onCopy}>
          {copied ? '✓ Copied' : 'Copy'}
        </button>
      </div>
      <pre><code>{code}</code></pre>
    </div>
  )
}

function ToolCard({ tool }: { tool: ToolInfo }) {
  const [open, setOpen] = useState(tool.status !== 'done')
  return (
    <div className="tool-card">
      <div className="tool-card-header" onClick={() => setOpen(o => !o)}>
        <span className={`tool-status-dot ${tool.status}`} />
        <span>tool: {tool.name}</span>
        <span style={{ marginLeft: 'auto', color: 'var(--text-tertiary, #4a4866)' }}>
          {open ? '▾' : '▸'}
        </span>
      </div>
      {open && (
        <div className="tool-card-body">
          <div style={{ marginBottom: 6 }}>
            <span style={{ color: 'var(--text-tertiary, #4a4866)' }}>input:</span> {tool.input}
          </div>
          {tool.output && (
            <div>
              <span style={{ color: tool.isError ? 'var(--red-text, #f87171)' : 'var(--text-tertiary, #4a4866)' }}>
                {tool.isError ? 'error:' : 'output:'}
              </span> {tool.output}
            </div>
          )}
        </div>
      )}
    </div>
  )
}

export default function MessageList({ messages }: Props) {
  const containerRef = useRef<HTMLDivElement>(null)
  const bottomRef = useRef<HTMLDivElement>(null)
  const userScrolledUp = useRef(false)
  const lastScrollTop = useRef(0)

  useEffect(() => {
    const el = containerRef.current
    if (!el) return
    const onScroll = () => {
      const fromBottom = el.scrollHeight - el.scrollTop - el.clientHeight
      userScrolledUp.current = fromBottom > 100
      lastScrollTop.current = el.scrollTop
    }
    el.addEventListener('scroll', onScroll)
    return () => el.removeEventListener('scroll', onScroll)
  }, [])

  // Auto-scroll on new messages unless user scrolled up
  useEffect(() => {
    if (!userScrolledUp.current) {
      bottomRef.current?.scrollIntoView({ behavior: 'smooth', block: 'end' })
    }
  }, [messages])

  const rendered = useMemo(
    () =>
      messages.map(msg => {
        if (msg.role === 'tool_call' && msg.tool) {
          return (
            <div key={msg.id} className="msg-wrapper">
              <div className="msg-head">
                <span className="msg-label tool">TOOL</span>
                <span className="msg-timestamp">{formatTime(msg.timestamp)}</span>
              </div>
              <ToolCard tool={msg.tool} />
            </div>
          )
        }
        if (msg.role === 'error') {
          return (
            <div key={msg.id} className="msg-wrapper">
              <div className="msg-head">
                <span className="msg-label error">ERROR</span>
                <span className="msg-timestamp">{formatTime(msg.timestamp)}</span>
              </div>
              <div className="msg-error-block">{msg.content}</div>
            </div>
          )
        }
        const labelClass = msg.role
        const content = renderContent(msg.content ?? '', !!msg.isStreaming, msg.id)
        return (
          <div key={msg.id} className="msg-wrapper">
            <div className="msg-head">
              <span className={`msg-label ${labelClass}`}>
                {msg.role === 'user' ? 'TU' : msg.role === 'assistant' ? 'ORION' : 'SYSTEM'}
              </span>
              {msg.model && <span className="msg-timestamp">{msg.model}</span>}
              <span className="msg-timestamp">{formatTime(msg.timestamp)}</span>
            </div>
            <div className="msg-content">{content}</div>
          </div>
        )
      }),
    [messages]
  )

  if (messages.length === 0) {
    return (
      <div className="message-list" ref={containerRef}>
        <div className="welcome-screen">
          <div className="welcome-content">
            <div className="welcome-logo">ORION</div>
            <div className="welcome-subtitle">Asistente de codificación con IA</div>
            <div className="welcome-shortcuts">
              <div className="welcome-section-title">Atajos</div>
              <div className="shortcut-row">
                <span className="shortcut-keys"><kbd>Enter</kbd></span>
                <span>Enviar mensaje</span>
              </div>
              <div className="shortcut-row">
                <span className="shortcut-keys"><kbd>Shift</kbd> + <kbd>Enter</kbd></span>
                <span>Nueva línea</span>
              </div>
              <div className="shortcut-row">
                <span className="shortcut-keys"><kbd>Ctrl</kbd> + <kbd>,</kbd></span>
                <span>Abrir configuración</span>
              </div>
            </div>
            <div className="welcome-modes">
              <div className="welcome-section-title">Modos</div>
              <div className="mode-card-row">
                <div className="mode-card">
                  <div className="mode-card-title">Build</div>
                  <div className="mode-card-desc">Respuesta directa del modelo, sin herramientas</div>
                </div>
                <div className="mode-card">
                  <div className="mode-card-title">Plan</div>
                  <div className="mode-card-desc">Solo lectura: explora el código sin editarlo</div>
                </div>
                <div className="mode-card">
                  <div className="mode-card-title">Agent</div>
                  <div className="mode-card-desc">ORION usa herramientas: leer, escribir, bash, MCP</div>
                </div>
              </div>
            </div>
          </div>
        </div>
        <div ref={bottomRef} />
      </div>
    )
  }

  return (
    <div className="message-list" ref={containerRef}>
      <div className="message-list-inner">
        {rendered}
        <div ref={bottomRef} />
      </div>
    </div>
  )
}
