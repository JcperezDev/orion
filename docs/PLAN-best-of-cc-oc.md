# Plan: Best-of-Claude-Code + Best-of-OpenCode → Orion

> Strategic plan. No code yet. Awaiting scope decisions.

## 0. Legitimacy & Safety Verification

Before borrowing anything, we confirm the two reference repos are safe to study. We did NOT clone them; we used GitHub MCP and public docs only.

| Repo | Owner | Stars | License | Source on GitHub | Verdict |
|---|---|---|---|---|---|
| `anthropics/claude-code` | Anthropic Inc (org id 76263028, the company) | ~134k | MIT (code); proprietary binary | **Mostly closed.** Public repo holds README, plugins/, SECURITY.md, examples. Binary distributed via npm `@anthropic-ai/claude-code`. Real agent loop is in the npm package, not on GitHub. | ✅ Safe to study. We can read plugins + docs. We CANNOT copy the agent loop — it's not in the public repo. We CAN port the *plugin structure* (it's published and the docs encourage it). |
| `anomalyco/opencode` | Anomaly (opencode.ai org) | ~178k | MIT | Fully open source on `dev` branch. TypeScript + Bun monorepo. 28 packages: `app`, `cli`, `core`, `server`, `desktop`, `tui`, `web`, `llm`, `plugin`, `sdk`, `schema`, `effect-*`, etc. | ✅ Safe to study + port patterns. MIT license means we can reuse patterns freely with attribution. |

**Supply-chain notes for Orion users when adopting similar install patterns later:** both projects use `curl … \| bash` installers. We will NOT ship that pattern in Orion's installer — Orion already uses `cargo install` + a static `install.sh` (good). If we ever add a one-liner, sign it with a checksum.

---

## 1. Why this matters

Orion today is a **multi-provider chat client with a desktop shell**, not an agent. The LLM streams words back; it cannot touch the filesystem, run commands, call tools, or delegate to subagents. Both reference projects **are** agents in the full sense — the user describes intent, the model plans, takes actions through gated tools, and reports back. This is the largest single gap between Orion and either of them.

The other big gap: Orion has **no extension mechanism**. Both Claude Code and OpenCode ship a plugin/agent/command system so users (and the team) can extend the product without forking.

Everything else is incremental polish.

---

## 2. Orion today — what's actually built (verified by reading source)

- **Workspace**: `crates/orion-core` (5,801 LoC), `apps/orion-desktop` (Tauri 2 + React + TS), `apps/orion-server` (Axum HTTP), `apps/orion-tui` (ratatui).
- **Providers wired** (`crates/orion-core/src/providers/`): `anthropic`, `openai_compatible` (covers OpenAI + OpenRouter), `ollama`. 12 providers exist in the desktop UI / provider-test endpoint (`minimax`, `google`, `groq`, `mistral`, `together`, `perplexity`, etc.) but only 3 are actually implemented as `LlmProvider` trait impls. Most other "providers" are routed through `openai_compatible`.
- **Chat loop**: streaming only. `crates/orion-core/src/core/agent.rs` — no tool calling, no function dispatch. `Message` type (`providers/traits.rs:13-18`) has only `role` + `content`. `ChatRequest` has an unused `tools` field but the agent never populates or handles it.
- **Sessions**: SQLite-backed (`memory/store.rs`, `models/catalog.rs` v3 migration). Per-session title, model, message list, active session. No forking, no branching, no share link.
- **Memory**: SQLite (`memory/store.rs`). Flat key/value style with a `Settings` struct (`default_provider`, `theme`, `auto_accept_permissions`, etc.) but **no project-scoped memory** (no CLAUDE.md / AGENTS.md auto-load).
- **MCP**: stub (`mcp/manager.rs`). Has `McpServer { name, url, enabled }`, a hardcoded port map (`filesystem:3000`, `github:3001`…), and one `pre_request_hook` (token-god trim). **No real MCP protocol client**, no JSON-RPC, no `tools/list` / `tools/call`.
- **No tools**: no Read, Write, Edit, Bash, Grep, Glob, WebFetch — anything. The agent can only stream text.
- **No permissions**: nothing gates any action, because there are no actions.
- **No subagents**: no plan vs build mode, no `@general` / `@explore`, no Task tool.
- **No slash commands**: TUI has a hardcoded match in `agent.rs:74-100`. Desktop has none — only buttons.
- **No plugin/extension system**: nowhere to drop a `~/.config/orion/plugins/foo/` folder.
- **No themes** (fixed dark in `index.css:7-54`), **no undo/redo**, **no share**, **no compaction beyond message-count trim** (`mcp/manager.rs:34-42`).
- **Pre-request hook** (`middleware/token_optimizer.rs`) and provider catalog sync (`models/sync.rs`) are genuinely good and worth keeping — they're beyond what either reference ships.

---

## 3. What to borrow from each

### From Claude Code (high-leverage, mostly about UX patterns we can re-implement)

1. **Plugin directory structure** — `plugin-name/{commands,agents,skills,hooks,.mcp.json,.claude-plugin/plugin.json,README.md}`. Cheap to define, huge leverage. Borrow the **shape**, not the contents (their plugins use Anthropic-specific model names).
2. **Slash commands from `commands/`** — borrow the *pattern* of a folder-of-markdown-files that the user can drop in. e.g. `/commit`, `/test`, `/review-pr` — these are tiny prompt templates + tool restrictions.
3. **Hooks concept** — PreToolUse / PostToolUse / Stop. Adopt this *after* we have tools (Phase 1 below).
4. **CLAUDE.md equivalent** — but for Orion we'll call it `AGENTS.md` (matches the file `gstack` and the broader agent ecosystem already use; also lines up with `ORION.md`).
5. **Multi-agent PR review / code-review plugin** — too deep for v1; defer.

### From OpenCode (high-leverage, mostly about agent architecture)

1. **Two-mode split: Build (full) vs Plan (read-only)** — primary agents, switchable. Highest-leverage single feature in this plan. Plan agent has all file/bash tools denied or set to `ask`.
2. **Subagent system** — `general`, `explore` (read-only), `scout` (read-only external). Subagents are invoked via a Task tool; they get child sessions, the parent can `session_child_first/cycle/parent` to navigate.
3. **Permission system** — per-tool `allow`/`ask`/`deny` with **glob pattern matching** for bash commands. e.g. `{ "bash": { "*": "ask", "git status *": "allow" } }`. Last match wins. Cleanest permission design we've seen.
4. **Custom tool definitions** — let users declare a tool in `~/.config/orion/tools/` as a JSON schema + a shell command or HTTP call. Massive leverage: zero code to add a tool.
5. **Built-in tool set** — bash, edit, write, read, grep, glob, todowrite, webfetch, question (ask user mid-task). These are the *first* tools to add.
6. **Markdown-defined agents** — `~/.config/orion/agents/review.md` and `.orion/agents/review.md`. Same pattern as slash commands; one file = one subagent.
7. **Auto-compaction** — hidden subagent that summarizes long context. v2 feature; defer.

### What we explicitly will NOT port

- **OpenCode's Bun + Effect + Drizzle stack** — Orion is Rust + Ratatui + Tauri. Porting their entire monorepo is a rewrite.
- **Claude Code's installer pattern** — `curl | bash` is a non-starter for a Rust tool that already has `install.sh`.
- **OpenCode's Astro docs site** — irrelevant to Orion.
- **Either project's cloud-sync / `/share` link feature** — out of scope for v1.
- **Either project's Tauri-vs-Electron debate** — Orion is already Tauri. Done.

---

## 4. High-impact gap table

Columns: **O** = Orion has it (✓/✗), **CC** = Claude Code, **OC** = OpenCode, **Effort** (S/M/L for Orion, Rust-friendly), **Impact** (1-10 for an Orion user today).

| Capability | O | CC | OC | Effort | Impact | Note |
|---|---|---|---|---|---|---|
| Multi-provider chat + streaming | ✓ | ✓ | ✓ | — | — | Orion's baseline; better than both at task-routing |
| Sessions persisted in SQLite | ✓ | ✓ | ✓ | — | — | Orion ✓ |
| MCP client (real protocol) | ✗ | ✓ | ✓ | M | 8 | Stub → wire JSON-RPC `initialize` / `tools/list` / `tools/call`. We have hooks already; just need the wire. |
| MCP server (expose Orion's tools to other agents) | ✗ | ✗ | ✓ | L | 5 | Defer. |
| Tool use in chat loop (function calling) | ✗ | ✓ | ✓ | L | 10 | **Biggest single gap.** Add `ToolCall`/`ToolResult` to `Message`, dispatch loop in `agent.rs`. |
| Built-in tools: read, write, edit, bash, grep, glob | ✗ | ✓ | ✓ | M | 10 | Blocked on above. |
| Permission system (allow/ask/deny + glob) | ✗ | ✓ | ✓ | M | 9 | Blocked on tools. Without this, tools are unsafe. |
| Slash commands (user-defined) | ✗ | ✓ | ✓ | S | 6 | Folder of markdown files. Cheap once commands are parsed. |
| Custom subagents via markdown | ✗ | ✓ | ✓ | M | 8 | Reuses subagent + permission infra. |
| Build / Plan mode toggle | ✗ | ✗ | ✓ | S | 8 | Single subagent + permission flip. |
| Project memory auto-load (`AGENTS.md` / `ORION.md`) | ✗ | ✓ | ✓ | S | 8 | Read file at session start, prepend to system prompt. |
| Auto-compaction of long context | partial | ✓ | ✓ | M | 5 | Today only message-count trim. Add LLM-based summarizer later. |
| Session forking / branching | ✗ | ✓ | ✓ | M | 4 | Defer to v2. |
| Undo / Redo last tool action | ✗ | ✓ | ✓ | S | 7 | Git is the source of truth; just need `git stash` integration. |
| Themes (configurable) | ✗ | ✗ | ✓ | S | 3 | Cosmetic. Defer. |
| Web search / WebFetch | ✗ | ✓ | ✓ | S | 6 | One HTTP tool + one search tool. |
| `question` tool (model asks user mid-task) | ✗ | ✗ | ✓ | S | 7 | Tauri command `ask_user` that pops a dialog; model uses it like AskUserQuestion. |
| Plugin folder structure | ✗ | ✓ | ✓ | M | 8 | Composite — depends on commands + subagents + tools. |
| `curl | bash` installer | ✗ | ✓ | ✓ | — | Hard NO. |
| Real provider impls for the 12 declared providers | partial | n/a | n/a | M | 6 | Right now, 12 names in UI but only 3 real providers. Need to actually implement Anthropic via OpenAI-compatible? Or expand? |

---

## 5. Recommended roadmap (sequential; each phase is shippable on its own)

### Phase 1 — Make Orion an agent (foundational, ~2-3 weeks of focused work)

This is THE change. Without it, Orion is a chat client.

1. **Extend `Message` + `ChatRequest`** to carry `tool_calls` (per OpenAI/Claude format) and `tool_results`. Add a `Tool` trait in `crates/orion-core/src/tools/`.
2. **Implement 5 built-in tools**: `read`, `write`, `edit`, `bash`, `grep`, `glob`. Each tool = small Rust struct + JSON schema exposed to the model. Each tool gated by the permission system.
3. **Implement the permission system** at `crates/orion-core/src/permissions.rs`: `enum Action { Allow, Ask, Deny }`, glob matcher (use the `globset` crate), `Rule { pattern: String, action: Action }`, last-match-wins, per-agent override.
4. **Agent dispatch loop** in `core/agent.rs`: stream → if model returns `tool_calls`, execute tools, append `tool_results`, re-call model. Repeat until model returns text-only. Hook permission system at the gate.
5. **Tauri commands** for `ask_permission` (pops a desktop dialog with Allow/Always-allow-this-pattern/Deny).
6. **Real MCP client** in `crates/orion-core/src/mcp/`: JSON-RPC over stdio, `initialize`, `tools/list`, `tools/call`. Exposed as `Tool` impls so MCP tools are first-class.
7. **Tests**: each tool unit-tested; permission matrix covered; agent loop integration test with a mocked LLM.

User-visible: Orion can now edit a file the user names, run a test, fix it, and report back. Game over for "just a chat client."

### Phase 2 — Modes + subagents + project memory (~1-2 weeks)

8. **Build / Plan mode** — two subagents defined in `crates/orion-core/src/agents/`. Plan = `permission: { edit: deny, bash: "ask" }`. Build = full access. Toggle in Tauri command + TUI keybind + desktop switch.
9. **Subagent system** — `Task` tool that lets the active agent invoke a child subagent in a child session. Built-ins: `general` (full), `explore` (read-only), `scout` (read-only, webfetch-heavy).
10. **`AGENTS.md` / `ORION.md` auto-load** — at session start, read `./AGENTS.md` then `~/.config/orion/AGENTS.md`, merge, prepend to system prompt. Cache file mtime.
11. **Markdown-defined agents** — `~/.config/orion/agents/foo.md` and `.orion/agents/foo.md`. Schema mirrors OpenCode's markdown agent format. Loader is ~80 lines.
12. **Markdown-defined slash commands** — same shape, in `commands/` subfolder.

User-visible: User can drop `~/.config/orion/agents/reviewer.md` and immediately have a `reviewer` subagent. Project conventions auto-loaded.

### Phase 3 — Polish, MCP server, distribution (~1-2 weeks)

13. **MCP server** — expose Orion's built-in tools (read/write/edit/bash) as an MCP server so other agents can call them. Low effort because we already have the `Tool` trait.
14. **Undo/Redo** — shell out to `git stash` (or a built-in file-history store for non-git dirs). UI buttons in the desktop app.
15. **WebFetch + WebSearch tools** — one HTTP client each; gated by permissions.
16. **`question` tool** — Tauri dialog; model pauses for human input on ambiguity. Big UX win.
17. **Real provider impls** for the 8 providers that are currently just UI placeholders. Most can share `openai_compatible` with provider-specific base URLs + auth headers.
18. **Update channel** — `orion update` command using `tauri-plugin-updater` (already in `Cargo.toml:11`).
19. **Theme system** — JSON theme files in `~/.config/orion/themes/`, hot-reload.

User-visible: Orion is now a peer competitor to Claude Code and OpenCode for the developer-agent niche, with a real desktop UI and the provider breadth both lack.

---

## 6. Top picks (impact-effort, in priority order)

1. **Tool-use in the chat loop + 5 built-in tools** (Phase 1.1-1.2). Impact 10, effort L. Without this, nothing else matters.
2. **Permission system** (Phase 1.3). Impact 9, effort M. Without this, tools are a liability.
3. **Build / Plan mode** (Phase 2.8). Impact 8, effort S. Reuses permission infra; cheap after Phase 1.
4. **`AGENTS.md` auto-load** (Phase 2.10). Impact 8, effort S. Low effort, high daily-use value.
5. **Markdown-defined agents + slash commands** (Phase 2.11-2.12). Impact 8, effort M. Unlocks the ecosystem.
6. **Real MCP client** (Phase 1.6). Impact 8, effort M. Connects Orion to the broader agent ecosystem.

Cut from v1 (parking lot): session forking, theme system, MCP server, undo/redo, websearch. All valuable, none urgent.

---

## 7. Risks & anti-patterns

- **Permission UX is hard.** OpenCode's `allow/ask/deny` is clean in YAML; in a desktop app, "ask" means a modal. We need a non-blocking pattern (Tauri tray notification?) for long-running tool loops.
- **Tool output truncation.** A `bash` tool that returns 10MB of logs will kill the context. Need a `head`/truncation layer with a "view full output" affordance.
- **The 12 providers gap.** Adding 9 more provider impls is 9 places to get rate limiting, retry, and streaming wrong. Consider OpenCode's approach: most providers are OpenAI-compatible — centralize and only Anthropic + Ollama need bespoke code.
- **License.** Both repos are MIT. We can borrow patterns freely with attribution; we should add a `THIRD_PARTY.md` and reference both in `README.md` under "Inspired by."
- **Scope creep.** Boil-the-ocean applies *within* each phase, not across all three. Don't start Phase 2 before Phase 1 is merged and used in anger.
- **The current Orion users.** Whatever ships, the chat-only path must keep working — no forced migration.

---

## 8. Locked scope (decided by user)

- **Scope**: Phase 1 only — Make Orion an agent.
- **PR strategy**: One PR, end-to-end. Tool trait + 5 built-in tools + permission system + agent loop + MCP client + Tauri permission dialog + tests, atomic. Includes the two operational fixes (titlebar + catalog.db note) since they're small and unblock testing.
- **Backwards compat**: Chat-only mode stays as a selectable mode; agent mode is the default for new sessions.
- **`AGENTS.md` lookup**: At session start, read in order `./AGENTS.md` → `./ORION.md` → `~/.config/orion/AGENTS.md` → `~/.config/orion/ORION.md`. Merge all found, cwd-first precedence. Optional YAML frontmatter for per-project permission overrides.
- **`bash` isolation**: Permission system only (allow/ask/deny + glob patterns). No sandbox in v1. Land a Landlock/seccomp layer in v2 once we see real command patterns.
- **Operational fixes folded in**:
  - Titlebar: guard the `getCurrentWindow()` calls in `TitleBar.tsx` so the bar renders harmlessly in non-Tauri contexts (browser preview) and the buttons still work in Tauri webview. `data-tauri-drag-region` is a no-op outside Tauri; that's fine.
  - Catalog.db: no code change — `~/.config/orion/catalog.db` is correct per-user behavior. Add a one-paragraph note in `README.md` ("Configuration is per-user; copy across users with `cp ~/.config/orion/catalog.db /target/home/.config/orion/`").

## 9. What this plan does NOT decide (deferred)

- Session forking / branching — v2.
- Markdown-defined agents and slash commands — Phase 2.
- MCP server mode (expose Orion's tools to other agents) — v2.
- Auto-compaction of long context via LLM summarizer — v2.
- Theme system — v2.
- Real provider impls for the 8 providers that are currently UI-only — separate workstream.
- Sandbox for `bash` (Landlock / seccomp / container) — v2 after we see real command patterns.

---

## File map for implementation

New crates/files expected (sketch — for review only):

```
crates/orion-core/src/
  tools/
    mod.rs           # Tool trait + registry
    read.rs          # ~80 lines
    write.rs         # ~60 lines
    edit.rs          # ~80 lines
    bash.rs          # ~120 lines (with permission glob)
    grep.rs          # ~50 lines
    glob.rs          # ~40 lines
    mcp_client.rs    # ~250 lines, JSON-RPC stdio
    webfetch.rs      # ~60 lines
    websearch.rs     # ~70 lines
    question.rs      # ~30 lines (delegates to Tauri command)
    todowrite.rs     # ~50 lines
  permissions.rs     # ~180 lines, glob matcher + rule engine
  agents/
    mod.rs           # Agent trait + markdown loader
    builtins.rs      # general, explore, scout, build, plan, compaction
  memory/
    project.rs       # AGENTS.md loader (~60 lines)
  commands/          # slash command parser + loader
    mod.rs           # ~120 lines

apps/orion-desktop/src/
  components/
    ModeSwitch.tsx           # Build / Plan toggle
    PermissionDialog.tsx     # Tauri ask_user modal
    ToolCallCard.tsx         # renders tool invocations in chat
    SubagentTree.tsx         # parent/child session navigation
  hooks/
    usePermissions.ts        # bridge to Rust permission state

crates/orion-core/tests/
  permissions_test.rs
  tool_dispatch_test.rs
  agent_loop_test.rs         # mock LLM, real tools, assert flow

docs/
  AGENTS.md                  # file format spec for project memory
  PLUGINS.md                 # plugin folder spec
  PERMISSIONS.md             # permission glob syntax
```

Estimated total new code: ~3,500 LoC Rust + ~800 LoC TypeScript across all three phases. Phase 1 alone: ~2,000 LoC Rust + 200 LoC TS.

---

## 10. Open scope questions for the user

None remaining. Scope is locked per section 8. Next step is user green-light to implement.
