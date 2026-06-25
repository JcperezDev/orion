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

> Configuration and the model catalog are stored **per-user**. If you signed in as one Linux user and want to copy the saved providers / API keys to another user, copy `~/.config/orion/catalog.db` and `~/.config/orion/config.toml` to the target user's `~/.config/orion/` (paths respect `$XDG_CONFIG_HOME`).

## Agent mode (built-in tools)

Orion can run as an agent: the model reads files, runs commands, edits code, and reports back. Built-in tools are gated by a permission system (per-tool `allow` / `ask` / `deny` + glob patterns, last match wins).

| Tool        | What it does                                | Default |
|-------------|---------------------------------------------|---------|
| `read`      | Read a file (offset/limit supported)        | allow   |
| `write`     | Create / overwrite a file                   | ask     |
| `edit`      | Replace exact text in a file                | ask     |
| `bash`      | Run a shell command (stdout/stderr captured)| ask     |
| `grep`      | Regex search across files                   | allow   |
| `glob`      | Find files by glob                          | allow   |
| `todowrite` | Update a per-session todo list              | allow   |

Permission rules can be tuned at runtime from the Tauri command `add_permission_rule(tool, pattern, action)` or by editing the rules in `~/.config/orion/config.toml`.

## Project memory (AGENTS.md / ORION.md)

At session start Orion reads project memory in this order and merges it into the system prompt:

1. `./AGENTS.md`
2. `./ORION.md`
3. `~/.config/orion/AGENTS.md`
4. `~/.config/orion/ORION.md`

Each file may include optional YAML frontmatter delimited by `---` for `description`, `model`, and `permission` overrides.

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
