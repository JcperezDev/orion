import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import {
  Check,
  X,
  Trash2,
  RefreshCw,
  Cpu,
  Palette,
  Key,
  Loader2,
  ChevronRight,
  Plug,
  Database,
  Bot,
  Server,
  Shield,
  Globe,
  Keyboard,
  Settings as SettingsIcon,
} from 'lucide-react'
import MCPServerView from './MCPServerView'

interface Provider {
  id: string
  name: string
  kind: string
  enabled: boolean
  available: boolean
  has_api_key: boolean
  base_url?: string
  models_count: number
}

interface Model {
  id: string
  provider: string
  name: string
  context_window?: number
  supports_vision: boolean
  supports_tools: boolean
  is_free: boolean
  is_local: boolean
  is_available: boolean
}

const PROVIDER_META: Record<string, { color: string; dashboard?: string }> = {
  openrouter: { color: '#534AB7', dashboard: 'https://openrouter.ai/keys' },
  openai: { color: '#10a37f', dashboard: 'https://platform.openai.com/api-keys' },
  anthropic: { color: '#d4a574', dashboard: 'https://console.anthropic.com/keys' },
  google: { color: '#4285f4', dashboard: 'https://aistudio.google.com/app/apikey' },
  deepseek: { color: '#4d9eff', dashboard: 'https://platform.deepseek.com/api_keys' },
  groq: { color: '#f55036', dashboard: 'https://console.groq.com/keys' },
  mistral: { color: '#f7a800', dashboard: 'https://console.mistral.ai/api-keys/' },
  together: { color: '#10b981', dashboard: 'https://api.together.xyz/settings/api-keys' },
  perplexity: { color: '#20b2aa', dashboard: 'https://www.perplexity.ai/settings/api' },
  minimax: { color: '#a855f7', dashboard: 'https://www.minimax.io/' },
  ollama: { color: '#1D9E75' },
  custom: { color: '#888888' },
}

const THEMES: Array<{ id: string; name: string; bg: string; side: string; acc: string; txt: string; brd: string }> = [
  { id: 'orion-dark', name: 'Orion Dark', bg: '#0a0a0c', side: '#111116', acc: '#534AB7', txt: '#e2e0f0', brd: '#1e1e28' },
  { id: 'tokyonight', name: 'Tokyonight', bg: '#1a1b26', side: '#16161e', acc: '#7aa2f7', txt: '#c0caf5', brd: '#414868' },
  { id: 'catppuccin', name: 'Catppuccin', bg: '#1e1e2e', side: '#181825', acc: '#cba6f7', txt: '#cdd6f4', brd: '#313244' },
  { id: 'one-dark', name: 'One Dark', bg: '#282c34', side: '#21252b', acc: '#61afef', txt: '#abb2bf', brd: '#3e4451' },
  { id: 'dracula', name: 'Dracula', bg: '#282a36', side: '#21222c', acc: '#bd93f9', txt: '#f8f8f2', brd: '#44475a' },
  { id: 'nord', name: 'Nord', bg: '#2e3440', side: '#272c36', acc: '#88c0d0', txt: '#eceff4', brd: '#3b4252' },
  { id: 'gruvbox', name: 'Gruvbox', bg: '#282828', side: '#1d2021', acc: '#fabd2f', txt: '#ebdbb2', brd: '#3c3836' },
  { id: 'rose-pine', name: 'Rose Pine', bg: '#191724', side: '#1f1d2e', acc: '#c4a7e7', txt: '#e0def4', brd: '#2a2739' },
  { id: 'kanagawa', name: 'Kanagawa', bg: '#1f1f28', side: '#16161d', acc: '#7e9cd8', txt: '#dcd7ba', brd: '#2a2a37' },
  { id: 'everforest', name: 'Everforest', bg: '#2d353b', side: '#272e33', acc: '#a7c080', txt: '#d3c6aa', brd: '#3d484d' },
  { id: 'monokai', name: 'Monokai', bg: '#272822', side: '#1e1f1c', acc: '#f92672', txt: '#f8f8f2', brd: '#3e3d32' },
  { id: 'synthwave', name: 'Synthwave', bg: '#262335', side: '#1d1927', acc: '#ff7edb', txt: '#ffffff', brd: '#3b3557' },
]

type Section =
  | 'general'
  | 'providers'
  | 'language'
  | 'appearance'
  | 'shortcuts'
  | 'memory'
  | 'agents'
  | 'mcp'
  | 'models'
  | 'servers'
  | 'permissions'

type SectionGroup = 'GENERAL' | 'INTERFACE' | 'TOOLS' | 'SYSTEM'

interface SectionDef {
  id: Section
  label: string
  icon: any
  group: SectionGroup
}

export default function SettingsView() {
  const [activeSection, setActiveSection] = useState<Section>('providers')
  const [providers, setProviders] = useState<Provider[]>([])
  const [models, setModels] = useState<Model[]>([])
  const [defaultModel, setDefaultModel] = useState<string | null>(null)
  const [theme, setTheme] = useState('orion-dark')
  const [loading, setLoading] = useState(true)
  const [syncing, setSyncing] = useState<string | null>(null)
  const [editingKey, setEditingKey] = useState<string | null>(null)
  const [newKey, setNewKey] = useState('')

  useEffect(() => {
    loadAll()
    const saved = localStorage.getItem('orion-theme')
    if (saved) setTheme(saved)
  }, [])

  useEffect(() => {
    document.documentElement.setAttribute('data-theme', theme)
    localStorage.setItem('orion-theme', theme)
    const t = THEMES.find(x => x.id === theme)
    if (t) {
      document.documentElement.style.setProperty('--bg-primary', t.bg)
      document.documentElement.style.setProperty('--bg-secondary', t.side)
      document.documentElement.style.setProperty('--accent', t.acc)
      document.documentElement.style.setProperty('--text-primary', t.txt)
      document.documentElement.style.setProperty('--border-subtle', t.brd)
    }
  }, [theme])

  async function loadAll() {
    setLoading(true)
    try {
      const [p, m, d] = await Promise.all([
        invoke<Provider[]>('list_providers'),
        invoke<Model[]>('list_models', { provider: null }),
        invoke<string | null>('get_default_model'),
      ])
      setProviders(p)
      setModels(m)
      setDefaultModel(d)
    } catch (e) {
      console.error('Failed to load settings:', e)
    } finally {
      setLoading(false)
    }
  }

  async function handleSaveKey(providerId: string) {
    if (!newKey.trim()) return
    try {
      await invoke('save_provider', { providerId, apiKey: newKey.trim() })
      await invoke('reload_registry')
      setEditingKey(null)
      setNewKey('')
      await loadAll()
    } catch (e) {
      console.error('Save failed:', e)
    }
  }

  async function handleDeleteKey(providerId: string) {
    try {
      await invoke('delete_provider_api_key', { providerId })
      await invoke('reload_registry')
      await loadAll()
    } catch (e) {
      console.error('Delete failed:', e)
    }
  }

  async function handleSync(providerId: string) {
    setSyncing(providerId)
    try {
      await invoke('sync_provider_models', { providerId })
      await loadAll()
    } catch (e) {
      console.error('Sync failed:', e)
    } finally {
      setSyncing(null)
    }
  }

  async function handleSetDefault(modelId: string) {
    try {
      await invoke('set_active_model', { modelId })
      setDefaultModel(modelId)
    } catch (e) {
      console.error('Set default failed:', e)
    }
  }

  const sections: SectionDef[] = [
    { id: 'general',     label: 'General',     icon: SettingsIcon, group: 'GENERAL' },
    { id: 'providers',   label: 'Providers',   icon: Key,          group: 'GENERAL' },
    { id: 'language',    label: 'Language',    icon: Globe,        group: 'GENERAL' },
    { id: 'appearance',  label: 'Appearance',  icon: Palette,      group: 'INTERFACE' },
    { id: 'shortcuts',   label: 'Shortcuts',   icon: Keyboard,     group: 'INTERFACE' },
    { id: 'memory',      label: 'Memory',      icon: Database,     group: 'TOOLS' },
    { id: 'agents',      label: 'Agents',      icon: Bot,          group: 'TOOLS' },
    { id: 'mcp',         label: 'MCP Hub',     icon: Plug,         group: 'TOOLS' },
    { id: 'models',      label: 'Models',      icon: Cpu,          group: 'TOOLS' },
    { id: 'servers',     label: 'Servers',     icon: Server,       group: 'SYSTEM' },
    { id: 'permissions', label: 'Permissions', icon: Shield,       group: 'SYSTEM' },
  ]

  const groups: SectionGroup[] = ['GENERAL', 'INTERFACE', 'TOOLS', 'SYSTEM']
  const sectionsByGroup = (g: SectionGroup) => sections.filter(s => s.group === g)

  if (loading) {
    return (
      <div className="flex h-full items-center justify-center">
        <Loader2 className="size-5 animate-spin text-accent" />
      </div>
    )
  }

  return (
    <div className="flex h-full">
      {/* Settings nav */}
      <div
        className="w-[210px] flex-shrink-0 border-r p-3 overflow-y-auto"
        style={{ borderColor: 'var(--border-subtle)' }}
      >
        <div
          className="px-2 mb-3"
          style={{
            fontFamily: "'JetBrains Mono', monospace",
            fontSize: '10px',
            letterSpacing: '0.2em',
            color: 'var(--text-tertiary)',
            textTransform: 'uppercase',
            fontWeight: 600,
          }}
        >
          Settings
        </div>
        {groups.map(g => (
          <div key={g} style={{ marginBottom: 10 }}>
            <div
              style={{
                padding: '6px 12px 4px',
                fontFamily: "'JetBrains Mono', monospace",
                fontSize: '9px',
                letterSpacing: '0.12em',
                color: 'var(--text-tertiary)',
                textTransform: 'uppercase',
                fontWeight: 600,
              }}
            >
              {g}
            </div>
            <ul className="space-y-0.5">
              {sectionsByGroup(g).map(s => {
                const Icon = s.icon
                return (
                  <li
                    key={s.id}
                    role="button"
                    tabIndex={0}
                    onClick={() => setActiveSection(s.id)}
                    onKeyDown={(e) => {
                      if (e.key === 'Enter' || e.key === ' ') {
                        e.preventDefault()
                        setActiveSection(s.id)
                      }
                    }}
                    className="flex cursor-pointer items-center gap-2.5 rounded-md outline-none transition"
                    style={{
                      padding: '7px 10px',
                      margin: '1px 6px',
                      fontSize: '12px',
                      background: activeSection === s.id ? 'var(--bg-tertiary)' : 'transparent',
                      color: activeSection === s.id ? 'var(--text-primary)' : 'var(--text-secondary)',
                    }}
                  >
                    <Icon
                      className="size-3.5"
                      style={{ color: activeSection === s.id ? 'var(--accent)' : 'var(--text-tertiary)' }}
                    />
                    <span className="flex-1">{s.label}</span>
                    {activeSection === s.id && <ChevronRight className="size-3" style={{ color: 'var(--text-tertiary)' }} />}
                  </li>
                )
              })}
            </ul>
          </div>
        ))}
      </div>

      {/* Settings content */}
      <div className="flex-1 overflow-y-auto" style={{ padding: '26px 30px' }}>
        {activeSection === 'general' && <GeneralSection />}
        {activeSection === 'providers' && (
          <ProvidersSection
            providers={providers}
            editingKey={editingKey}
            newKey={newKey}
            syncing={syncing}
            onEdit={(id) => { setEditingKey(id); setNewKey('') }}
            onChangeKey={setNewKey}
            onSaveKey={handleSaveKey}
            onCancelEdit={() => { setEditingKey(null); setNewKey('') }}
            onDeleteKey={handleDeleteKey}
            onSync={handleSync}
          />
        )}
        {activeSection === 'language' && <LanguageSection />}
        {activeSection === 'appearance' && (
          <AppearanceSection theme={theme} onThemeChange={setTheme} />
        )}
        {activeSection === 'shortcuts' && <ShortcutsSection />}
        {activeSection === 'memory' && <MemorySection />}
        {activeSection === 'agents' && <AgentsSection />}
        {activeSection === 'mcp' && <MCPServerView />}
        {activeSection === 'models' && (
          <ModelsSection
            models={models}
            defaultModel={defaultModel}
            onSetDefault={handleSetDefault}
            onSync={handleSync}
            syncing={syncing}
            providers={providers}
          />
        )}
        {activeSection === 'servers' && <ServersSection />}
        {activeSection === 'permissions' && <PermissionsSection />}
      </div>
    </div>
  )
}

function Row({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div
      className="flex items-center justify-between"
      style={{
        padding: '11px 0',
        borderBottom: '0.5px solid var(--border-subtle)',
      }}
    >
      <span style={{ fontSize: '13px', color: 'var(--text-secondary)' }}>{label}</span>
      <div>{children}</div>
    </div>
  )
}

function SectionTitle({ children }: { children: React.ReactNode }) {
  return (
    <h2
      style={{
        fontSize: '17px',
        fontWeight: 600,
        color: 'var(--text-primary)',
        marginBottom: '16px',
      }}
    >
      {children}
    </h2>
  )
}

function ProvidersSection(props: {
  providers: Provider[]
  editingKey: string | null
  newKey: string
  syncing: string | null
  onEdit: (id: string) => void
  onChangeKey: (k: string) => void
  onSaveKey: (id: string) => void
  onCancelEdit: () => void
  onDeleteKey: (id: string) => void
  onSync: (id: string) => void
}) {
  const { providers, editingKey, newKey, syncing, onEdit, onChangeKey, onSaveKey, onCancelEdit, onDeleteKey, onSync } = props
  const [filter, setFilter] = useState('')
  const filtered = providers.filter(p => p.name.toLowerCase().includes(filter.toLowerCase()))

  return (
    <div>
      <SectionTitle>Providers</SectionTitle>
      <p style={{ fontSize: '12px', color: 'var(--text-secondary)', marginBottom: '20px' }}>
        Bring your own API keys. Keys are stored locally and never sent to our servers.
      </p>

      <input
        type="text"
        value={filter}
        onChange={(e) => setFilter(e.target.value)}
        placeholder="Filter providers..."
        className="w-full focus:outline-none"
        style={{
          background: 'var(--bg-secondary)',
          border: '0.5px solid var(--border-subtle)',
          borderRadius: '6px',
          padding: '8px 12px',
          color: 'var(--text-primary)',
          fontSize: '12px',
          marginBottom: '16px',
          fontFamily: "'JetBrains Mono', monospace",
        }}
      />

      <div
        className="rounded-lg overflow-hidden"
        style={{
          background: 'var(--bg-secondary)',
          border: '0.5px solid var(--border-subtle)',
        }}
      >
        {filtered.map((p, i) => {
          const meta = PROVIDER_META[p.id] || { color: '#888' }
          const isEditing = editingKey === p.id
          return (
            <div
              key={p.id}
              style={{
                padding: '12px 16px',
                borderBottom: i < filtered.length - 1 ? '0.5px solid var(--border-subtle)' : 'none',
                display: 'flex',
                alignItems: 'center',
                gap: '12px',
              }}
            >
              <span
                className="rounded-full flex-shrink-0"
                style={{ width: '8px', height: '8px', background: meta.color }}
              />
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2 mb-1">
                  <span style={{ fontSize: '13px', fontWeight: 500, color: 'var(--text-primary)' }}>{p.name}</span>
                  <span
                    style={{
                      fontSize: '10px',
                      letterSpacing: '0.06em',
                      textTransform: 'uppercase',
                      color: 'var(--text-tertiary)',
                      fontFamily: "'JetBrains Mono', monospace",
                    }}
                  >
                    {p.kind}
                  </span>
                </div>
                <div
                  style={{
                    fontSize: '11px',
                    color: 'var(--text-tertiary)',
                    fontFamily: "'JetBrains Mono', monospace",
                  }}
                >
                  {p.has_api_key ? (
                    <span style={{ color: 'var(--green-text)' }}>key set</span>
                  ) : (
                    <span>no key</span>
                  )}
                  {p.models_count > 0 && <> &middot; {p.models_count} models</>}
                </div>
              </div>

              {isEditing ? (
                <div className="flex gap-2">
                  <input
                    type="password"
                    value={newKey}
                    onChange={(e) => onChangeKey(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.key === 'Enter') onSaveKey(p.id)
                      else if (e.key === 'Escape') onCancelEdit()
                    }}
                    placeholder="new api key"
                    autoFocus
                    className="focus:outline-none"
                    style={{
                      background: 'var(--bg-primary)',
                      border: '0.5px solid var(--border-subtle)',
                      borderRadius: '4px',
                      padding: '6px 10px',
                      color: 'var(--text-primary)',
                      fontSize: '11px',
                      fontFamily: "'JetBrains Mono', monospace",
                      width: '180px',
                    }}
                  />
                  <button
                    onClick={() => onSaveKey(p.id)}
                    className="rounded flex items-center justify-center"
                    style={{
                      background: 'var(--accent-muted)',
                      border: '0.5px solid var(--accent)',
                      color: 'var(--accent-text)',
                      padding: '6px 10px',
                      fontSize: '11px',
                      cursor: 'pointer',
                    }}
                  >
                    <Check className="size-3" />
                  </button>
                  <button
                    onClick={onCancelEdit}
                    className="rounded flex items-center justify-center"
                    style={{
                      background: 'transparent',
                      border: '0.5px solid var(--border-subtle)',
                      color: 'var(--text-secondary)',
                      padding: '6px 10px',
                      fontSize: '11px',
                      cursor: 'pointer',
                    }}
                  >
                    <X className="size-3" />
                  </button>
                </div>
              ) : (
                <div className="flex items-center gap-1">
                  <button
                    onClick={() => onEdit(p.id)}
                    className="rounded"
                    style={{
                      background: 'transparent',
                      border: '0.5px solid var(--border-subtle)',
                      color: 'var(--text-secondary)',
                      padding: '6px 10px',
                      fontSize: '11px',
                      cursor: 'pointer',
                    }}
                  >
                    {p.has_api_key ? 'Replace' : 'Add key'}
                  </button>
                  {p.has_api_key && (
                    <button
                      onClick={() => onDeleteKey(p.id)}
                      className="rounded"
                      style={{
                        background: 'transparent',
                        border: '0.5px solid var(--border-subtle)',
                        color: 'var(--red-text)',
                        padding: '6px',
                        cursor: 'pointer',
                      }}
                    >
                      <Trash2 className="size-3" />
                    </button>
                  )}
                  <button
                    onClick={() => onSync(p.id)}
                    disabled={syncing === p.id}
                    className="rounded"
                    style={{
                      background: 'transparent',
                      border: '0.5px solid var(--border-subtle)',
                      color: 'var(--text-secondary)',
                      padding: '6px',
                      cursor: 'pointer',
                    }}
                  >
                    {syncing === p.id ? (
                      <Loader2 className="size-3 animate-spin" />
                    ) : (
                      <RefreshCw className="size-3" />
                    )}
                  </button>
                </div>
              )}
            </div>
          )
        })}
        {filtered.length === 0 && (
          <div
            style={{
              padding: '32px',
              textAlign: 'center',
              color: 'var(--text-tertiary)',
              fontSize: '12px',
            }}
          >
            No providers match "{filter}"
          </div>
        )}
      </div>
    </div>
  )
}

function ModelsSection(props: {
  models: Model[]
  defaultModel: string | null
  onSetDefault: (id: string) => void
  onSync: (id: string) => void
  syncing: string | null
  providers: Provider[]
}) {
  const { models, defaultModel, onSetDefault, onSync, syncing, providers } = props
  const [filter, setFilter] = useState('')
  const filtered = models.filter(m =>
    m.name.toLowerCase().includes(filter.toLowerCase()) ||
    m.provider.toLowerCase().includes(filter.toLowerCase())
  )

  const providersWithModels = providers.filter(p => p.models_count > 0 || p.has_api_key || p.id === 'ollama')

  return (
    <div>
      <SectionTitle>Models</SectionTitle>
      <p style={{ fontSize: '12px', color: 'var(--text-secondary)', marginBottom: '20px' }}>
        Choose your default model. {models.length} models available.
      </p>

      {providersWithModels.length > 0 && (
        <div className="mb-4 flex flex-wrap gap-2">
          {providersWithModels.map(p => (
            <button
              key={p.id}
              onClick={() => onSync(p.id)}
              disabled={syncing === p.id}
              className="rounded"
              style={{
                background: 'var(--bg-tertiary)',
                border: '0.5px solid var(--border-subtle)',
                color: 'var(--text-secondary)',
                padding: '4px 10px',
                fontSize: '11px',
                cursor: 'pointer',
                display: 'flex',
                alignItems: 'center',
                gap: '6px',
              }}
            >
              {syncing === p.id ? <Loader2 className="size-3 animate-spin" /> : <RefreshCw className="size-3" />}
              Sync {p.name}
            </button>
          ))}
        </div>
      )}

      <input
        type="text"
        value={filter}
        onChange={(e) => setFilter(e.target.value)}
        placeholder="Filter models..."
        className="w-full focus:outline-none"
        style={{
          background: 'var(--bg-secondary)',
          border: '0.5px solid var(--border-subtle)',
          borderRadius: '6px',
          padding: '8px 12px',
          color: 'var(--text-primary)',
          fontSize: '12px',
          marginBottom: '16px',
          fontFamily: "'JetBrains Mono', monospace",
        }}
      />

      <div
        className="rounded-lg overflow-hidden"
        style={{
          background: 'var(--bg-secondary)',
          border: '0.5px solid var(--border-subtle)',
        }}
      >
        {filtered.slice(0, 50).map((m, i) => {
          const isDefault = defaultModel === m.id
          const meta = PROVIDER_META[m.provider] || { color: '#888' }
          return (
            <div
              key={m.id}
              style={{
                padding: '10px 16px',
                borderBottom: i < Math.min(filtered.length, 50) - 1 ? '0.5px solid var(--border-subtle)' : 'none',
                display: 'flex',
                alignItems: 'center',
                gap: '12px',
                cursor: 'pointer',
              }}
              onClick={() => onSetDefault(m.id)}
            >
              <span
                className="rounded-full flex-shrink-0"
                style={{ width: '6px', height: '6px', background: meta.color }}
              />
              <div className="flex-1 min-w-0">
                <div style={{ fontSize: '12px', color: 'var(--text-primary)' }}>{m.name}</div>
                <div
                  style={{
                    fontSize: '10px',
                    color: 'var(--text-tertiary)',
                    fontFamily: "'JetBrains Mono', monospace",
                  }}
                >
                  {m.provider}
                  {m.context_window && <> &middot; {(m.context_window / 1000).toFixed(0)}k ctx</>}
                  {m.supports_vision && <> &middot; vision</>}
                  {m.supports_tools && <> &middot; tools</>}
                  {m.is_free && <> &middot; free</>}
                  {m.is_local && <> &middot; local</>}
                </div>
              </div>
              {isDefault && (
                <span
                  style={{
                    background: 'var(--green-bg)',
                    color: 'var(--green-text)',
                    fontSize: '10px',
                    padding: '2px 8px',
                    borderRadius: '4px',
                    textTransform: 'uppercase',
                    letterSpacing: '0.06em',
                    fontWeight: 600,
                  }}
                >
                  Default
                </span>
              )}
            </div>
          )
        })}
        {filtered.length > 50 && (
          <div
            style={{
              padding: '12px 16px',
              fontSize: '11px',
              color: 'var(--text-tertiary)',
              textAlign: 'center',
            }}
          >
            +{filtered.length - 50} more
          </div>
        )}
        {filtered.length === 0 && (
          <div
            style={{
              padding: '32px',
              textAlign: 'center',
              color: 'var(--text-tertiary)',
              fontSize: '12px',
            }}
          >
            No models match "{filter}"
          </div>
        )}
      </div>
    </div>
  )
}

function AppearanceSection({ theme, onThemeChange }: { theme: string; onThemeChange: (id: string) => void }) {
  return (
    <div>
      <SectionTitle>Appearance</SectionTitle>
      <p style={{ fontSize: '12px', color: 'var(--text-secondary)', marginBottom: '20px' }}>
        Choose a theme. Changes apply instantly.
      </p>

      <div className="grid grid-cols-2 gap-3">
        {THEMES.map(t => {
          const isActive = theme === t.id
          return (
            <button
              key={t.id}
              onClick={() => onThemeChange(t.id)}
              className="text-left transition"
              style={{
                background: 'var(--bg-secondary)',
                border: isActive ? '0.5px solid var(--accent)' : '0.5px solid var(--border-subtle)',
                borderRadius: '8px',
                padding: '12px',
                cursor: 'pointer',
              }}
            >
              <div className="flex items-center gap-2 mb-2">
                <span style={{ fontSize: '12px', fontWeight: 500, color: 'var(--text-primary)' }}>{t.name}</span>
                {isActive && <Check className="size-3" style={{ color: 'var(--accent)' }} />}
              </div>
              <div
                className="rounded overflow-hidden"
                style={{
                  height: '60px',
                  background: t.bg,
                  border: '0.5px solid var(--border-subtle)',
                  display: 'flex',
                }}
              >
                <div style={{ width: '20px', background: t.side }} />
                <div className="flex-1 p-2 flex flex-col gap-1">
                  <div style={{ width: '40%', height: '6px', background: t.acc, borderRadius: '2px' }} />
                  <div style={{ width: '80%', height: '3px', background: t.txt, borderRadius: '1px', opacity: 0.5 }} />
                  <div style={{ width: '60%', height: '3px', background: t.txt, borderRadius: '1px', opacity: 0.3 }} />
                </div>
              </div>
              <div
                className="mt-2 flex gap-1"
                style={{ fontFamily: "'JetBrains Mono', monospace", fontSize: '9px', color: 'var(--text-tertiary)' }}
              >
                <span>{t.bg}</span>
              </div>
            </button>
          )
        })}
      </div>
    </div>
  )
}

function AboutSection() {
  const [appInfo, setAppInfo] = useState<{ version: string; platform: string; arch: string } | null>(null)

  useEffect(() => {
    setAppInfo({
      version: '0.1.0',
      platform: navigator.platform || 'unknown',
      arch: navigator.userAgent.includes('x86_64') ? 'x86_64' : 'unknown',
    })
  }, [])

  return (
    <div>
      <SectionTitle>General</SectionTitle>

      <div
        className="rounded-lg"
        style={{
          background: 'var(--bg-secondary)',
          border: '0.5px solid var(--border-subtle)',
          padding: '16px 20px',
        }}
      >
        <Row label="Auto-launch on startup">
          <input type="checkbox" disabled />
        </Row>
        <Row label="Check for updates">
          <input type="checkbox" defaultChecked disabled />
        </Row>
        <Row label="Send anonymous telemetry">
          <input type="checkbox" disabled />
        </Row>
        <Row label="Version">
          <span style={{ fontSize: '12px', fontFamily: "'JetBrains Mono', monospace", color: 'var(--text-primary)' }}>
            {appInfo?.version || '0.1.0'}
          </span>
        </Row>
        <Row label="Platform">
          <span style={{ fontSize: '12px', fontFamily: "'JetBrains Mono', monospace", color: 'var(--text-primary)' }}>
            {appInfo?.platform || 'unknown'} · {appInfo?.arch || '?'}
          </span>
        </Row>
        <Row label="Renderer">
          <span style={{ fontSize: '12px', fontFamily: "'JetBrains Mono', monospace", color: 'var(--text-primary)' }}>
            Tauri 2 + WebView
          </span>
        </Row>
        <Row label="Core">
          <span style={{ fontSize: '12px', fontFamily: "'JetBrains Mono', monospace", color: 'var(--text-primary)' }}>
            orion-core (in-process)
          </span>
        </Row>
      </div>

      <div
        className="mt-4 rounded-lg p-4"
        style={{
          background: 'var(--accent-muted)',
          border: '0.5px solid var(--accent)',
        }}
      >
        <div className="flex items-center gap-2 mb-2">
          <Plug className="size-4" style={{ color: 'var(--accent-text)' }} />
          <span style={{ fontSize: '13px', fontWeight: 500, color: 'var(--accent-text)' }}>
            Bring your own keys
          </span>
        </div>
        <p style={{ fontSize: '11px', color: 'var(--text-secondary)', lineHeight: 1.5 }}>
          ORION routes to any provider. Your API keys are stored locally in a SQLite database at
          <span style={{ fontFamily: "'JetBrains Mono', monospace", color: 'var(--accent-text)' }}> ~/.config/orion/catalog.db</span>
          {' '}and never leave your machine.
        </p>
      </div>
    </div>
  )
}

function GeneralSection() {
  return <AboutSection />
}

function LanguageSection() {
  const [lang, setLang] = useState(() => localStorage.getItem('orion-lang') ?? 'es')
  useEffect(() => {
    localStorage.setItem('orion-lang', lang)
  }, [lang])

  return (
    <div>
      <SectionTitle>Language</SectionTitle>
      <div
        className="rounded-lg"
        style={{ background: 'var(--bg-secondary)', border: '0.5px solid var(--border-subtle)', padding: '4px 8px' }}
      >
        {[
          { id: 'es', label: 'Español', hint: 'default' },
          { id: 'en', label: 'English' },
          { id: 'pt', label: 'Português' },
          { id: 'zh', label: '中文' },
        ].map(l => (
          <label
            key={l.id}
            className="flex items-center justify-between cursor-pointer"
            style={{ padding: '11px 12px', borderBottom: '0.5px solid var(--border-subtle)' }}
          >
            <div>
              <div style={{ fontSize: '13px', color: 'var(--text-primary)' }}>{l.label}</div>
              {l.hint && <div style={{ fontSize: '10px', color: 'var(--text-tertiary)', marginTop: 2 }}>{l.hint}</div>}
            </div>
            <input
              type="radio"
              name="lang"
              checked={lang === l.id}
              onChange={() => setLang(l.id)}
              style={{ accentColor: 'var(--accent)' }}
            />
          </label>
        ))}
      </div>
      <p className="mt-3" style={{ fontSize: '11px', color: 'var(--text-tertiary)' }}>
        Language changes UI strings only. LLM responses follow the system prompt regardless.
      </p>
    </div>
  )
}

function ShortcutsSection() {
  const shortcuts: Array<{ keys: string[]; desc: string }> = [
    { keys: ['Ctrl', ','], desc: 'Open Settings' },
    { keys: ['Esc'], desc: 'Close Settings' },
    { keys: ['Enter'], desc: 'Send message' },
    { keys: ['Shift', 'Enter'], desc: 'New line in input' },
    { keys: ['Ctrl', 'N'], desc: 'New session' },
    { keys: ['Ctrl', 'K'], desc: 'Search / command palette' },
  ]
  return (
    <div>
      <SectionTitle>Shortcuts</SectionTitle>
      <div
        className="rounded-lg"
        style={{ background: 'var(--bg-secondary)', border: '0.5px solid var(--border-subtle)' }}
      >
        {shortcuts.map((s, i) => (
          <div
            key={i}
            className="flex items-center justify-between"
            style={{ padding: '11px 14px', borderBottom: i < shortcuts.length - 1 ? '0.5px solid var(--border-subtle)' : 'none' }}
          >
            <span style={{ fontSize: '13px', color: 'var(--text-secondary)' }}>{s.desc}</span>
            <div className="flex items-center gap-1">
              {s.keys.map((k, j) => (
                <span
                  key={j}
                  style={{
                    fontFamily: "'JetBrains Mono', monospace",
                    fontSize: '11px',
                    color: 'var(--text-primary)',
                    background: 'var(--bg-tertiary)',
                    border: '0.5px solid var(--border-mid)',
                    borderRadius: 4,
                    padding: '2px 7px',
                  }}
                >
                  {k}
                </span>
              ))}
            </div>
          </div>
        ))}
      </div>
    </div>
  )
}

function MemorySection() {
  return (
    <PlaceholderSection
      title="Memory"
      description="Persistent memory store. Memories survive across sessions and are auto-injected into relevant conversations."
      bullets={[
        'Storage: ~/.config/orion/memory.db (SQLite)',
        'Auto-recall: enabled',
        'Token budget: 2000 per turn',
      ]}
    />
  )
}

function AgentsSection() {
  return (
    <PlaceholderSection
      title="Agents"
      description="Multi-agent system. Each agent has its own role, prompt and toolset."
      bullets={[
        'Default agent: orion-general',
        'Plan agent: read-only research mode',
        'Build agent: read-write execution mode',
      ]}
    />
  )
}

function ServersSection() {
  return (
    <PlaceholderSection
      title="Servers"
      description="Backend HTTP server (orion-server on port 7337) for external integrations."
      bullets={[
        'HTTP server: bundled (orion-server binary)',
        'Default port: 7337',
        'Status: managed by the desktop app',
      ]}
    />
  )
}

function PermissionsSection() {
  const [rules] = useState<Array<{ pattern: string; decision: 'allow' | 'deny' }>>([
    { pattern: 'shell.run:git status', decision: 'allow' },
    { pattern: 'shell.run:rm -rf *', decision: 'deny' },
  ])

  return (
    <div>
      <SectionTitle>Permissions</SectionTitle>
      <p style={{ fontSize: '12px', color: 'var(--text-secondary)', marginBottom: 12 }}>
        Tool calls matching these patterns are auto-approved or denied without prompting.
      </p>
      <div
        className="rounded-lg"
        style={{ background: 'var(--bg-secondary)', border: '0.5px solid var(--border-subtle)' }}
      >
        {rules.length === 0 ? (
          <div className="text-center" style={{ padding: '24px 16px', color: 'var(--text-tertiary)', fontSize: '12px' }}>
            No rules yet. Tool calls will prompt for approval by default.
          </div>
        ) : (
          rules.map((r, i) => (
            <div
              key={i}
              className="flex items-center justify-between"
              style={{ padding: '11px 14px', borderBottom: i < rules.length - 1 ? '0.5px solid var(--border-subtle)' : 'none' }}
            >
              <code style={{ fontFamily: "'JetBrains Mono', monospace", fontSize: '12px', color: 'var(--text-primary)' }}>
                {r.pattern}
              </code>
              <span
                style={{
                  fontFamily: "'JetBrains Mono', monospace",
                  fontSize: '10px',
                  letterSpacing: '0.08em',
                  textTransform: 'uppercase',
                  color: r.decision === 'allow' ? 'var(--green-text)' : 'var(--red-text)',
                  background: r.decision === 'allow' ? 'var(--green-bg)' : 'var(--red-bg)',
                  border: `0.5px solid ${r.decision === 'allow' ? 'var(--green)' : 'var(--red)'}`,
                  borderRadius: 20,
                  padding: '2px 8px',
                }}
              >
                {r.decision}
              </span>
            </div>
          ))
        )}
      </div>
    </div>
  )
}

function PlaceholderSection({
  title,
  description,
  bullets,
}: {
  title: string
  description: string
  bullets: string[]
}) {
  return (
    <div>
      <SectionTitle>{title}</SectionTitle>
      <p style={{ fontSize: '12px', color: 'var(--text-secondary)', marginBottom: 12, lineHeight: 1.6 }}>
        {description}
      </p>
      <div
        className="rounded-lg"
        style={{ background: 'var(--bg-secondary)', border: '0.5px solid var(--border-subtle)', padding: '14px 18px' }}
      >
        {bullets.map((b, i) => (
          <div
            key={i}
            style={{
              fontSize: '12px',
              color: 'var(--text-secondary)',
              padding: '6px 0',
              borderBottom: i < bullets.length - 1 ? '0.5px solid var(--border-subtle)' : 'none',
              fontFamily: "'JetBrains Mono', monospace",
            }}
          >
            {b}
          </div>
        ))}
      </div>
      <p className="mt-3" style={{ fontSize: '11px', color: 'var(--text-tertiary)' }}>
        UI shell ready · backend wiring pending.
      </p>
    </div>
  )
}
