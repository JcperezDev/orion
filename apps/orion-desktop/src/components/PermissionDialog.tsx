import { useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

interface ApprovalRequest {
  id: number
  tool: string
  action: string
  pattern?: string | null
  arguments: Record<string, unknown>
}

export function PermissionDialog() {
  const [request, setRequest] = useState<ApprovalRequest | null>(null)

  useEffect(() => {
    const unlisten = listen<ApprovalRequest>('orion://approval_request', (e) => {
      setRequest(e.payload)
    })
    return () => {
      unlisten.then((fn) => fn())
    }
  }, [])

  if (!request) return null

  const submit = (decision: 'allow' | 'allow_always' | 'deny') => {
    invoke('submit_approval', { id: request.id, decision }).catch(() => {})
    setRequest(null)
  }

  const argsJson = (() => {
    try {
      return JSON.stringify(request.arguments, null, 2)
    } catch {
      return String(request.arguments)
    }
  })()

  return (
    <div className="permission-dialog-backdrop" role="dialog" aria-modal="true">
      <div className="permission-dialog">
        <div className="permission-dialog-header">
          <span className="permission-tool-badge">{request.tool}</span>
          <h2>Allow this action?</h2>
        </div>
        <div className="permission-dialog-body">
          <div className="permission-action">{request.action}</div>
          {argsJson && (
            <pre className="permission-args">{argsJson}</pre>
          )}
          {request.pattern && (
            <p className="permission-pattern">
              <strong>Pattern:</strong> <code>{request.pattern}</code>
              <br />
              <small>Choosing "Allow always" remembers this pattern for future matches.</small>
            </p>
          )}
        </div>
        <div className="permission-dialog-actions">
          <button className="permission-btn deny" onClick={() => submit('deny')}>
            Deny
          </button>
          <button className="permission-btn" onClick={() => submit('allow')}>
            Allow once
          </button>
          <button className="permission-btn primary" onClick={() => submit('allow_always')}>
            Allow always
          </button>
        </div>
      </div>
    </div>
  )
}
