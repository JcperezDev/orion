import { MessageSquare, Bot, Cloud, Plug, Settings as SettingsIcon } from 'lucide-react'
import type { LucideIcon } from 'lucide-react'

type View = 'chat' | 'models' | 'providers' | 'mcp' | 'settings'

interface Props {
  activeView: View
  onViewChange: (view: View) => void
}

const navItems: { id: View; label: string; icon: LucideIcon }[] = [
  { id: 'chat', label: 'Chat', icon: MessageSquare },
  { id: 'models', label: 'Models', icon: Bot },
  { id: 'providers', label: 'Providers', icon: Cloud },
  { id: 'mcp', label: 'MCP Hub', icon: Plug },
  { id: 'settings', label: 'Settings', icon: SettingsIcon },
]

export default function Sidebar({ activeView, onViewChange }: Props) {
  return (
    <aside className="w-56 bg-surface border-r border-border-subtle flex flex-col">
      <div className="p-4 border-b border-border-subtle">
        <h1 className="text-xl font-bold tracking-wider text-accent">ORION</h1>
        <p className="text-xs text-text-subtle mt-1">AI Model Router</p>
      </div>
      <nav className="flex-1 p-2">
        {navItems.map((item) => {
          const Icon = item.icon
          return (
            <button
              key={item.id}
              onClick={() => onViewChange(item.id)}
              className={`w-full flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm font-medium transition-colors ${
                activeView === item.id
                  ? 'bg-primary/20 text-primary'
                  : 'text-text-muted hover:text-text hover:bg-surface-raised'
              }`}
            >
              <Icon className="size-4" />
              {item.label}
            </button>
          )
        })}
      </nav>
      <div className="p-4 border-t border-border-subtle">
        <div className="text-xs text-text-subtle">
          <div className="flex items-center gap-2">
            <span className="w-2 h-2 rounded-full bg-success animate-pulse" />
            Connected
          </div>
        </div>
      </div>
    </aside>
  )
}
