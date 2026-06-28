import { useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'

interface Session {
  id: string
  title: string
  created_at: string
  updated_at: string
  message_count: number
  active_model: string | null
}

export interface ProviderView {
  id: string
  name: string
  kind: string
  enabled: boolean
  available: boolean
  has_api_key: boolean
  base_url?: string
  models_count: number
}

interface Props {
  onOpenSettings: () => void
  workspaceName?: string
  workspacePath?: string
}

function PlusIcon() {
  return (
    <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
      <line x1="6" y1="2" x2="6" y2="10" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" />
      <line x1="2" y1="6" x2="10" y2="6" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" />
    </svg>
  )
}

function GearIcon() {
  // A real cog/gear (Bootstrap "gear"), not the sun-like rayed circle.
  return (
    <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
      <path d="M8 4.754a3.246 3.246 0 1 0 0 6.492 3.246 3.246 0 0 0 0-6.492M5.754 8a2.246 2.246 0 1 1 4.492 0 2.246 2.246 0 0 1-4.492 0" />
      <path d="M9.796 1.343c-.527-1.79-3.065-1.79-3.592 0l-.094.319a.873.873 0 0 1-1.255.52l-.292-.16c-1.64-.892-3.433.902-2.54 2.541l.159.292a.873.873 0 0 1-.52 1.255l-.319.094c-1.79.527-1.79 3.065 0 3.592l.319.094a.873.873 0 0 1 .52 1.255l-.16.292c-.892 1.64.902 3.434 2.541 2.54l.292-.159a.873.873 0 0 1 1.255.52l.094.319c.527 1.79 3.065 1.79 3.592 0l.094-.319a.873.873 0 0 1 1.255-.52l.292.16c1.64.893 3.434-.902 2.54-2.541l-.159-.292a.873.873 0 0 1 .52-1.255l.319-.094c1.79-.527 1.79-3.065 0-3.592l-.319-.094a.873.873 0 0 1-.52-1.255l.16-.292c.893-1.64-.902-3.433-2.541-2.54l-.292.159a.873.873 0 0 1-1.255-.52zm-2.633.283c.246-.835 1.428-.835 1.674 0l.094.319a1.873 1.873 0 0 0 2.693 1.115l.291-.16c.764-.415 1.6.42 1.184 1.185l-.159.292a1.873 1.873 0 0 0 1.116 2.692l.318.094c.835.246.835 1.428 0 1.674l-.319.094a1.873 1.873 0 0 0-1.115 2.693l.16.291c.415.764-.42 1.6-1.185 1.184l-.291-.159a1.873 1.873 0 0 0-2.693 1.116l-.094.318c-.246.835-1.428.835-1.674 0l-.094-.319a1.873 1.873 0 0 0-2.692-1.115l-.292.16c-.764.415-1.6-.42-1.184-1.185l.159-.291A1.873 1.873 0 0 0 1.945 8.93l-.319-.094c-.835-.246-.835-1.428 0-1.674l.319-.094A1.873 1.873 0 0 0 3.06 4.377l-.16-.292c-.415-.764.42-1.6 1.185-1.184l.292.159a1.873 1.873 0 0 0 2.692-1.115z" />
    </svg>
  )
}

function TrashIcon() {
  return (
    <svg width="13" height="13" viewBox="0 0 14 14" fill="none">
      <path d="M2.5 3.5 H11.5" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" />
      <path d="M5.5 3.5 V2.5 H8.5 V3.5" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" strokeLinejoin="round" />
      <path d="M3.5 3.5 L4 11.5 H10 L10.5 3.5" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" strokeLinejoin="round" />
      <path d="M6 5.5 V9.5 M8 5.5 V9.5" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" />
    </svg>
  )
}

function deriveWorkspace(path: string | undefined, fallbackName: string | undefined): { name: string; path: string } {
  if (path) {
    const segments = path.split('/').filter(Boolean)
    const last = segments[segments.length - 1] || 'workspace'
    return { name: fallbackName || last, path }
  }
  return { name: fallbackName || 'workspace', path: '~/' }
}

function workspaceInitial(name: string): string {
  return name.trim().charAt(0).toUpperCase() || 'W'
}

function truncatePath(p: string, max: number = 32): string {
  if (p.length <= max) return p
  const head = p.slice(0, 12)
  const tail = p.slice(-(max - 15))
  return `${head}…${tail}`
}

export default function Sidebar({ onOpenSettings, workspaceName, workspacePath }: Props) {
  const [sessions, setSessions] = useState<Session[]>([])
  const [activeId, setActiveId] = useState<string | null>(null)
  const [showAll, setShowAll] = useState(false)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [searchQuery, setSearchQuery] = useState('')

  useEffect(() => {
    loadSessions()
  }, [])

  async function loadSessions() {
    setLoading(true)
    setError(null)
    try {
      const [all, active] = await Promise.all([
        invoke<Session[]>('get_sessions'),
        invoke<Session | null>('get_active_session'),
      ])
      setSessions(all)
      setActiveId(active?.id ?? all[0]?.id ?? null)
    } catch (e) {
      setError(String(e))
    } finally {
      setLoading(false)
    }
  }

  function notifySessionChange(id: string | null) {
    window.dispatchEvent(new CustomEvent('orion:session', { detail: id }))
  }

  // Ctrl+N → new session (from the global shortcut).
  useEffect(() => {
    const onNew = () => { handleNewSession() }
    window.addEventListener('orion:new-session', onNew)
    return () => window.removeEventListener('orion:new-session', onNew)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  async function handleNewSession() {
    try {
      const session = await invoke<Session>('create_session', { title: 'New session' })
      setSessions(prev => [session, ...prev])
      setActiveId(session.id)
      await invoke('set_active_session', { id: session.id })
      notifySessionChange(session.id)
    } catch (e) {
      setError(String(e))
    }
  }

  async function handleSelectSession(id: string) {
    // Always notify so clicking a session (even the active one) returns to chat.
    if (id === activeId) {
      notifySessionChange(id)
      return
    }
    try {
      await invoke('set_active_session', { id })
      setActiveId(id)
      notifySessionChange(id)
    } catch (e) {
      setError(String(e))
    }
  }

  async function handleDeleteSession(id: string, e: React.MouseEvent) {
    e.stopPropagation()
    try {
      await invoke('delete_session', { id })
      setSessions(prev => prev.filter(s => s.id !== id))
      if (id === activeId) {
        setActiveId(null)
        notifySessionChange(null)
      }
    } catch (err) {
      setError(String(err))
    }
  }

  const ws = deriveWorkspace(workspacePath, workspaceName)

  const filtered = searchQuery
    ? sessions.filter(s => s.title.toLowerCase().includes(searchQuery.toLowerCase()))
    : sessions

  const visible = showAll ? filtered : filtered.slice(0, 8)
  const hasMore = filtered.length > 8

  return (
    <aside className="sidebar">
      <div className="sidebar-workspace">
        <div className="workspace-row">
          <div className="workspace-avatar">{workspaceInitial(ws.name)}</div>
          <div className="workspace-info">
            <div className="workspace-name">{ws.name}</div>
            <div className="workspace-path" title={ws.path}>{truncatePath(ws.path)}</div>
          </div>
        </div>
      </div>

      <button className="new-session-btn" onClick={handleNewSession}>
        <PlusIcon />
        <span>New session</span>
      </button>

      <div className="sessions-label">
        <span>SESSIONS</span>
        <span className="sessions-count">{filtered.length}</span>
      </div>

      <div className="sidebar-search">
        <input
          className="sidebar-search-input"
          type="text"
          placeholder="Search sessions..."
          value={searchQuery}
          onChange={e => setSearchQuery(e.target.value)}
        />
      </div>

      <div className="sessions-list">
        {loading && sessions.length === 0 && (
          <div className="session-empty">Loading…</div>
        )}
        {!loading && filtered.length === 0 && (
          <div className="session-empty">
            {searchQuery ? 'No results' : 'No sessions yet'}
          </div>
        )}
        {error && (
          <div className="session-empty" style={{ color: 'var(--red-text, #f87171)' }}>{error}</div>
        )}
        {visible.map(s => (
          <div
            key={s.id}
            className={`session-item${s.id === activeId ? ' active' : ''}`}
            onClick={() => handleSelectSession(s.id)}
            title={s.title}
          >
            <span className="session-dot" />
            <span className="session-title">{s.title}</span>
            <button
              className="session-delete"
              aria-label="Delete session"
              title="Delete session"
              onClick={e => handleDeleteSession(s.id, e)}
            >
              <TrashIcon />
            </button>
          </div>
        ))}
        {!showAll && hasMore && !searchQuery && (
          <button className="load-more" onClick={() => setShowAll(true)}>
            Load more ({filtered.length - 8})
          </button>
        )}
      </div>

      <div className="sidebar-bottom">
        <button className="settings-btn" onClick={onOpenSettings}>
          <GearIcon />
          <span>Settings</span>
        </button>
      </div>
    </aside>
  )
}
