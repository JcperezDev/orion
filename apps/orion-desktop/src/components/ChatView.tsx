import { useState } from 'react'

interface Props {
  defaultModel: string | null
}

export default function ChatView({ defaultModel }: Props) {
  const [input, setInput] = useState('')
  const [messages, setMessages] = useState<Array<{role: string; content: string}>>([
    { role: 'assistant', content: 'Welcome to Orion. Select a model from the Models page to start chatting.' }
  ])
  const [loading, setLoading] = useState(false)

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault()
    if (!input.trim() || loading) return

    const userMessage = input
    setInput('')
    setMessages(prev => [...prev, { role: 'user', content: userMessage }])
    setLoading(true)

    try {
      if (!defaultModel) {
        setMessages(prev => [...prev, { 
          role: 'assistant', 
          content: 'No model configured. Please go to Models page and select a default model first.' 
        }])
        return
      }

      setMessages(prev => [...prev, { 
        role: 'assistant', 
        content: `Using model: ${defaultModel}. This is a demo - actual API calls would be made here.` 
      }])
    } catch (err) {
      setMessages(prev => [...prev, { 
        role: 'assistant', 
        content: `Error: ${String(err)}` 
      }])
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="flex flex-col h-full">
      <div className="flex-1 overflow-y-auto p-6">
        <div className="max-w-3xl mx-auto space-y-4">
          {messages.map((msg, i) => (
            <div 
              key={i}
              className={`rounded-xl p-4 ${
                msg.role === 'user' 
                  ? 'bg-primary/20 ml-auto max-w-[80%]' 
                  : 'bg-surface border border-border-subtle mr-auto max-w-[80%]'
              }`}
            >
              <div className="text-xs text-text-subtle mb-1 capitalize">{msg.role}</div>
              <p className="text-text">{msg.content}</p>
            </div>
          ))}
          {loading && (
            <div className="bg-surface border border-border-subtle rounded-xl p-4 mr-auto max-w-[80%]">
              <div className="text-text-muted">Thinking...</div>
            </div>
          )}
        </div>
      </div>
      <div className="p-4 border-t border-border-subtle">
        <form onSubmit={handleSubmit} className="max-w-3xl mx-auto flex gap-3">
          <input
            type="text"
            value={input}
            onChange={(e) => setInput(e.target.value)}
            placeholder={defaultModel ? "Message Orion..." : "Configure a model first..."}
            disabled={loading}
            className="flex-1 bg-surface border border-border rounded-xl px-4 py-3 text-text placeholder:text-text-subtle focus:outline-none focus:border-primary disabled:opacity-50"
          />
          <button
            type="submit"
            disabled={loading || !input.trim() || !defaultModel}
            className="px-6 py-3 bg-primary hover:bg-primary-hover text-white rounded-xl font-medium transition-colors disabled:opacity-50"
          >
            Send
          </button>
        </form>
      </div>
    </div>
  )
}
