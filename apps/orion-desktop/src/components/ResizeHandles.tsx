import { getCurrentWindow } from '@tauri-apps/api/window'

// A borderless window (decorations: false) gets no OS resize borders, so we
// render invisible grips on every edge/corner that start a native resize drag.
type Dir =
  | 'North' | 'South' | 'East' | 'West'
  | 'NorthEast' | 'NorthWest' | 'SouthEast' | 'SouthWest'

function isTauri(): boolean {
  if (typeof window === 'undefined') return false
  return Boolean((window as any).__TAURI_INTERNALS__ || (window as any).__TAURI__)
}

const grips: Array<{ dir: Dir; className: string }> = [
  { dir: 'North', className: 'rz rz-n' },
  { dir: 'South', className: 'rz rz-s' },
  { dir: 'East', className: 'rz rz-e' },
  { dir: 'West', className: 'rz rz-w' },
  { dir: 'NorthWest', className: 'rz rz-nw' },
  { dir: 'NorthEast', className: 'rz rz-ne' },
  { dir: 'SouthWest', className: 'rz rz-sw' },
  { dir: 'SouthEast', className: 'rz rz-se' },
]

export default function ResizeHandles() {
  if (!isTauri()) return null

  const start = (dir: Dir) => (e: React.MouseEvent) => {
    if (e.button !== 0) return
    e.preventDefault()
    // ResizeDirection accepts the PascalCase string directly.
    getCurrentWindow().startResizeDragging(dir as any).catch(() => {})
  }

  return (
    <>
      {grips.map((g) => (
        <div key={g.dir} className={g.className} onMouseDown={start(g.dir)} />
      ))}
    </>
  )
}
