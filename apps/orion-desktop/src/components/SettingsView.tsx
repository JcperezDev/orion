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
  Bot,
  Shield,
  Globe,
  Keyboard,
  Settings as SettingsIcon,
} from 'lucide-react'

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

interface ThemeDef { id: string; name: string; bg: string; side: string; acc: string; txt: string; brd: string; light?: boolean }

const THEMES: ThemeDef[] = [
  // ---- Signature / popular dark ----
  { id: 'orion-dark', name: 'Orion Dark', bg: '#0a0a0c', side: '#111116', acc: '#534AB7', txt: '#e2e0f0', brd: '#1e1e28' },
  { id: 'tokyonight', name: 'Tokyo Night', bg: '#1a1b26', side: '#16161e', acc: '#7aa2f7', txt: '#c0caf5', brd: '#414868' },
  { id: 'tokyonight-storm', name: 'Tokyo Night Storm', bg: '#24283b', side: '#1f2335', acc: '#7aa2f7', txt: '#c0caf5', brd: '#414868' },
  { id: 'catppuccin', name: 'Catppuccin Mocha', bg: '#1e1e2e', side: '#181825', acc: '#cba6f7', txt: '#cdd6f4', brd: '#313244' },
  { id: 'catppuccin-macchiato', name: 'Catppuccin Macchiato', bg: '#24273a', side: '#1e2030', acc: '#c6a0f6', txt: '#cad3f5', brd: '#363a4f' },
  { id: 'catppuccin-frappe', name: 'Catppuccin Frappé', bg: '#303446', side: '#292c3c', acc: '#ca9ee6', txt: '#c6d0f5', brd: '#414559' },
  { id: 'one-dark', name: 'One Dark', bg: '#282c34', side: '#21252b', acc: '#61afef', txt: '#abb2bf', brd: '#3e4451' },
  { id: 'dracula', name: 'Dracula', bg: '#282a36', side: '#21222c', acc: '#bd93f9', txt: '#f8f8f2', brd: '#44475a' },
  { id: 'nord', name: 'Nord', bg: '#2e3440', side: '#272c36', acc: '#88c0d0', txt: '#eceff4', brd: '#3b4252' },
  { id: 'gruvbox', name: 'Gruvbox', bg: '#282828', side: '#1d2021', acc: '#fabd2f', txt: '#ebdbb2', brd: '#3c3836' },
  { id: 'gruvbox-material', name: 'Gruvbox Material', bg: '#1d2021', side: '#282828', acc: '#d8a657', txt: '#d4be98', brd: '#3c3836' },
  { id: 'rose-pine', name: 'Rosé Pine', bg: '#191724', side: '#1f1d2e', acc: '#c4a7e7', txt: '#e0def4', brd: '#2a2739' },
  { id: 'rose-pine-moon', name: 'Rosé Pine Moon', bg: '#232136', side: '#2a273f', acc: '#c4a7e7', txt: '#e0def4', brd: '#393552' },
  { id: 'kanagawa', name: 'Kanagawa', bg: '#1f1f28', side: '#16161d', acc: '#7e9cd8', txt: '#dcd7ba', brd: '#2a2a37' },
  { id: 'everforest', name: 'Everforest', bg: '#2d353b', side: '#272e33', acc: '#a7c080', txt: '#d3c6aa', brd: '#3d484d' },
  { id: 'monokai', name: 'Monokai', bg: '#272822', side: '#1e1f1c', acc: '#f92672', txt: '#f8f8f2', brd: '#3e3d32' },
  { id: 'monokai-pro', name: 'Monokai Pro', bg: '#2d2a2e', side: '#221f22', acc: '#ffd866', txt: '#fcfcfa', brd: '#403e41' },
  { id: 'synthwave', name: 'Synthwave ’84', bg: '#262335', side: '#1d1927', acc: '#ff7edb', txt: '#ffffff', brd: '#3b3557' },
  // ---- More dark ----
  { id: 'solarized-dark', name: 'Solarized Dark', bg: '#002b36', side: '#073642', acc: '#268bd2', txt: '#93a1a1', brd: '#0a4250' },
  { id: 'material-ocean', name: 'Material Ocean', bg: '#0f111a', side: '#0b0d14', acc: '#84ffff', txt: '#a6accd', brd: '#1f2233' },
  { id: 'material-palenight', name: 'Material Palenight', bg: '#292d3e', side: '#232635', acc: '#c792ea', txt: '#a6accd', brd: '#3a3f58' },
  { id: 'ayu-dark', name: 'Ayu Dark', bg: '#0b0e14', side: '#0d1017', acc: '#ffb454', txt: '#bfbdb6', brd: '#1b1f2b' },
  { id: 'ayu-mirage', name: 'Ayu Mirage', bg: '#1f2430', side: '#191e2a', acc: '#ffcc66', txt: '#cbccc6', brd: '#2d3343' },
  { id: 'night-owl', name: 'Night Owl', bg: '#011627', side: '#001122', acc: '#82aaff', txt: '#d6deeb', brd: '#1d3b53' },
  { id: 'cobalt2', name: 'Cobalt2', bg: '#193549', side: '#15293a', acc: '#ffc600', txt: '#ffffff', brd: '#234d6b' },
  { id: 'oceanic-next', name: 'Oceanic Next', bg: '#1b2b34', side: '#16242b', acc: '#6699cc', txt: '#cdd3de', brd: '#2b3b44' },
  { id: 'horizon', name: 'Horizon', bg: '#1c1e26', side: '#16181e', acc: '#e95678', txt: '#d5d8da', brd: '#2e303e' },
  { id: 'nightfox', name: 'Nightfox', bg: '#192330', side: '#131a24', acc: '#719cd6', txt: '#cdcecf', brd: '#29394f' },
  { id: 'carbonfox', name: 'Carbonfox', bg: '#161616', side: '#0c0c0c', acc: '#78a9ff', txt: '#f2f4f8', brd: '#2a2a2a' },
  { id: 'oxocarbon', name: 'Oxocarbon', bg: '#161616', side: '#0c0c0c', acc: '#ee5396', txt: '#f2f4f8', brd: '#262626' },
  { id: 'github-dark', name: 'GitHub Dark', bg: '#0d1117', side: '#010409', acc: '#58a6ff', txt: '#c9d1d9', brd: '#21262d' },
  { id: 'vesper', name: 'Vesper', bg: '#101010', side: '#0a0a0a', acc: '#ffc799', txt: '#ffffff', brd: '#232323' },
  { id: 'poimandres', name: 'Poimandres', bg: '#1b1e28', side: '#171922', acc: '#5de4c7', txt: '#e4f0fb', brd: '#2a2e3d' },
  { id: 'tomorrow-night', name: 'Tomorrow Night', bg: '#1d1f21', side: '#161719', acc: '#81a2be', txt: '#c5c8c6', brd: '#373b41' },
  { id: 'panda', name: 'Panda', bg: '#292a2b', side: '#1f2021', acc: '#ff75b5', txt: '#e6e6e6', brd: '#3a3b3c' },
  { id: 'aura', name: 'Aura', bg: '#21202e', side: '#1c1b25', acc: '#a277ff', txt: '#edecee', brd: '#2d2b3a' },
  { id: 'moonfly', name: 'Moonfly', bg: '#080808', side: '#0c0c0c', acc: '#80a0ff', txt: '#bdbdbd', brd: '#303030' },
  { id: 'zenburn', name: 'Zenburn', bg: '#3f3f3f', side: '#383838', acc: '#dca3a3', txt: '#dcdccc', brd: '#4f4f4f' },
  { id: 'espresso', name: 'Espresso', bg: '#2a211c', side: '#231a15', acc: '#d8b48b', txt: '#e6d9c8', brd: '#3d3128' },
  { id: 'tokyo-night-moon', name: 'Tokyo Night Moon', bg: '#222436', side: '#1e2030', acc: '#82aaff', txt: '#c8d3f5', brd: '#3b4261' },
  { id: 'noctis', name: 'Noctis', bg: '#052529', side: '#04181c', acc: '#49d6e0', txt: '#c5cdd3', brd: '#0c3a40' },
  // ---- Light ----
  { id: 'orion-light', name: 'Orion Light', bg: '#ffffff', side: '#f4f4f7', acc: '#534AB7', txt: '#1a1a2e', brd: '#e2e2ea', light: true },
  { id: 'github-light', name: 'GitHub Light', bg: '#ffffff', side: '#f6f8fa', acc: '#0969da', txt: '#24292f', brd: '#d0d7de', light: true },
  { id: 'solarized-light', name: 'Solarized Light', bg: '#fdf6e3', side: '#eee8d5', acc: '#268bd2', txt: '#586e75', brd: '#ddd6c1', light: true },
  { id: 'catppuccin-latte', name: 'Catppuccin Latte', bg: '#eff1f5', side: '#e6e9ef', acc: '#8839ef', txt: '#4c4f69', brd: '#ccd0da', light: true },
  { id: 'rose-pine-dawn', name: 'Rosé Pine Dawn', bg: '#faf4ed', side: '#fffaf3', acc: '#907aa9', txt: '#575279', brd: '#dfdad9', light: true },
  { id: 'one-light', name: 'One Light', bg: '#fafafa', side: '#eaeaeb', acc: '#4078f2', txt: '#383a42', brd: '#dbdbdc', light: true },
  { id: 'ayu-light', name: 'Ayu Light', bg: '#fcfcfc', side: '#f3f4f5', acc: '#fa8d3e', txt: '#5c6166', brd: '#e7e8e9', light: true },
  { id: 'gruvbox-light', name: 'Gruvbox Light', bg: '#fbf1c7', side: '#f2e5bc', acc: '#b57614', txt: '#3c3836', brd: '#e3d8ac', light: true },
]

// --- Theme application with derived variables (so light themes read well too) ---
function hexToRgb(h: string): [number, number, number] {
  const s = h.replace('#', '')
  const n = parseInt(s.length === 3 ? s.split('').map(c => c + c).join('') : s, 16)
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255]
}
function rgbToHex(r: number, g: number, b: number): string {
  const c = (x: number) => Math.max(0, Math.min(255, Math.round(x))).toString(16).padStart(2, '0')
  return `#${c(r)}${c(g)}${c(b)}`
}
function mix(a: string, b: string, t: number): string {
  const [r1, g1, b1] = hexToRgb(a)
  const [r2, g2, b2] = hexToRgb(b)
  return rgbToHex(r1 + (r2 - r1) * t, g1 + (g2 - g1) * t, b1 + (b2 - b1) * t)
}
function applyTheme(t: ThemeDef) {
  const root = document.documentElement
  const set = (k: string, v: string) => root.style.setProperty(k, v)
  set('--bg-primary', t.bg)
  set('--bg-secondary', t.side)
  set('--bg-tertiary', mix(t.side, t.txt, 0.06))
  set('--text-primary', t.txt)
  set('--text-secondary', mix(t.txt, t.bg, 0.32))
  set('--text-tertiary', mix(t.txt, t.bg, 0.55))
  set('--border-subtle', t.brd)
  set('--border-mid', mix(t.brd, t.txt, 0.22))
  set('--border-strong', mix(t.brd, t.txt, 0.42))
  set('--accent', t.acc)
  set('--accent-text', t.light ? mix(t.acc, '#000000', 0.1) : t.acc)
  set('--accent-muted', mix(t.acc, t.bg, 0.82))
}

type Section =
  | 'general'
  | 'providers'
  | 'language'
  | 'appearance'
  | 'shortcuts'
  | 'agents'
  | 'models'
  | 'permissions'

type SectionGroup = 'GENERAL' | 'INTERFACE' | 'TOOLS' | 'SYSTEM'

interface SectionDef {
  id: Section
  label: string
  icon: any
  group: SectionGroup
}

export default function SettingsView({ onClose }: { onClose?: () => void }) {
  const [activeSection, setActiveSection] = useState<Section>(() => {
    if (typeof window === 'undefined') return 'providers'
    const m = window.location.hash.match(/^#settings\/([a-z]+)/)
    if (m && ['general','providers','language','appearance','shortcuts','agents','models','permissions'].includes(m[1])) {
      return m[1] as Section
    }
    return 'providers'
  })
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
    if (t) applyTheme(t)
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
    { id: 'models',      label: 'Models',      icon: Cpu,          group: 'TOOLS' },
    { id: 'agents',      label: 'Agents',      icon: Bot,          group: 'TOOLS' },
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
        <button
          onClick={() => onClose?.()}
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 6,
            width: '100%',
            padding: '7px 10px',
            marginBottom: 10,
            border: '0.5px solid var(--border-subtle)',
            borderRadius: 6,
            background: 'transparent',
            color: 'var(--text-secondary)',
            fontSize: '12px',
            cursor: 'pointer',
          }}
          onMouseEnter={e => { (e.currentTarget.style.background = 'var(--bg-tertiary)'); (e.currentTarget.style.color = 'var(--text-primary)') }}
          onMouseLeave={e => { (e.currentTarget.style.background = 'transparent'); (e.currentTarget.style.color = 'var(--text-secondary)') }}
          title="Back to chat (Esc)"
        >
          <span style={{ fontSize: 14, lineHeight: 1 }}>←</span>
          <span>Back to chat</span>
        </button>
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
        {activeSection === 'agents' && <AgentsSection />}
        {activeSection === 'models' && (
          <ModelsSection
            models={models}
            defaultModel={defaultModel}
            onSetDefault={handleSetDefault}
            onSync={handleSync}
            syncing={syncing}
          />
        )}
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
}) {
  const { models, defaultModel, onSetDefault, onSync, syncing } = props
  const [filter, setFilter] = useState('')
  const filtered = models.filter(m =>
    m.name.toLowerCase().includes(filter.toLowerCase()) ||
    m.provider.toLowerCase().includes(filter.toLowerCase())
  )

  const syncingAny = syncing !== null

  return (
    <div>
      <SectionTitle>Models</SectionTitle>
      <div className="flex items-center justify-between" style={{ marginBottom: 16, gap: 12 }}>
        <p style={{ fontSize: '12px', color: 'var(--text-secondary)' }}>
          Choose your default model · {models.length} available
        </p>
        <button
          onClick={() => onSync('openrouter')}
          disabled={syncingAny}
          className="rounded"
          style={{
            flexShrink: 0,
            background: 'var(--bg-tertiary)',
            border: '0.5px solid var(--border-subtle)',
            color: 'var(--text-secondary)',
            padding: '5px 12px',
            fontSize: '11px',
            cursor: syncingAny ? 'default' : 'pointer',
            display: 'flex',
            alignItems: 'center',
            gap: '6px',
          }}
        >
          {syncingAny ? <Loader2 className="size-3 animate-spin" /> : <RefreshCw className="size-3" />}
          Sync models
        </button>
      </div>

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
        <Row label="Version">
          <span style={{ fontSize: '12px', fontFamily: "'JetBrains Mono', monospace", color: 'var(--text-primary)' }}>
            0.1.1
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
            Your keys stay on your machine
          </span>
        </div>
        <p style={{ fontSize: '11px', color: 'var(--text-secondary)', lineHeight: 1.5 }}>
          Bring your own provider keys. They are stored securely in your operating system's
          keyring and never leave your device. No telemetry, no tracking.
        </p>
      </div>
    </div>
  )
}

function GeneralSection() {
  return <AboutSection />
}

function LanguageSection() {
  const [lang, setLang] = useState(() => localStorage.getItem('orion-lang') ?? 'en')
  useEffect(() => {
    localStorage.setItem('orion-lang', lang)
  }, [lang])

  return (
    <div>
      <SectionTitle>Language</SectionTitle>
      <p style={{ fontSize: '12px', color: 'var(--text-secondary)', marginBottom: 12, lineHeight: 1.6 }}>
        The interface is currently English-only. Your preference is saved for when UI translations land.
      </p>
      <div
        className="rounded-lg"
        style={{ background: 'var(--bg-secondary)', border: '0.5px solid var(--border-subtle)', padding: '4px 8px' }}
      >
        {[
          { id: 'en', label: 'English', hint: 'default' },
          { id: 'es', label: 'Español' },
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
    { keys: ['Ctrl', 'K'], desc: 'Quick model switch' },
    { keys: ['Tab'], desc: 'Autocomplete a slash command' },
    { keys: ['Esc'], desc: 'Close a dialog / picker' },
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

interface AgentSpecView {
  id: string
  name: string
  mode: string
  description: string
  allowed_tools: string[]
  denied_tools: string[]
  color?: string | null
}

function AgentsSection() {
  const [agents, setAgents] = useState<AgentSpecView[]>([])

  useEffect(() => {
    invoke<AgentSpecView[]>('list_agents').then(setAgents).catch(() => {})
  }, [])

  const capability = (a: AgentSpecView) =>
    ['write', 'edit', 'apply_patch', 'bash'].some(t => a.denied_tools.includes(t)) ? 'Read-only' : 'Read-write'

  return (
    <div>
      <SectionTitle>Agents</SectionTitle>
      <p style={{ fontSize: '12px', color: 'var(--text-secondary)', marginBottom: 14, lineHeight: 1.6 }}>
        Switch the primary agent with the <strong>Build / Plan / Agent</strong> buttons in the chat.
        Subagents run automatically when a task needs them.
      </p>
      <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
        {agents.map(a => {
          const ro = capability(a) === 'Read-only'
          return (
            <div key={a.id} className="rounded-lg" style={{ background: 'var(--bg-secondary)', border: '0.5px solid var(--border-subtle)', padding: '12px 14px' }}>
              <div className="flex items-center" style={{ gap: 8, marginBottom: 4 }}>
                <span style={{ width: 8, height: 8, borderRadius: '50%', background: a.color || 'var(--accent)', flexShrink: 0 }} />
                <span style={{ fontSize: 13, fontWeight: 600, color: 'var(--text-primary)' }}>{a.name}</span>
                <span style={{ fontSize: 10, letterSpacing: '0.05em', textTransform: 'uppercase', color: ro ? 'var(--text-secondary)' : 'var(--green-text)', background: ro ? 'var(--bg-tertiary)' : 'var(--green-bg)', border: `0.5px solid ${ro ? 'var(--border-subtle)' : 'var(--green)'}`, borderRadius: 5, padding: '1px 7px' }}>{capability(a)}</span>
                <span style={{ fontSize: 10, letterSpacing: '0.05em', textTransform: 'uppercase', color: 'var(--text-tertiary)', marginLeft: 'auto' }}>{a.mode}</span>
              </div>
              <div style={{ fontSize: 12, color: 'var(--text-secondary)', lineHeight: 1.5 }}>{a.description}</div>
            </div>
          )
        })}
      </div>
    </div>
  )
}

interface PermRule { tool: string; pattern: string; action: string; learned: boolean }
interface PermDefault { tool: string; action: string }

function PermissionsSection() {
  const [fullAccess, setFullAccess] = useState(false)
  const [loadingFa, setLoadingFa] = useState(true)
  const [defaults, setDefaults] = useState<PermDefault[]>([])
  const [rules, setRules] = useState<PermRule[]>([])
  // new-rule form
  const [newTool, setNewTool] = useState('bash')
  const [newPattern, setNewPattern] = useState('')
  const [newAction, setNewAction] = useState<'allow' | 'ask' | 'deny'>('allow')

  async function reload() {
    try {
      const p = await invoke<{ defaults: PermDefault[]; rules: PermRule[] }>('get_permissions')
      setDefaults(p.defaults)
      setRules(p.rules)
    } catch {/* ignore */}
  }

  useEffect(() => {
    invoke<boolean>('get_full_access')
      .then((v) => setFullAccess(v))
      .catch(() => {})
      .finally(() => setLoadingFa(false))
    reload()
  }, [])

  async function addRule() {
    const pattern = newPattern.trim()
    if (!pattern) return
    try {
      await invoke('add_permission_rule', { tool: newTool, pattern, action: newAction })
      setNewPattern('')
      reload()
    } catch (e) { console.error('add rule failed:', e) }
  }

  async function removeRule(tool: string, pattern: string) {
    try {
      await invoke('remove_permission_rule', { tool, pattern })
      reload()
    } catch (e) { console.error('remove rule failed:', e) }
  }

  const actionColor = (a: string) =>
    a === 'allow' ? 'var(--green-text)' : a === 'deny' ? 'var(--red-text)' : 'var(--text-secondary)'
  const actionBg = (a: string) =>
    a === 'allow' ? 'var(--green-bg)' : a === 'deny' ? 'var(--red-bg)' : 'var(--bg-tertiary)'
  const actionBorder = (a: string) =>
    a === 'allow' ? 'var(--green)' : a === 'deny' ? 'var(--red)' : 'var(--border-subtle)'

  const toggleFullAccess = async () => {
    const next = !fullAccess
    setFullAccess(next)
    try {
      await invoke('set_full_access', { enabled: next })
    } catch {
      setFullAccess(!next) // revert on failure
    }
  }

  return (
    <div>
      <SectionTitle>Permissions</SectionTitle>

      {/* Master "full access" switch */}
      <div
        className="rounded-lg flex items-center justify-between"
        style={{
          background: fullAccess ? 'var(--red-bg)' : 'var(--bg-secondary)',
          border: `0.5px solid ${fullAccess ? 'var(--red)' : 'var(--border-subtle)'}`,
          padding: '14px 16px',
          marginBottom: 16,
        }}
      >
        <div style={{ marginRight: 16 }}>
          <div style={{ fontSize: '13px', fontWeight: 600, color: 'var(--text-primary)', marginBottom: 4 }}>
            Full access mode
          </div>
          <div style={{ fontSize: '11px', color: fullAccess ? 'var(--red-text)' : 'var(--text-secondary)', lineHeight: 1.5 }}>
            {fullAccess
              ? '⚠ Every tool runs with no prompts — rm, curl, and writes to /etc, /home, /tmp are all allowed.'
              : 'Off: the Trust Engine auto-allows safe, reversible actions and only asks before risky ones.'}
          </div>
        </div>
        <button
          role="switch"
          aria-checked={fullAccess}
          aria-label="Toggle full access mode"
          disabled={loadingFa}
          onClick={toggleFullAccess}
          style={{
            flexShrink: 0,
            width: 44,
            height: 26,
            borderRadius: 20,
            border: 'none',
            cursor: loadingFa ? 'default' : 'pointer',
            background: fullAccess ? 'var(--red)' : 'var(--border-strong, #555)',
            position: 'relative',
            transition: 'background 0.15s ease',
          }}
        >
          <span
            style={{
              position: 'absolute',
              top: 3,
              left: fullAccess ? 21 : 3,
              width: 20,
              height: 20,
              borderRadius: '50%',
              background: '#fff',
              transition: 'left 0.15s ease',
            }}
          />
        </button>
      </div>

      {/* Per-tool defaults (Trust Engine baseline) */}
      <div style={{ fontSize: '11px', letterSpacing: '0.08em', textTransform: 'uppercase', color: 'var(--text-tertiary)', fontWeight: 600, margin: '4px 0 8px' }}>
        Default policy per tool
      </div>
      <div className="rounded-lg" style={{ background: 'var(--bg-secondary)', border: '0.5px solid var(--border-subtle)', display: 'flex', flexWrap: 'wrap', gap: 6, padding: 12, marginBottom: 18 }}>
        {defaults.length === 0 && <span style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>Loading…</span>}
        {defaults.map(d => (
          <span key={d.tool} style={{ fontSize: 11, fontFamily: "'JetBrains Mono', monospace", display: 'inline-flex', gap: 6, alignItems: 'center', padding: '3px 8px', borderRadius: 6, background: 'var(--bg-tertiary)', border: '0.5px solid var(--border-subtle)' }}>
            <span style={{ color: 'var(--text-primary)' }}>{d.tool}</span>
            <span style={{ color: actionColor(d.action) }}>{d.action}</span>
          </span>
        ))}
      </div>

      {/* Custom rules (config + learned "always allow") */}
      <div style={{ fontSize: '11px', letterSpacing: '0.08em', textTransform: 'uppercase', color: 'var(--text-tertiary)', fontWeight: 600, margin: '4px 0 8px' }}>
        Custom rules
      </div>
      <p style={{ fontSize: '12px', color: 'var(--text-secondary)', marginBottom: 10 }}>
        Glob patterns auto-approved or denied without prompting (last match wins). For <code style={{ fontFamily: "'JetBrains Mono', monospace" }}>bash</code>, the pattern matches the command, e.g. <code style={{ fontFamily: "'JetBrains Mono', monospace" }}>git status*</code>.
      </p>

      {/* Add-rule form */}
      <div className="flex items-center" style={{ gap: 8, marginBottom: 10 }}>
        <select value={newTool} onChange={e => setNewTool(e.target.value)} className="rounded-lg" style={{ appearance: 'none', WebkitAppearance: 'none', background: 'var(--bg-secondary)', border: '0.5px solid var(--border-subtle)', color: 'var(--text-primary)', fontSize: 12, padding: '7px 22px 7px 8px' }}>
          {(defaults.length ? defaults.map(d => d.tool) : ['bash', 'read', 'write', 'edit', 'grep', 'glob', 'webfetch', 'websearch']).map(t => <option key={t} value={t} style={{ background: 'var(--bg-secondary)', color: 'var(--text-primary)' }}>{t}</option>)}
        </select>
        <input
          value={newPattern}
          onChange={e => setNewPattern(e.target.value)}
          onKeyDown={e => { if (e.key === 'Enter') addRule() }}
          placeholder="pattern, e.g. git status*"
          className="rounded-lg"
          style={{ flex: 1, background: 'var(--bg-secondary)', border: '0.5px solid var(--border-subtle)', color: 'var(--text-primary)', fontSize: 12, padding: '7px 10px', fontFamily: "'JetBrains Mono', monospace" }}
        />
        <select value={newAction} onChange={e => setNewAction(e.target.value as 'allow' | 'ask' | 'deny')} className="rounded-lg" style={{ appearance: 'none', WebkitAppearance: 'none', background: 'var(--bg-secondary)', border: '0.5px solid var(--border-subtle)', color: actionColor(newAction), fontSize: 12, padding: '7px 22px 7px 8px' }}>
          <option value="allow" style={{ background: 'var(--bg-secondary)', color: 'var(--text-primary)' }}>allow</option>
          <option value="ask" style={{ background: 'var(--bg-secondary)', color: 'var(--text-primary)' }}>ask</option>
          <option value="deny" style={{ background: 'var(--bg-secondary)', color: 'var(--text-primary)' }}>deny</option>
        </select>
        <button onClick={addRule} disabled={!newPattern.trim()} className="rounded-lg" style={{ border: 'none', background: 'var(--accent)', color: '#fff', fontSize: 12, padding: '7px 14px', cursor: newPattern.trim() ? 'pointer' : 'default', opacity: newPattern.trim() ? 1 : 0.5 }}>Add</button>
      </div>

      <div className="rounded-lg" style={{ background: 'var(--bg-secondary)', border: '0.5px solid var(--border-subtle)' }}>
        {rules.length === 0 ? (
          <div className="text-center" style={{ padding: '20px 16px', color: 'var(--text-tertiary)', fontSize: '12px' }}>
            No custom rules. The Trust Engine decides by risk/reversibility.
          </div>
        ) : (
          rules.map((r, i) => (
            <div key={`${r.tool}:${r.pattern}`} className="flex items-center justify-between" style={{ padding: '10px 14px', gap: 10, borderBottom: i < rules.length - 1 ? '0.5px solid var(--border-subtle)' : 'none' }}>
              <code style={{ fontFamily: "'JetBrains Mono', monospace", fontSize: '12px', color: 'var(--text-primary)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                <span style={{ color: 'var(--text-tertiary)' }}>{r.tool}:</span>{r.pattern}
                {r.learned && <span style={{ marginLeft: 8, fontSize: 9, color: 'var(--text-tertiary)', border: '0.5px solid var(--border-subtle)', borderRadius: 4, padding: '1px 5px' }}>learned</span>}
              </code>
              <div className="flex items-center" style={{ gap: 8, flexShrink: 0 }}>
                <span style={{ fontFamily: "'JetBrains Mono', monospace", fontSize: '10px', letterSpacing: '0.08em', textTransform: 'uppercase', color: actionColor(r.action), background: actionBg(r.action), border: `0.5px solid ${actionBorder(r.action)}`, borderRadius: 20, padding: '2px 8px' }}>
                  {r.action}
                </span>
                <button onClick={() => removeRule(r.tool, r.pattern)} aria-label="Delete rule" title="Delete rule" style={{ border: 'none', background: 'transparent', color: 'var(--text-tertiary)', cursor: 'pointer', fontSize: 13, padding: '2px 4px' }}>✕</button>
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  )
}

