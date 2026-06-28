import { useState } from 'react'
import { Plus, Power, Trash2, Server } from 'lucide-react'

interface McpServer {
  name: string
  status: 'running' | 'stopped' | 'error'
  command: string
  tools: number
}

const INITIAL_SERVERS: McpServer[] = [
  { name: 'filesystem', status: 'running', command: 'npx -y @modelcontextprotocol/server-filesystem', tools: 12 },
  { name: 'memory',     status: 'stopped', command: 'npx -y @modelcontextprotocol/server-memory',     tools: 9  },
]

export default function MCPServerView() {
  const [servers] = useState<McpServer[]>(INITIAL_SERVERS)

  return (
    <div style={{ padding: '4px 0' }}>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 18 }}>
        <div>
          <div style={{ fontSize: 13, color: 'var(--text-secondary)', fontFamily: "'JetBrains Mono', monospace" }}>
            MCP servers extend ORION with tools (filesystem, memory, git, web, etc).
          </div>
        </div>
        <button
          disabled
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 6,
            padding: '6px 12px',
            background: 'transparent',
            border: '0.5px solid var(--border-mid)',
            borderRadius: 6,
            color: 'var(--text-secondary)',
            fontSize: 12,
            fontFamily: "'JetBrains Mono', monospace",
            cursor: 'not-allowed',
            opacity: 0.6,
          }}
        >
          <Plus size={13} /> Add Server
        </button>
      </div>

      <div
        style={{
          background: 'var(--bg-secondary)',
          border: '0.5px solid var(--border-subtle)',
          borderRadius: 10,
          overflow: 'hidden',
        }}
      >
        {servers.length === 0 && (
          <div style={{ padding: 32, textAlign: 'center', color: 'var(--text-tertiary)', fontSize: 12 }}>
            No MCP servers configured. Click "Add Server" to install one.
          </div>
        )}
        {servers.map((s, i) => (
          <div
            key={s.name}
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'space-between',
              padding: '12px 16px',
              borderBottom: i < servers.length - 1 ? '0.5px solid var(--border-subtle)' : 'none',
            }}
          >
            <div style={{ display: 'flex', alignItems: 'center', gap: 12, minWidth: 0, flex: 1 }}>
              <div
                style={{
                  width: 8,
                  height: 8,
                  borderRadius: '50%',
                  background:
                    s.status === 'running'
                      ? 'var(--green, #1D9E75)'
                      : s.status === 'error'
                      ? 'var(--red, #A32D2D)'
                      : 'var(--text-tertiary, #4a4866)',
                  flexShrink: 0,
                }}
              />
              <div style={{ minWidth: 0, flex: 1 }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                  <Server size={13} style={{ color: 'var(--text-tertiary)', flexShrink: 0 }} />
                  <span style={{ fontFamily: "'JetBrains Mono', monospace", fontSize: 13, color: 'var(--text-primary)' }}>
                    {s.name}
                  </span>
                  <span style={{ fontSize: 10, color: 'var(--text-tertiary)', fontFamily: "'JetBrains Mono', monospace" }}>
                    {s.tools} tools
                  </span>
                </div>
                <div
                  style={{
                    fontFamily: "'JetBrains Mono', monospace",
                    fontSize: 11,
                    color: 'var(--text-tertiary)',
                    marginTop: 3,
                    overflow: 'hidden',
                    textOverflow: 'ellipsis',
                    whiteSpace: 'nowrap',
                  }}
                  title={s.command}
                >
                  {s.command}
                </div>
              </div>
            </div>
            <div style={{ display: 'flex', gap: 6, flexShrink: 0 }}>
              <button
                disabled
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 4,
                  padding: '4px 10px',
                  background: 'transparent',
                  border: '0.5px solid var(--border-mid)',
                  borderRadius: 6,
                  color: 'var(--text-secondary)',
                  fontSize: 11,
                  fontFamily: "'JetBrains Mono', monospace",
                  cursor: 'not-allowed',
                  opacity: 0.6,
                }}
                title={s.status === 'running' ? 'Stop server' : 'Start server'}
              >
                <Power size={11} />
                {s.status === 'running' ? 'Stop' : 'Start'}
              </button>
              <button
                disabled
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 4,
                  padding: '4px 10px',
                  background: 'transparent',
                  border: '0.5px solid var(--border-mid)',
                  borderRadius: 6,
                  color: 'var(--red-text, #f87171)',
                  fontSize: 11,
                  fontFamily: "'JetBrains Mono', monospace",
                  cursor: 'not-allowed',
                  opacity: 0.6,
                }}
                title="Remove server"
              >
                <Trash2 size={11} />
              </button>
            </div>
          </div>
        ))}
      </div>

      <p
        style={{
          fontSize: 11,
          color: 'var(--text-tertiary)',
          marginTop: 12,
          fontFamily: "'JetBrains Mono', monospace",
        }}
      >
        Backend wiring pending: Tauri commands to list/spawn/stop MCP servers will be added in a follow-up. The agent dispatch loop already calls MCP tools via orion_core::mcp::client.
      </p>
    </div>
  )
}