import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'

interface ModelSummary {
  id: string
  name: string
  provider: string
  description?: string
  task_kinds: string[]
  supports_vision: boolean
  supports_tools: boolean
  cost_input: number
  cost_output: number
  context_length: number
}

interface Props {
  models: ModelSummary[]
  onSearch: (query: string) => void
  onLoad: (provider?: string) => void
  loading: boolean
}

export default function ModelsView({ models, onSearch, onLoad, loading }: Props) {
  const [query, setQuery] = useState('')
  const [selectedModel, setSelectedModel] = useState<ModelSummary | null>(null)
  const [filterProvider, setFilterProvider] = useState<string>('')
  const [filterVision, setFilterVision] = useState(false)
  const [filterTools, setFilterTools] = useState(false)
  const [filterReasoning, setFilterReasoning] = useState(false)

  useEffect(() => {
    onLoad(filterProvider || undefined)
  }, [filterProvider])

  function handleSearch(e: React.FormEvent) {
    e.preventDefault()
    onSearch(query)
  }

  function handleSetDefault(model: ModelSummary) {
    invoke('set_active_model', { modelId: model.id })
      .then(() => {
        setSelectedModel(model)
        alert(`Default model set to: ${model.name}`)
      })
      .catch(err => alert(`Error: ${err}`))
  }

  const filteredModels = models.filter(m => {
    if (filterVision && !m.supports_vision) return false
    if (filterTools && !m.supports_tools) return false
    if (filterReasoning && !m.task_kinds.includes('reasoning')) return false
    return true
  })

  return (
    <div className="flex h-full">
      {/* Left sidebar - Filters */}
      <div className="w-64 border-r border-border-subtle flex flex-col bg-surface">
        <div className="p-4 border-b border-border-subtle">
          <h3 className="font-semibold mb-3">Filters</h3>
          
          <div className="space-y-2">
            <div>
              <label className="text-xs text-text-subtle">Provider</label>
              <select
                value={filterProvider}
                onChange={(e) => setFilterProvider(e.target.value)}
                className="w-full bg-surface-raised border border-border rounded px-2 py-1.5 text-sm text-text focus:outline-none focus:border-primary"
              >
                <option value="">All providers</option>
                <option value="openai">OpenAI</option>
                <option value="anthropic">Anthropic</option>
                <option value="google">Google</option>
                <option value="deepseek">DeepSeek</option>
                <option value="groq">Groq</option>
                <option value="mistral">Mistral</option>
                <option value="together">Together</option>
                <option value="openrouter">OpenRouter</option>
              </select>
            </div>

            <div className="space-y-1">
              <label className="text-xs text-text-subtle">Capabilities</label>
              <label className="flex items-center gap-2 cursor-pointer">
                <input
                  type="checkbox"
                  checked={filterVision}
                  onChange={(e) => setFilterVision(e.target.checked)}
                  className="w-3.5 h-3.5 rounded border-border bg-surface-raised text-primary"
                />
                <span className="text-sm">Vision</span>
              </label>
              <label className="flex items-center gap-2 cursor-pointer">
                <input
                  type="checkbox"
                  checked={filterTools}
                  onChange={(e) => setFilterTools(e.target.checked)}
                  className="w-3.5 h-3.5 rounded border-border bg-surface-raised text-primary"
                />
                <span className="text-sm">Tools</span>
              </label>
              <label className="flex items-center gap-2 cursor-pointer">
                <input
                  type="checkbox"
                  checked={filterReasoning}
                  onChange={(e) => setFilterReasoning(e.target.checked)}
                  className="w-3.5 h-3.5 rounded border-border bg-surface-raised text-primary"
                />
                <span className="text-sm">Reasoning</span>
              </label>
            </div>
          </div>
        </div>

        <div className="p-4">
          <form onSubmit={handleSearch} className="flex gap-2">
            <input
              type="text"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder="Search models..."
              className="flex-1 bg-surface-raised border border-border rounded px-2 py-1.5 text-sm text-text placeholder:text-text-subtle focus:outline-none focus:border-primary"
            />
          </form>
        </div>

        <div className="flex-1 overflow-y-auto px-2">
          {filteredModels.map((model) => (
            <button
              key={model.id}
              onClick={() => setSelectedModel(model)}
              className={`w-full text-left p-3 rounded-lg mb-1 transition-colors ${
                selectedModel?.id === model.id 
                  ? 'bg-primary/20 border border-primary/50' 
                  : 'hover:bg-surface-raised'
              }`}
            >
              <div className="font-medium text-sm truncate">{model.name}</div>
              <div className="text-xs text-text-muted">{model.provider}</div>
            </button>
          ))}
          {filteredModels.length === 0 && !loading && (
            <div className="text-center text-text-subtle text-sm py-8">
              No models found
            </div>
          )}
        </div>
      </div>

      {/* Main content - Model detail */}
      <div className="flex-1 overflow-y-auto p-6">
        {selectedModel ? (
          <div>
            <div className="flex items-center justify-between mb-4">
              <div>
                <h2 className="text-2xl font-bold">{selectedModel.name}</h2>
                <p className="text-text-muted">{selectedModel.provider}</p>
              </div>
              <button
                onClick={() => handleSetDefault(selectedModel)}
                className="px-4 py-2 bg-primary hover:bg-primary-hover text-white rounded-lg text-sm font-medium transition-colors"
              >
                Set as Default
              </button>
            </div>

            <p className="text-text-subtle mb-6">{selectedModel.description || 'No description available'}</p>

            {/* Capabilities */}
            <div className="mb-6">
              <h3 className="font-semibold mb-2">Capabilities</h3>
              <div className="flex flex-wrap gap-2">
                {selectedModel.task_kinds.map(tk => (
                  <span key={tk} className="text-xs bg-surface px-2 py-1 rounded text-text-muted capitalize">
                    {tk}
                  </span>
                ))}
                {selectedModel.supports_vision && (
                  <span className="text-xs bg-accent/20 text-accent px-2 py-1 rounded">Vision</span>
                )}
                {selectedModel.supports_tools && (
                  <span className="text-xs bg-accent-secondary/20 text-accent-secondary px-2 py-1 rounded">Tools</span>
                )}
              </div>
            </div>

            {/* Pricing */}
            <div className="grid grid-cols-2 gap-4 mb-6">
              <div className="bg-surface rounded-xl p-4">
                <div className="text-xs text-text-subtle">Input Cost</div>
                <div className="text-lg font-mono">
                  {selectedModel.cost_input > 0 ? `$${selectedModel.cost_input}/1M tokens` : 'Free'}
                </div>
              </div>
              <div className="bg-surface rounded-xl p-4">
                <div className="text-xs text-text-subtle">Output Cost</div>
                <div className="text-lg font-mono">
                  {selectedModel.cost_output > 0 ? `$${selectedModel.cost_output}/1M tokens` : 'Free'}
                </div>
              </div>
            </div>

            {/* Context */}
            <div className="bg-surface rounded-xl p-4">
              <div className="text-xs text-text-subtle">Context Window</div>
              <div className="text-lg font-mono">
                {selectedModel.context_length > 0 ? selectedModel.context_length.toLocaleString() : 'Unknown'} tokens
              </div>
            </div>
          </div>
        ) : (
          <div className="h-full flex items-center justify-center text-text-subtle">
            <div className="text-center">
              <p className="text-lg mb-2">Select a model</p>
              <p className="text-sm">Choose a model from the list or search to see details</p>
            </div>
          </div>
        )}
      </div>
    </div>
  )
}
