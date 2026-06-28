import { useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { homeDir } from '@tauri-apps/api/path'
import TitleBar from './components/TitleBar'
import Sidebar from './components/Sidebar'
import ChatView from './components/ChatView'
import OnboardingOverlay from './components/OnboardingOverlay'
import SettingsView from './components/SettingsView'
import { PermissionDialog } from './components/PermissionDialog'
import ResizeHandles from './components/ResizeHandles'

type View = 'chat' | 'settings'

export interface Provider {
  id: string
  name: string
  has_api_key: boolean
  models_count: number
}

const isProviderConnected = (p: Provider) => p.has_api_key || p.id === 'ollama'

export default function App() {
  const viewFromHash = (): View => {
    if (typeof window === 'undefined') return 'chat'
    const h = window.location.hash.replace('#', '')
    if (h.startsWith('settings/')) return 'settings'
    if (h === 'settings') return 'settings'
    return 'chat'
  }
  const [activeView, setActiveView] = useState<View>(viewFromHash())
  const [showOnboarding, setShowOnboarding] = useState(false)
  const [workspacePath, setWorkspacePath] = useState<string>('')
  const [workspaceName, setWorkspaceName] = useState<string>('')

  useEffect(() => {
    checkOnboarding()
  }, [])

  useEffect(() => {
    homeDir()
      .then(p => {
        setWorkspacePath(p)
        const segments = p.split('/').filter(Boolean)
        setWorkspaceName(segments[segments.length - 1] || 'workspace')
      })
      .catch(() => {
        setWorkspacePath('~')
        setWorkspaceName('workspace')
      })
  }, [])

  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if (e.key === 'Escape') {
        if (showOnboarding) return // handled by overlay
        if (activeView === 'settings') setActiveView('chat')
      } else if (e.key === ',' && (e.ctrlKey || e.metaKey)) {
        e.preventDefault()
        setActiveView(v => v === 'settings' ? 'chat' : 'settings')
      } else if ((e.key === 'n' || e.key === 'N') && (e.ctrlKey || e.metaKey)) {
        e.preventDefault()
        setActiveView('chat')
        window.dispatchEvent(new CustomEvent('orion:new-session'))
      } else if ((e.key === 'k' || e.key === 'K') && (e.ctrlKey || e.metaKey)) {
        e.preventDefault()
        setActiveView('chat')
        window.dispatchEvent(new CustomEvent('orion:open-model-picker'))
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [activeView, showOnboarding])

  // Open Settings on request (e.g. "Connect" from the model picker).
  useEffect(() => {
    const onOpen = () => setActiveView('settings')
    window.addEventListener('orion:open-settings', onOpen)
    return () => window.removeEventListener('orion:open-settings', onOpen)
  }, [])

  // Select-to-copy: copy any selected text to the clipboard automatically.
  useEffect(() => {
    const onMouseUp = () => {
      const sel = window.getSelection()?.toString() ?? ''
      if (sel.trim().length > 0) {
        navigator.clipboard?.writeText(sel).catch(() => {})
      }
    }
    document.addEventListener('mouseup', onMouseUp)
    return () => document.removeEventListener('mouseup', onMouseUp)
  }, [])

  async function checkOnboarding() {
    try {
      const connected = await invoke<Provider[]>('get_connected_providers')
      const hasConnected = connected.some(p => isProviderConnected(p))
      setShowOnboarding(!hasConnected)
    } catch (e) {
      console.error('Failed to check onboarding:', e)
    }
  }

  const handleOnboardingClose = async () => {
    setShowOnboarding(false)
    try {
      const connected = await invoke<Provider[]>('get_connected_providers')
      if (connected.length === 0) {
        // User closed without connecting — keep overlay closed but allow re-opening
      }
    } catch {}
  }

  return (
    <div className="app-shell">
      <ResizeHandles />
      <TitleBar />
      <div className="app-body">
        <Sidebar
          onOpenSettings={() => setActiveView('settings')}
          workspaceName={workspaceName}
          workspacePath={workspacePath}
        />
        <main className="app-main">
          {activeView === 'settings' ? <SettingsView onClose={() => setActiveView('chat')} /> : <ChatView />}
        </main>
      </div>
      {showOnboarding && <OnboardingOverlay onClose={handleOnboardingClose} />}
      <PermissionDialog />
    </div>
  )
}
