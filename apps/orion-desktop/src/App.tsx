import { useState, useEffect, useRef } from 'react'
import { invoke } from '@tauri-apps/api/core'
import {
  ChevronRight,
  Hexagon,
  Boxes,
  Cpu,
  Database,
  Terminal,
  ArrowUp,
  Settings as SettingsIcon,
} from 'lucide-react'
import ProviderConnect from './components/ProviderConnect'
import SettingsView from './components/SettingsView'

interface Provider {
  id: string
  name: string
  status: string
  models_count: number
  error?: string
}

type View = 'chat' | 'memory' | 'agents' | 'mcp' | 'models' | 'settings'

interface ChatMessage {
  id: string
  who: 'user' | 'orion'
  time: string
  content: React.ReactNode
}

const navItems = [
  { icon: ChevronRight, label: 'Chat', color: 'text-accent-blue', view: 'chat' as View, badge: null },
  { icon: Hexagon, label: 'Memory', color: 'text-accent-purple', view: 'memory' as View, badge: null },
  { icon: Boxes, label: 'Agents', color: 'text-accent-green', view: 'agents' as View, badge: null },
  { icon: Cpu, label: 'MCP Hub', color: 'text-accent-amber', view: 'mcp' as View, badge: null },
  { icon: Database, label: 'Models', color: 'text-text-dim', view: 'models' as View, badge: null },
  { icon: SettingsIcon, label: 'Settings', color: 'text-text-dim', view: 'settings' as View, badge: null },
]

const COMMANDS = [
  { id: '/model', label: 'Cambiar modelo', desc: 'Selecciona un modelo de IA' },
  { id: '/providers', label: 'Ver providers', desc: 'Lista providers conectados' },
  { id: '/sync', label: 'Sincronizar', desc: 'Sincroniza modelos del provider activo' },
  { id: '/clear', label: 'Limpiar chat', desc: 'Borra todos los mensajes' },
  { id: '/help', label: 'Ayuda', desc: 'Muestra esta ayuda' },
]

const DEFAULT_MODELS: Record<string, string> = {
  openai: 'openai:gpt-4o',
  anthropic: 'anthropic:claude-3-5-sonnet-20241022',
  google: 'google:gemini-1.5-pro',
  deepseek: 'deepseek:deepseek-chat',
  groq: 'groq:llama-3.1-70b-versatile',
  mistral: 'mistral:mistral-large-latest',
  together: 'together:meta-llama/Llama-3-70b-chat-hf',
  perplexity: 'perplexity:llama-3.1-sonar-large-128k-online',
  minimax: 'minimax:MiniMax-M3',
  openrouter: 'openrouter:anthropic/claude-3.5-sonnet',
}

function genId() {
  return Date.now().toString(36) + Math.random().toString(36).slice(2, 8)
}

function now() {
  return new Date().toLocaleTimeString('es', { hour: '2-digit', minute: '2-digit' })
}

function Pill({ label, tone }: { label: string; tone: 'blue' | 'green' | 'purple' | 'amber' }) {
  const map = {
    blue: 'text-accent-blue bg-accent-blue-bg border-accent-blue/30',
    green: 'text-accent-green bg-accent-green-bg border-accent-green/30',
    purple: 'text-accent-purple bg-accent-purple-bg border-accent-purple/30',
    amber: 'text-accent-amber bg-accent-amber-bg border-accent-amber/30',
  }
  return (
    <span className={`inline-flex items-center gap-1.5 rounded-full border px-2.5 py-0.5 text-[10px] tracking-[0.12em] uppercase ${map[tone]}`}>
      <span className="size-1.5 rounded-full bg-current" />
      {label}
    </span>
  )
}

function Section({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div>
      <div className="mb-2 px-1 text-[9px] tracking-[0.2em] text-subtle uppercase">{label}</div>
      {children}
    </div>
  )
}

function Message({ who, time, children }: { who: 'user' | 'orion'; time: string; children: React.ReactNode }) {
  return (
    <div className="space-y-1">
      <div className="flex items-center gap-2">
        <span className={`text-[10px] tracking-[0.2em] uppercase ${who === 'orion' ? 'text-accent-blue' : 'text-accent-purple'}`}>
          {who === 'orion' ? 'ORION' : 'TU'}
        </span>
        <span className="text-[9px] text-subtle">{time}</span>
      </div>
      <div className={`text-xs leading-relaxed ${who === 'orion' ? 'text-text' : 'text-text-dim'}`}>
        {children}
      </div>
    </div>
  )
}

function CodeBlock({ children }: { children: React.ReactNode }) {
  return (
    <div className="mt-1.5 rounded-md border border-border bg-surface-3/80 px-3 py-2.5 text-[11px] leading-relaxed">
      {children}
    </div>
  )
}

function PlaceholderView({ title, message }: { title: string; message: string }) {
  return (
    <div className="flex h-full items-center justify-center">
      <div className="max-w-md text-center space-y-2">
        <div className="text-xs tracking-[0.2em] uppercase text-accent-blue">{title}</div>
        <div className="text-xs text-subtle leading-relaxed">{message}</div>
      </div>
    </div>
  )
}

export type { Provider }

export default function App() {
  const [activeView, setActiveView] = useState<View>('chat')
  const [isOnboarded, setIsOnboarded] = useState(false)
  const [defaultModel, setDefaultModel] = useState<string | null>(null)
  const [activeProvider, setActiveProvider] = useState<string | null>(null)
  const [input, setInput] = useState('')
  const [showCommands, setShowCommands] = useState(false)
  const [commandIndex, setCommandIndex] = useState(0)
  const [messages, setMessages] = useState<ChatMessage[]>([
    { id: genId(), who: 'orion', time: 'ahora', content: 'Sistema inicializado. Conecta tu provider de IA para comenzar.' }
  ])
  const inputRef = useRef<HTMLInputElement>(null)
  const messagesEndRef = useRef<HTMLDivElement>(null)
  const messagesContainerRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    checkOnboarding()
  }, [])

  useEffect(() => {
    scrollToBottom()
  }, [messages])

  function scrollToBottom() {
    const el = messagesContainerRef.current
    if (el) {
      el.scrollTop = el.scrollHeight
    }
  }

  async function checkOnboarding() {
    try {
      const connected = await invoke<Provider[]>('get_connected_providers')
      const hasConnected = connected.some(p => p.status === 'connected')
      const modelId = await invoke<string | null>('get_default_model')

      if (hasConnected) {
        const activeP = connected.find(p => p.status === 'connected')
        if (activeP) setActiveProvider(activeP.id)
      }

      if (hasConnected && modelId) {
        setIsOnboarded(true)
        setDefaultModel(modelId)
      } else if (hasConnected) {
        setIsOnboarded(true)
      }
    } catch (e) {
      console.error('Failed to check onboarding:', e)
    }
  }

  async function handleConnected() {
    try {
      const providers = await invoke<any[]>('get_connected_providers')
      const connected = providers.find(p => p.status === 'connected')

      if (connected) {
        setActiveProvider(connected.id)
        if (DEFAULT_MODELS[connected.id]) {
          await invoke('set_default_model', { modelId: DEFAULT_MODELS[connected.id] })
          setDefaultModel(DEFAULT_MODELS[connected.id])
        }
      }

      setIsOnboarded(true)
    } catch (e) {
      console.error('Failed to handle connected:', e)
      setIsOnboarded(true)
    }
  }

  function addOrionMessage(content: React.ReactNode) {
    setMessages(prev => [...prev, { id: genId(), who: 'orion', time: now(), content }])
  }

  function handleCommand(cmd: string) {
    const command = cmd.trim().split(' ')[0]

    switch (command) {
      case '/clear':
        setMessages([{ id: genId(), who: 'orion', time: now(), content: 'Chat limpiado.' }])
        break

      case '/help':
        addOrionMessage(
          <CodeBlock>
            {COMMANDS.map(c => (
              <div key={c.id} className="flex gap-4 mb-1">
                <span className="text-accent-blue font-mono">{c.id}</span>
                <span className="text-text">{c.label}</span>
                <span className="text-subtle">- {c.desc}</span>
              </div>
            ))}
          </CodeBlock>
        )
        break

      case '/providers':
        invoke<any[]>('get_connected_providers')
          .then(providers => {
            const list = providers.map(p => `${p.name}: ${p.status}`).join('\n')
            addOrionMessage(
              <CodeBlock><pre className="text-text whitespace-pre-wrap">{list || 'No providers conectados'}</pre></CodeBlock>
            )
          })
          .catch(() => {
            addOrionMessage(<CodeBlock><span className="text-accent-red">Error cargando providers</span></CodeBlock>)
          })
        break

      case '/model':
        Promise.all([
          invoke<string | null>('get_default_model'),
          invoke<any[]>('list_models', { provider: null }),
        ])
          .then(([modelId, models]) => {
            addOrionMessage(
              <CodeBlock>
                <div className="text-text mb-2">Modelo actual: {modelId || 'ninguno'}</div>
                <div className="text-subtle text-[10px] mb-2">Modelos disponibles: {models.length}</div>
                {models.length > 0 && (
                  <div className="mt-2 space-y-1">
                    {models.slice(0, 5).map((m: any) => (
                      <div key={m.id} className="text-text-dim">- {m.name} ({m.provider})</div>
                    ))}
                  </div>
                )}
              </CodeBlock>
            )
          })
          .catch(() => {
            addOrionMessage(<CodeBlock><span className="text-accent-red">Error cargando modelos</span></CodeBlock>)
          })
        break

      case '/sync': {
        const targetProvider = activeProvider || 'openrouter'
        addOrionMessage(<CodeBlock><span className="text-accent-blue">Sincronizando {targetProvider}...</span></CodeBlock>)
        invoke('sync_provider_models', { providerId: targetProvider })
          .then(() => {
            addOrionMessage(<CodeBlock><span className="text-accent-green">Modelos sincronizados</span></CodeBlock>)
          })
          .catch(() => {
            addOrionMessage(<CodeBlock><span className="text-accent-amber">Sync no disponible para este provider</span></CodeBlock>)
          })
        break
      }

      default:
        addOrionMessage(<CodeBlock><span className="text-subtle">Comando desconocido. Escribe /help para ver comandos disponibles.</span></CodeBlock>)
    }
  }

  async function submitMessage() {
    const text = input.trim()
    if (!text) return

    setInput('')
    setMessages(prev => [...prev, { id: genId(), who: 'user', time: now(), content: text }])

    if (text.startsWith('/')) {
      handleCommand(text)
      return
    }

    const loadingId = genId()
    setMessages(prev => [...prev, {
      id: loadingId,
      who: 'orion',
      time: now(),
      content: <CodeBlock><span className="text-accent-blue">Pensando...</span></CodeBlock>
    }])

    const removeLoading = () => {
      setMessages(prev => prev.filter(m => m.id !== loadingId))
    }

    try {
      let modelId = defaultModel

      if (!modelId) {
        const providers = await invoke<any[]>('get_connected_providers')
        const connected = providers.find(p => p.status === 'connected')

        if (!connected) {
          removeLoading()
          addOrionMessage(<CodeBlock><span className="text-accent-red">No hay provider conectado. Conecta uno desde el inicio.</span></CodeBlock>)
          return
        }

        if (!DEFAULT_MODELS[connected.id]) {
          removeLoading()
          addOrionMessage(<CodeBlock><span className="text-accent-red">No hay modelo por defecto para {connected.name}.</span></CodeBlock>)
          return
        }

        modelId = DEFAULT_MODELS[connected.id]
        setDefaultModel(modelId)
        setActiveProvider(connected.id)
      }

      const [providerId, model] = modelId.split(':')

      const payload = [{ role: 'user', content: text }]

      const response = await invoke<string>('chat', {
        providerId,
        modelId: model,
        messages: payload
      })

      removeLoading()
      addOrionMessage(response)
    } catch (err) {
      removeLoading()
      addOrionMessage(<CodeBlock><span className="text-accent-red">Error: {String(err)}</span></CodeBlock>)
    }
  }

  function handleSend(e: React.FormEvent) {
    e.preventDefault()
    submitMessage()
  }

  if (!isOnboarded) {
    return <ProviderConnect onConnected={handleConnected} />
  }

  return (
    <div className="relative w-full h-screen overflow-hidden bg-background text-foreground font-mono">
      <div className="grid-bg pointer-events-none absolute inset-0" />
      <div className="pointer-events-none absolute inset-0 bg-[radial-gradient(ellipse_at_top,oklch(0.3_0.1_260/0.25),transparent_60%)]" />

      <div className="relative mx-auto flex h-screen max-w-[1400px] flex-col px-6 py-6 overflow-hidden">
        {/* Top bar */}
        <header className="flex items-center justify-between border-b border-border pb-4">
          <div className="flex items-center gap-3">
            <div className="flex items-center gap-2.5">
              <div className="relative grid size-8 place-items-center">
                <span className="absolute inset-0 rounded-md border border-accent-blue/40" />
                <span className="absolute inset-2 rounded-sm border border-accent-blue/20 rotate-45" />
                <span className="relative size-1.5 rounded-full bg-accent-blue" />
              </div>
              <div className="leading-tight">
                <div className="text-sm font-semibold tracking-[0.35em] text-text">ORION</div>
                <div className="text-[10px] tracking-[0.2em] text-subtle uppercase">v0.4 - local</div>
              </div>
            </div>
            <div className="ml-4 hidden h-6 w-px bg-border md:block" />
            <div className="hidden items-center gap-2 md:flex">
              <Pill label={defaultModel?.split(':')[1] || 'sin modelo'} tone="blue" />
            </div>
          </div>
        </header>

        {/* Body */}
        <div className="mt-5 flex flex-1 gap-5 min-h-0">
          {/* Sidebar */}
          <aside className="flex w-[200px] flex-shrink-0 flex-col gap-6 rounded-lg border border-border bg-surface/40 p-4 backdrop-blur h-full overflow-y-auto">
            <Section label="Navigation">
              <ul className="space-y-0.5">
                {navItems.map((n) => {
                  const Icon = n.icon
                  return (
                    <li
                      key={n.label}
                      role="button"
                      tabIndex={0}
                      onClick={() => setActiveView(n.view)}
                      onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); setActiveView(n.view) } }}
                      className={`group flex cursor-pointer items-center gap-2.5 rounded-md px-2.5 py-1.5 text-xs transition outline-none ${
                        activeView === n.view
                          ? 'bg-surface-2 text-text'
                          : 'text-text-dim hover:bg-surface-2/60 hover:text-text focus-visible:bg-surface-2/60'
                      }`}
                    >
                      <Icon className={`size-3.5 ${n.color}`} />
                      <span className="flex-1">{n.label}</span>
                      {n.badge && (
                        <span className="rounded bg-surface-3 px-1.5 py-0.5 text-[9px] text-subtle">{n.badge}</span>
                      )}
                    </li>
                  )
                })}
              </ul>
            </Section>
          </aside>

          {/* Main */}
          {activeView === 'chat' ? (
            <main className="flex flex-1 min-h-0 flex-col overflow-hidden rounded-lg border border-border bg-surface/40 backdrop-blur">
              <div className="flex items-center justify-between border-b border-border px-4 py-2.5">
                <div className="flex items-center gap-2 text-[10px] tracking-[0.2em] uppercase text-accent-blue">
                  <span className="size-1.5 rounded-full bg-accent-blue" />
                  chat
                </div>
                <div className="text-[10px] tracking-[0.15em] text-subtle uppercase">sesion local</div>
              </div>

              <div ref={messagesContainerRef} className="scroll-thin flex-1 space-y-5 overflow-y-auto px-6 py-5">
                {messages.map((msg) => (
                  <Message key={msg.id} who={msg.who} time={msg.time}>{msg.content}</Message>
                ))}
                <div ref={messagesEndRef} />
              </div>

              {/* Input */}
              <div className="border-t border-border bg-surface-3/60 p-3">
                <form onSubmit={handleSend} className="flex items-center gap-2 rounded-lg border border-border bg-background/60 px-3 py-2 focus-within:border-accent-blue/60 relative">
                  <Terminal className="size-3.5 text-accent-blue" />
                  <input
                    ref={inputRef}
                    value={input}
                    onChange={(e) => {
                      const val = e.target.value
                      setInput(val)
                      if (val.startsWith('/')) {
                        setShowCommands(true)
                        setCommandIndex(0)
                      } else {
                        setShowCommands(false)
                      }
                    }}
                    onKeyDown={(e) => {
                      if (showCommands) {
                        if (e.key === 'ArrowDown') {
                          e.preventDefault()
                          setCommandIndex(i => Math.min(i + 1, COMMANDS.length - 1))
                        } else if (e.key === 'ArrowUp') {
                          e.preventDefault()
                          setCommandIndex(i => Math.max(i - 1, 0))
                        } else if (e.key === 'Enter') {
                          e.preventDefault()
                          const cmd = COMMANDS[commandIndex]
                          setInput('')
                          setShowCommands(false)
                          handleCommand(cmd.id)
                        } else if (e.key === 'Escape') {
                          setShowCommands(false)
                        }
                        return
                      }

                      if (e.key === 'Enter' && !e.shiftKey) {
                        e.preventDefault()
                        submitMessage()
                      }
                    }}
                    onBlur={() => setTimeout(() => setShowCommands(false), 150)}
                    placeholder="Escribe un mensaje o / para comandos..."
                    className="flex-1 bg-transparent text-xs text-text placeholder:text-subtle focus:outline-none"
                  />
                  <button
                    type="submit"
                    disabled={!input.trim()}
                    className="grid size-7 place-items-center rounded-md border border-accent-blue/40 bg-accent-blue-bg text-accent-blue transition hover:border-accent-blue/70 disabled:opacity-40 disabled:cursor-not-allowed"
                    aria-label="Enviar"
                  >
                    <ArrowUp className="size-3.5" />
                  </button>

                  {/* Command dropdown */}
                  {showCommands && (
                    <div className="absolute bottom-full left-0 right-0 mb-1 max-h-64 overflow-y-auto rounded-lg border border-border bg-[var(--surface)] shadow-lg">
                      {COMMANDS.map((cmd, i) => (
                        <div
                          key={cmd.id}
                          onMouseDown={(e) => {
                            e.preventDefault()
                            setInput('')
                            setShowCommands(false)
                            handleCommand(cmd.id)
                            inputRef.current?.focus()
                          }}
                          className={`px-3 py-2 cursor-pointer flex items-center justify-between ${
                            i === commandIndex ? 'bg-accent-blue-bg' : 'hover:bg-surface-2'
                          }`}
                        >
                          <div className="flex items-center gap-3 min-w-0">
                            <Terminal className="size-3 text-accent-blue flex-shrink-0" />
                            <span className="text-xs text-accent-blue font-mono">{cmd.id}</span>
                            <span className="text-xs text-text truncate">{cmd.label}</span>
                          </div>
                          <span className="text-[10px] text-subtle flex-shrink-0 ml-2">{cmd.desc}</span>
                        </div>
                      ))}
                    </div>
                  )}
                </form>
              </div>
            </main>
          ) : activeView === 'settings' ? (
            <main className="flex flex-1 min-h-0 overflow-hidden rounded-lg border border-border bg-surface/40 backdrop-blur">
              <SettingsView />
            </main>
          ) : (
            <main className="flex flex-1 min-h-0 overflow-hidden rounded-lg border border-border bg-surface/40 backdrop-blur">
              {activeView === 'memory' && (
                <PlaceholderView title="MEMORY" message="Memoria persistente del agente. Proximamente disponible." />
              )}
              {activeView === 'agents' && (
                <PlaceholderView title="AGENTS" message="Sistema de agentes. Proximamente disponible." />
              )}
              {activeView === 'mcp' && (
                <PlaceholderView title="MCP HUB" message="Servidores MCP (Model Context Protocol). Proximamente disponible." />
              )}
              {activeView === 'models' && (
                <PlaceholderView title="MODELS" message="Catalogo de modelos. Escribe /model en el chat para ver y cambiar el modelo activo." />
              )}
            </main>
          )}
        </div>
      </div>
    </div>
  )
}
