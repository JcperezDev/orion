import { useEffect, useState } from 'react'
import { getCurrentWindow } from '@tauri-apps/api/window'

interface Props {
  onOpenSettings: () => void
  onOpenSearch?: () => void
}

const isMac = typeof navigator !== 'undefined' && /Mac/i.test(navigator.userAgent)

// Detect whether we're running inside a Tauri webview. Outside Tauri (e.g. a
// plain browser pointed at the vite dev server), the window API throws and
// `data-tauri-drag-region` is a no-op. Render the titlebar harmlessly either way.
function isTauri(): boolean {
  if (typeof window === 'undefined') return false
  // Tauri v2 exposes a global; older v1 used `__TAURI__`.
  return Boolean((window as any).__TAURI_INTERNALS__ || (window as any).__TAURI__)
}

function safeCall<T>(fn: () => Promise<T> | T, fallback: T): Promise<T> {
  return Promise.resolve()
    .then(fn)
    .catch(() => fallback)
}

function MinimizeIcon() {
  return (
    <svg width="10" height="10" viewBox="0 0 10 10" fill="none">
      <line x1="0" y1="5" x2="10" y2="5" stroke="currentColor" strokeWidth="1" />
    </svg>
  )
}

function MaximizeIcon({ maximized }: { maximized: boolean }) {
  if (maximized) {
    return (
      <svg width="10" height="10" viewBox="0 0 10 10" fill="none">
        <rect x="0" y="2.5" width="7" height="7" stroke="currentColor" strokeWidth="1" fill="none" />
        <line x1="2.5" y1="0" x2="10" y2="0" stroke="currentColor" strokeWidth="1" />
        <line x1="10" y1="0" x2="10" y2="7.5" stroke="currentColor" strokeWidth="1" />
      </svg>
    )
  }
  return (
    <svg width="10" height="10" viewBox="0 0 10 10" fill="none">
      <rect x="0" y="0" width="10" height="10" stroke="currentColor" strokeWidth="1" fill="none" />
    </svg>
  )
}

function CloseIcon() {
  return (
    <svg width="10" height="10" viewBox="0 0 10 10" fill="none">
      <line x1="0" y1="0" x2="10" y2="10" stroke="currentColor" strokeWidth="1" />
      <line x1="10" y1="0" x2="0" y2="10" stroke="currentColor" strokeWidth="1" />
    </svg>
  )
}

function SearchIcon() {
  return (
    <svg width="13" height="13" viewBox="0 0 13 13" fill="none">
      <circle cx="5.5" cy="5.5" r="4" stroke="currentColor" strokeWidth="1.2" fill="none" />
      <line x1="8.5" y1="8.5" x2="12" y2="12" stroke="currentColor" strokeWidth="1.2" />
    </svg>
  )
}

function SettingsIcon() {
  return (
    <svg width="13" height="13" viewBox="0 0 13 13" fill="none">
      <circle cx="6.5" cy="6.5" r="2" stroke="currentColor" strokeWidth="1.2" fill="none" />
      <path
        d="M6.5 0.5 L6.5 2.5 M6.5 10.5 L6.5 12.5 M0.5 6.5 L2.5 6.5 M10.5 6.5 L12.5 6.5 M2.2 2.2 L3.6 3.6 M9.4 9.4 L10.8 10.8 M10.8 2.2 L9.4 3.6 M3.6 9.4 L2.2 10.8"
        stroke="currentColor"
        strokeWidth="1.2"
        strokeLinecap="round"
      />
    </svg>
  )
}

export default function TitleBar({ onOpenSettings, onOpenSearch }: Props) {
  const [maximized, setMaximized] = useState(false)

  useEffect(() => {
    if (!isTauri()) return
    try {
      const win = getCurrentWindow()
      let unlisten: (() => void) | undefined
      win.isMaximized().then(setMaximized).catch(() => {})
      win.onResized(() => {
        win.isMaximized().then(setMaximized).catch(() => {})
      }).then(u => { unlisten = u }).catch(() => {})

      return () => {
        if (unlisten) unlisten()
      }
    } catch {
      // getCurrentWindow() can throw outside a Tauri webview; ignore.
    }
  }, [])

  const onMinimize = () => isTauri() && safeCall(() => getCurrentWindow().minimize(), undefined)
  const onToggleMaximize = () => isTauri() && safeCall(() => getCurrentWindow().toggleMaximize(), undefined)
  const onClose = () => isTauri() && safeCall(() => getCurrentWindow().close(), undefined)

  return (
    <div className="titlebar" data-tauri-drag-region>
      {isMac && (
        <div className="titlebar-buttons mac-buttons">
          <button className="titlebar-btn mac-close" onClick={onClose} aria-label="Close">
            <span className="mac-dot" style={{ background: '#ff5f57' }} />
          </button>
          <button className="titlebar-btn" onClick={onMinimize} aria-label="Minimize">
            <span className="mac-dot" style={{ background: '#febc2e' }} />
          </button>
          <button className="titlebar-btn" onClick={onToggleMaximize} aria-label="Maximize">
            <span className="mac-dot" style={{ background: '#28c840' }} />
          </button>
        </div>
      )}
      {!isMac && (
        <div className="titlebar-brand" data-tauri-drag-region>
          <img src="/orion.svg" className="titlebar-logo" alt="" draggable={false} />
        </div>
      )}
      <div className="titlebar-title" data-tauri-drag-region>ORION</div>
      <div className="titlebar-right">
        {onOpenSearch && (
          <button className="titlebar-icon" onClick={onOpenSearch} aria-label="Search">
            <SearchIcon />
          </button>
        )}
        <button className="titlebar-icon" onClick={onOpenSettings} aria-label="Settings">
          <SettingsIcon />
        </button>
        {!isMac && (
          <div className="titlebar-buttons">
            <button className="titlebar-btn" onClick={onMinimize} aria-label="Minimize">
              <MinimizeIcon />
            </button>
            <button className="titlebar-btn" onClick={onToggleMaximize} aria-label="Maximize">
              <MaximizeIcon maximized={maximized} />
            </button>
            <button className="titlebar-btn close" onClick={onClose} aria-label="Close">
              <CloseIcon />
            </button>
          </div>
        )}
      </div>
    </div>
  )
}
