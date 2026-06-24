import { useState } from 'react'

export default function MCPServerView() {
  const [servers] = useState([
    { name: 'Filesystem', status: 'running', port: 3000 },
    { name: 'Memory', status: 'stopped', port: 3001 },
  ])

  return (
    <div className="h-full overflow-y-auto p-6">
      <div className="flex items-center justify-between mb-6">
        <h2 className="text-xl font-bold">MCP Hub</h2>
        <button className="px-4 py-2 bg-primary hover:bg-primary-hover text-white rounded-lg text-sm font-medium">
          Add Server
        </button>
      </div>
      <div className="space-y-3">
        {servers.map((server) => (
          <div key={server.name} className="bg-surface border border-border-subtle rounded-xl p-4 flex items-center justify-between">
            <div className="flex items-center gap-4">
              <div className={`w-3 h-3 rounded-full ${server.status === 'running' ? 'bg-success' : 'bg-text-subtle'}`} />
              <div>
                <div className="font-medium">{server.name}</div>
                <div className="text-xs text-text-muted">localhost:{server.port}</div>
              </div>
            </div>
            <div className="flex gap-2">
              <button className="px-3 py-1.5 text-xs bg-surface-raised hover:bg-border rounded-lg transition-colors">
                {server.status === 'running' ? 'Stop' : 'Start'}
              </button>
              <button className="px-3 py-1.5 text-xs text-error hover:bg-error/10 rounded-lg transition-colors">
                Remove
              </button>
            </div>
          </div>
        ))}
      </div>
    </div>
  )
}
