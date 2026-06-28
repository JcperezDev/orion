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
  return (
    <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
      <circle cx="7" cy="7" r="2.2" stroke="currentColor" strokeWidth="1.2" fill="none" />
      <path
        d="M7 0.5 L7 2.5 M7 11.5 L7 13.5 M0.5 7 L2.5 7 M11.5 7 L13.5 7 M2.4 2.4 L3.8 3.8 M10.2 10.2 L11.6 11.6 M11.6 2.4 L10.2 3.8 M3.8 10.2 L2.4 11.6"
        stroke="currentColor"
        strokeWidth="1.2"
        strokeLinecap="round"
      />
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

  async function handleNewSession() {
    try {
      const session = await invoke<Session>('create_session', { title: 'New session' })
      setSessions(prev => [session, ...prev])
      setActiveId(session.id)
    } catch (e) {
      setError(String(e))
    }
  }

  async function handleSelectSession(id: string) {
    if (id === activeId) return
    try {
      await invoke('set_active_session', { id })
      setActiveId(id)
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
