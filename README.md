# ORION - Terminal Agent

AI terminal agent built with Rust + Ratatui. Dynamic provider registry, model catalog, intelligent routing.

## Features

- **Dynamic Provider Registry** - OpenRouter, OpenAI, Anthropic, Ollama, and more
- **Model Catalog** - SQLite-backed with sync from OpenRouter API
- **Intelligent Router** - Selects best model by task type (vision, coding, cheap, etc.)
- **Fallback Chain** - Automatic provider fallback on errors
- **MCP Client** - Pre-request hooks with token-god middleware
- **Image Support** - Watch folder, process images via mcp-eyes
- **Memory** - SQLite-backed persistent memory
- **Streaming** - Token-by-token AI response streaming

## Install

```bash
cargo install orion-agent
```

Or from source:
```bash
git clone https://github.com/your-repo/orion
cd orion
./install.sh
```

## Configuration

Set API keys as environment variables:
```bash
export OPENROUTER_API_KEY="sk-or-..."
export OPENAI_API_KEY="sk-..."
export ANTHROPIC_API_KEY="sk-ant-..."
```

Config location: `~/.config/orion/config.toml`

Default model catalog: `~/.config/orion/catalog.db`

## Commands

```
/providers list              List available providers
/providers sync              Sync models from OpenRouter

/models list [provider]     List models
/models search <query>      Search models
/models vision              Models with vision support
/models tools               Models with tool calling
/models free               Free local models (Ollama)

/model <provider:model>    Set model (e.g., openrouter:anthropic/claude-3.5-sonnet)
/model                     Show current model

/best <task>               Best model for task (coding, vision, cheap, etc.)

/help                      Show help
```

## Model ID Format

```
provider:model

Examples:
  openrouter:anthropic/claude-3.5-sonnet
  openrouter:deepseek/deepseek-chat
  openai:gpt-4o
  anthropic:claude-sonnet-4-5
  ollama:llama3.2
```

## Architecture

```
src/
├── main.rs                 Entry point
├── app/                   App state, events, input
├── ui/                    Ratatui widgets
├── core/                  Agent, context, memory
├── providers/             LLM provider adapters
│   ├── traits.rs         Provider trait (async)
│   ├── registry.rs       Dynamic provider registry
│   ├── openrouter.rs     OpenRouter adapter
│   ├── openai_compatible.rs
│   ├── anthropic.rs
│   └── ollama.rs
├── models/               Model catalog
│   ├── catalog.rs        SQLite catalog
│   └── sync.rs          OpenRouter sync
├── router/              Intelligent routing
│   ├── selector.rs       Task-based model selector
│   └── fallback.rs      Provider fallback chain
├── config/              Configuration
├── mcp/                 MCP client, middleware
├── images/              Image processing, watch folder
└── analytics/           Anonymous analytics
```

## Database Schema

```sql
CREATE TABLE providers (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  kind TEXT NOT NULL,
  base_url TEXT,
  api_key_env TEXT,
  enabled INTEGER DEFAULT 1
);

CREATE TABLE models (
  id TEXT PRIMARY KEY,
  provider_id TEXT NOT NULL,
  model_id TEXT NOT NULL,
  display_name TEXT,
  context_window INTEGER,
  max_output INTEGER,
  input_price REAL,
  output_price REAL,
  supports_tools INTEGER DEFAULT 0,
  supports_vision INTEGER DEFAULT 0,
  supports_reasoning INTEGER DEFAULT 0,
  enabled INTEGER DEFAULT 1
);
```
