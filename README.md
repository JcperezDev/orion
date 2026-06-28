# ORION — AI Coding Agent

Multi-surface AI coding agent built in Rust. One binary (`orion`) drives the TUI, the headless CLI, and the HTTP server; a Tauri desktop app wraps the same `orion-core`.

## Highlights

- **Agent mode with built-in tools** — the model reads files, runs commands, edits code, searches, and reports back.
- **Trust Engine** — an innovative permission model that decides by *consequence and reversibility* instead of asking per tool, so it prompts far less than typical agents (see below). A single **Full access** switch disables all prompts when you want it.
- **Multi-provider** — Anthropic + Ollama have native adapters; OpenAI, OpenRouter, DeepSeek, Groq, Mistral, Together, Perplexity, MiniMax, Google and more run through the OpenAI-compatible adapter. A default model is seeded per provider so a connected provider is usable immediately.
- **Model catalog** — SQLite-backed, with sync from OpenRouter / models.dev.
- **Persistent sessions** — chat history is stored in SQLite and survives restarts; conversations are multi-turn aware.
- **Secure key storage** — API keys live in the OS keyring (with an encrypted-DB fallback on headless systems).
- **Desktop app** — Tauri 2 + React 18 with a custom borderless window.

## Install

**One-liner (downloads a prebuilt release):**

```bash
curl -fsSL https://raw.githubusercontent.com/JcperezDev/orion/master/install.sh | bash
```

The script detects your OS/arch (Linux & macOS, x86_64 & arm64) and installs `orion` + `orion-server` to `~/.local/bin/`. Requires a published [release](https://github.com/JcperezDev/orion/releases).

**From source:**

```bash
git clone https://github.com/JcperezDev/orion
cd orion
./install.sh        # same script — builds in release mode when run inside the repo
```

Override the install dir with `ORION_INSTALL_DIR=/path`. Windows users can grab the `.zip` from the releases page.

Desktop app (dev):

```bash
cd apps/orion-desktop
npm install
npm run tauri dev
```

## CLI

```bash
orion                       # launch the TUI (default with no subcommand)
orion run "fix the bug"     # headless single prompt, streaming output
orion run --model openrouter:anthropic/claude-3.5-sonnet "..."
orion serve --port 7337     # start the HTTP server
orion providers             # list providers + which have API keys
orion connect minimax       # save an API key for a provider (reads from stdin)
orion sessions              # list recent chat sessions
orion agents                # list configured agents (Build/Plan/Explore/Scout)
orion full-access on|off    # toggle the Trust Engine's master switch
orion stats --days 30       # token / cost usage stats
orion acp                   # ACP server over stdio (editor integration)
orion init                  # write AGENTS.md in the current dir
```

All surfaces read from the same SQLite catalog at `~/.config/orion/catalog.db`, so providers, keys, sessions, and settings are shared.

## Configuration & keys

Connect a provider in one of two ways:

```bash
# 1) Save the key into ORION (stored in the OS keyring; recommended)
orion connect openai        # prompts for the key

# 2) Or set an environment variable (used as a fallback)
export OPENAI_API_KEY="sk-..."
export ANTHROPIC_API_KEY="sk-ant-..."
export OPENROUTER_API_KEY="sk-or-..."
```

API keys are stored in the **OS keyring** (Secret Service / Keychain / Credential Manager). On systems with no keyring available, ORION falls back to the local catalog DB and migrates the key to the keyring the next time one is present. Set `ORION_DISABLE_KEYRING=1` to force the DB path.

- Config: `~/.config/orion/config.toml`
- Catalog (providers, models, sessions, messages): `~/.config/orion/catalog.db`
- Learned permissions: `~/.config/orion/permissions.db`

Paths respect `$XDG_CONFIG_HOME`.

## Agent mode & built-in tools

Three modes (switchable in the desktop and TUI):

- **Build** — direct model response, no tools.
- **Plan** — read-only: explores the code without editing.
- **Agent** — full tool use.

Built-in tools include `read`, `write`, `edit`, `apply_patch`, `bash`, `grep`, `glob`, `list`, `webfetch`, `websearch`, `lsp`, `todowrite`, and `task` (subagents).

## Trust Engine (permissions)

Instead of asking before every tool, ORION decides by **what the action actually does**:

- **Reads** and edits/writes **inside the project** are auto-allowed — they're snapshot-reversible (and you get an Undo affordance).
- **Bash** commands are classified by their AST (via tree-sitter): `ls`, `git status`, `cargo test`… run automatically; `rm -rf`, `curl | sh`, `sudo`, writes outside the project, etc. prompt for approval.
- **"Always allow"** decisions are remembered per project (`permissions.db`).
- A read-only **Plan** agent can hard-deny edits regardless.

The classic glob rules still apply (per-tool `allow`/`ask`/`deny`, last match wins) and take precedence over the heuristics.

**Full access** — a master switch (`orion full-access on`, or Settings → Permissions in the desktop) that allows every tool with no prompts, for when you don't want to be asked at all.

When a provider returns a usage/rate limit, ORION surfaces a clear message: transient limits retry with backoff (and the desktop offers Resume); hard quota limits suggest switching model or adding credits.

## Project memory (AGENTS.md / ORION.md)

At session start ORION reads project memory in this order and merges it into the system prompt:

1. `./AGENTS.md`
2. `./ORION.md`
3. `~/.config/orion/AGENTS.md`
4. `~/.config/orion/ORION.md`

Each file may include optional YAML frontmatter (`---`) for `description`, `model`, and `permission` overrides.

## Slash commands (TUI)

```
/providers list|status|sync   Manage providers / sync models
/models list|search|vision|tools|free   Browse the catalog
/model <provider:model>       Set the active model
/model                        Show the current model
/best <task>                  Best model for a task (coding, vision, cheap…)
/help                         Show help
```

## Model ID format

```
provider:model

Examples:
  openrouter:anthropic/claude-3.5-sonnet
  openai:gpt-4o-mini
  anthropic:claude-3-5-sonnet-20241022
  minimax:MiniMax-Text-01
  ollama:llama3.2
```

## Workspace layout

```
crates/orion-core      Core library: providers, catalog, router, agent loop,
                       tools, permissions (Trust Engine), MCP, sessions, ACP.
apps/orion-cli         The `orion` binary (TUI + headless run + serve).
apps/orion-tui         Ratatui terminal UI.
apps/orion-server      Axum HTTP server.
apps/orion-desktop     Tauri 2 + React desktop app.
```

## Development

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace          # advisory; findings are being cleared
cd apps/orion-desktop && npm run build
```

CI runs build + tests as hard gates, with fmt/clippy as advisory for now.

## License

MIT — see [LICENSE](LICENSE).
