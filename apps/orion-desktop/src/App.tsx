import { useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { homeDir } from '@tauri-apps/api/path'
import TitleBar from './components/TitleBar'
import Sidebar from './components/Sidebar'
import ChatView from './components/ChatView'
import OnboardingOverlay from './components/OnboardingOverlay'
import SettingsView from './components/SettingsView'
import { PermissionDialog } from './components/PermissionDialog'

type View = 'chat' | 'settings'

export interface Provider {
  id: string
  name: string
  has_api_key: boolean
  models_count: number
}

const isProviderConnected = (p: Provider) => p.has_api_key || p.id === 'ollama'

export default function App() {
  const [activeView, setActiveView] = useState<View>('chat')
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
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [activeView, showOnboarding])

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
      <TitleBar onOpenSettings={() => setActiveView('settings')} />
      <div className="app-body">
        <Sidebar
          onOpenSettings={() => setActiveView('settings')}
          workspaceName={workspaceName}
          workspacePath={workspacePath}
        />
        <main className="app-main">
          {activeView === 'settings' ? <SettingsView /> : <ChatView />}
        </main>
      </div>
      {showOnboarding && <OnboardingOverlay onClose={handleOnboardingClose} />}
      <PermissionDialog />
    </div>
  )
}
