use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use orion_core::acp::run_acp_server;
use orion_core::config::Config;
use orion_core::core::agent::Agent;
use orion_core::models::catalog::ModelCatalog;
use orion_core::providers::registry::ProviderRegistry;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;

const ABOUT: &str = "ORION — AI coding agent with multi-provider routing, sessions, and MCP.";

#[derive(Parser, Debug)]
#[command(name = "orion", version, about = ABOUT, long_about = None)]
struct Cli {
    /// Working directory (default: current dir)
    #[arg(short, long, global = true)]
    dir: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Initialize an ORION project in the current directory (writes AGENTS.md)
    Init,

    /// Run a single prompt headlessly and print the response (streaming)
    Run {
        /// The prompt to send to the model
        prompt: String,

        /// Override the active model (format: provider:model)
        #[arg(long)]
        model: Option<String>,
    },

    /// Start the HTTP server (orion-server-compatible) on a port
    Serve {
        /// Port to bind
        #[arg(short, long, default_value = "7337")]
        port: u16,
    },

    /// Start the ACP (Agent Client Protocol) server over stdio (for editor integrations)
    Acp,

    /// List configured providers and which have API keys
    Providers,

    /// Save an API key for a provider (reads from stdin)
    Connect {
        /// Provider id (openai, anthropic, minimax, openrouter, …)
        provider: String,
    },

    /// OAuth-style login for a provider (prints URL, reads credential from stdin)
    Login {
        /// Provider id (anthropic, openai, google, …)
        provider: String,

        /// Credential to save (skips interactive prompt)
        credential: Option<String>,
    },

    /// Logout (remove a provider's stored credential)
    Logout {
        /// Provider id
        provider: String,
    },

    /// Toggle the master "full access" switch (Trust Engine off → allow
    /// everything with no prompts). Run with no arg to show current state.
    FullAccess {
        /// `on` or `off`. Omit to print the current setting.
        state: Option<String>,
    },

    /// List recent chat sessions
    Sessions,

    /// List configured agents (Build/Plan/Explore/Scout/General)
    Agents {
        /// Subcommand: list, activate <id>, cycle
        action: Option<String>,
    },

    /// Show usage stats (tokens, cost) — `--days N`, `--tools`, `--models`, `--project <name>`
    Stats {
        /// Days to look back (default: 30)
        #[arg(long, default_value = "30")]
        days: i64,

        /// Only show per-tool breakdown
        #[arg(long)]
        tools: bool,

        /// Only show per-model breakdown
        #[arg(long)]
        models: bool,

        /// Filter by project name
        #[arg(long)]
        project: Option<String>,
    },

    /// Export a session to JSON (with optional GitHub Gist publish)
    Share {
        /// Session id
        session_id: String,

        /// Sanitize (redact secrets + truncate paths)
        #[arg(long)]
        sanitize: bool,

        /// Path to write the JSON file (defaults to stdout)
        #[arg(long)]
        out: Option<PathBuf>,

        /// Publish to GitHub Gist (requires GITHUB_TOKEN env var)
        #[arg(long)]
        gist: bool,

        /// Make the gist public (default: secret)
        #[arg(long)]
        public: bool,
    },

    /// Import a session from a JSON file
    Import {
        /// Path to the JSON file
        path: PathBuf,

        /// New title for the imported session
        #[arg(long)]
        title: Option<String>,
    },

    /// Manage shared team memory facts / conventions
    Memory {
        /// Action to perform: add, search, list
        action: String,

        /// Text of the fact (for 'add') or query string (for 'search')
        arg: Option<String>,

        /// Optional scope (e.g. "orion-core")
        #[arg(long, default_value = "project")]
        scope: String,

        /// Optional tags in comma-separated form (e.g. "db,sqlite")
        #[arg(long)]
        tags: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Some(dir) = &cli.dir {
        std::env::set_current_dir(dir)
            .with_context(|| format!("changing to {}", dir.display()))?;
    }

    match cli.command {
        None => run_tui_fallback().await,
        Some(Commands::Init) => cmd_init().await,
        Some(Commands::Run { prompt, model }) => cmd_run(&prompt, model.as_deref()).await,
        Some(Commands::Serve { port }) => cmd_serve(port).await,
        Some(Commands::Acp) => cmd_acp().await,
        Some(Commands::Providers) => cmd_providers().await,
        Some(Commands::Connect { provider }) => cmd_connect(&provider).await,
        Some(Commands::Login { provider, credential }) => cmd_login(&provider, credential.as_deref()).await,
        Some(Commands::Logout { provider }) => cmd_logout(&provider).await,
        Some(Commands::FullAccess { state }) => cmd_full_access(state.as_deref()).await,
        Some(Commands::Sessions) => cmd_sessions().await,
        Some(Commands::Agents { action }) => cmd_agents(action.as_deref()).await,
        Some(Commands::Stats { days, tools, models, project }) => cmd_stats(days, tools, models, project.as_deref()).await,
        Some(Commands::Share { session_id, sanitize, out, gist, public }) => {
            cmd_share(&session_id, sanitize, out.as_ref(), gist, public).await
        }
        Some(Commands::Import { path, title }) => cmd_import(&path, title.as_deref()).await,
        Some(Commands::Memory { action, arg, scope, tags }) => {
            cmd_memory(&action, arg.as_deref(), &scope, tags.as_deref()).await
        }
    }
}

async fn run_tui_fallback() -> Result<()> {
    eprintln!("orion: launching TUI (no subcommand given)…");
    eprintln!("hint: use `orion run \"<prompt>\"` for headless, `orion serve` for HTTP.");
    let candidates = ["orion-tui", "cargo"];
    for cand in &candidates {
        let mut cmd = Command::new(cand);
        if *cand == "cargo" {
            cmd.args(["run", "-p", "orion-tui", "--quiet"]);
        }
        match cmd.status() {
            Ok(s) if s.success() => return Ok(()),
            _ => continue,
        }
    }
    anyhow::bail!(
        "could not launch TUI. Install it with `cargo install --path apps/orion-tui` or run `orion-tui` directly."
    );
}

async fn cmd_init() -> Result<()> {
    let cwd = std::env::current_dir()?;
    let agents_md = cwd.join("AGENTS.md");

    if agents_md.exists() {
        eprintln!(
            "AGENTS.md already exists at {} — skipping (delete it first to re-init).",
            agents_md.display()
        );
        return Ok(());
    }

    let project_name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("project")
        .to_string();

    let mut markers: Vec<&'static str> = Vec::new();
    for marker in ["Cargo.toml", "package.json", "pyproject.toml", "go.mod", "pom.xml", "build.gradle"] {
        if cwd.join(marker).exists() {
            markers.push(marker);
        }
    }

    let body = format!(
        "# {project_name}\n\n\
         AI agent configuration for this project. ORION reads this file at the start of every session.\n\n\
         ## Stack\n\n\
         {stack}\n\n\
         ## Conventions\n\n\
         - Be concise. Prefer direct edits over long explanations.\n\
         - Match existing style and patterns.\n\
         - Don't add dependencies without being asked.\n\n\
         ## What NOT to do\n\n\
         - Never commit secrets, API keys, or .env files.\n\
         - Never run `rm -rf` outside the project directory.\n\
         - Never push to main without explicit confirmation.\n",
        project_name = project_name,
        stack = if markers.is_empty() {
            "- (no project markers detected)".to_string()
        } else {
            format!("Detected: {}", markers.join(", "))
        },
    );

    std::fs::write(&agents_md, body)?;
    println!("✓ wrote {}", agents_md.display());
    Ok(())
}

async fn cmd_run(prompt: &str, model_override: Option<&str>) -> Result<()> {
    init_tracing();

    let config = Config::load()?;
    let mut agent = Agent::new(config.clone()).await?;

    if let Some(m) = model_override {
        let (provider_id, model_id) = m
            .split_once(':')
            .with_context(|| format!("model override '{m}' must be provider:model"))?;
        let full_id = format!("{provider_id}:{model_id}");
        let model = agent
            .catalog
            .get_model(&full_id)
            .with_context(|| format!("model '{full_id}' not found in catalog"))?;
        *agent.current_model.lock() = model;
        eprintln!("[orion] using model: {full_id}");
    }

    agent.send_message(prompt).await;
    Ok(())
}

async fn cmd_serve(port: u16) -> Result<()> {
    init_tracing();

    // The HTTP server is provided by apps/orion-server. We exec it as a subprocess
    // so this CLI doesn't need to duplicate axum/tower wiring.
    eprintln!("orion: delegating to orion-server on port {port}…");
    let status = Command::new("orion-server")
        .env("ORION_PORT", port.to_string())
        .status();
    match status {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => {
            eprintln!("orion-server exited with status {s}");
            Ok(())
        }
        Err(e) => {
            eprintln!(
                "could not exec `orion-server` ({e}). Install it with `cargo install --path apps/orion-server`."
            );
            // Fallback: serve in-process using orion-core
            serve_in_process(port).await
        }
    }
}

async fn serve_in_process(port: u16) -> Result<()> {
    use orion_core::server::AppState;
    use orion_core::{build_router, memory::MemoryStore, middleware::TokenOptimizer};
    use std::net::SocketAddr;
    use std::sync::Arc;

    let catalog = Arc::new(ModelCatalog::new()?);
    let registry = Arc::new(ProviderRegistry::new(catalog.clone()));
    registry.load_from_catalog();

    let memory = Arc::new(MemoryStore::new()?);
    let token_optimizer = Arc::new(TokenOptimizer::new()?);
    let state = AppState {
        registry,
        catalog,
        memory,
        token_optimizer,
    };

    let app = build_router(state);
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse()?;
    eprintln!("orion-server (in-process) listening on http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn cmd_acp() -> Result<()> {
    let config = Arc::new(Config::load().unwrap_or_default());
    run_acp_server(config).await
}

async fn cmd_providers() -> Result<()> {
    let catalog = ModelCatalog::new()?;
    let providers = catalog.list_providers();
    if providers.is_empty() {
        println!("No providers registered. Run `orion connect <id>` to add one.");
        return Ok(());
    }
    println!("{:<14}  {:<24}  {:<8}  KEY", "ID", "NAME", "ENABLED");
    println!("{}", "-".repeat(60));
    for p in providers {
        let key_status = if catalog.get_api_key(&p.id).is_some() {
            "✓"
        } else if p.api_key_env.as_ref().and_then(|k| std::env::var(k).ok()).is_some() {
            "✓ (env)"
        } else {
            "—"
        };
        println!(
            "{:<14}  {:<24}  {:<8}  {}",
            p.id,
            p.name,
            if p.enabled { "yes" } else { "no" },
            key_status,
        );
    }
    Ok(())
}

async fn cmd_full_access(state: Option<&str>) -> Result<()> {
    let catalog = ModelCatalog::new()?;
    match state.map(|s| s.trim().to_lowercase()).as_deref() {
        Some("on") | Some("true") | Some("yes") | Some("1") => {
            catalog.set_config("full_access", "true")?;
            println!("✓ full access ON — ORION will run every tool with no prompts.");
            println!("  ⚠ rm, curl, writes to /etc and /home are all allowed. Use with care.");
        }
        Some("off") | Some("false") | Some("no") | Some("0") => {
            catalog.set_config("full_access", "false")?;
            println!("✓ full access OFF — the Trust Engine asks only for risky actions.");
        }
        None => {
            let on = catalog.get_bool_config("full_access");
            println!("full access is {}", if on { "ON" } else { "OFF" });
        }
        Some(other) => anyhow::bail!("expected `on` or `off`, got `{other}`"),
    }
    Ok(())
}

async fn cmd_connect(provider: &str) -> Result<()> {
    use std::io::{self, BufRead, Write};
    let catalog = ModelCatalog::new()?;

    let known = catalog.list_providers().into_iter().find(|p| p.id == provider);
    let known = match known {
        Some(p) => p,
        None => {
            eprintln!("Unknown provider '{}'. Run `orion providers` to see valid ids.", provider);
            std::process::exit(1);
        }
    };

    eprint!("API key for {} ({}): ", known.name, known.id);
    io::stderr().flush().ok();
    let stdin = io::stdin();
    let line = stdin.lock().lines().next().context("no input")??;
    let key = line.trim();
    if key.is_empty() {
        anyhow::bail!("empty key, aborting");
    }

    catalog.save_api_key(&known.id, key)?;
    catalog.set_provider_enabled(&known.id, true)?;
    println!("✓ saved API key for {} (length: {} chars)", known.id, key.len());
    println!("hint: restart any running orion-desktop / orion-server to pick up the change.");
    Ok(())
}

async fn cmd_login(provider: &str, credential: Option<&str>) -> Result<()> {
    orion_core::oauth::login(provider, credential).await
}

async fn cmd_logout(provider: &str) -> Result<()> {
    let catalog = ModelCatalog::new()?;
    catalog.save_api_key(provider, "")?;
    catalog.set_provider_enabled(provider, false)?;
    println!("✓ removed credential for {provider}");
    Ok(())
}

async fn cmd_agents(action: Option<&str>) -> Result<()> {
    use orion_core::agents::{AgentMode, AgentRegistry};
    let registry = AgentRegistry::with_builtins();
    match action {
        None | Some("list") => {
            println!("{:<10}  {:<10}  {:<22}  {}", "ID", "MODE", "NAME", "DESCRIPTION");
            println!("{}", "-".repeat(78));
            let mut specs = registry.list_visible();
            specs.sort_by(|a, b| a.id.cmp(&b.id));
            for spec in specs {
                let mode = match spec.mode {
                    AgentMode::Primary => "primary",
                    AgentMode::Subagent => "subagent",
                };
                println!(
                    "{:<10}  {:<10}  {:<22}  {}",
                    spec.id,
                    mode,
                    truncate(&spec.name, 20),
                    truncate(&spec.description, 60)
                );
            }
        }
        Some("cycle") => {
            if let Some(next) = registry.cycle_next() {
                println!("✓ active agent: {} ({})", next.id, next.name);
            }
        }
        Some(id) => {
            if registry.activate(id) {
                if let Some(spec) = registry.get(id) {
                    println!("✓ active agent: {} ({})", spec.id, spec.name);
                }
            } else {
                eprintln!("Unknown agent id '{id}'. Run `orion agents list`.");
                std::process::exit(1);
            }
        }
    }
    Ok(())
}

async fn cmd_stats(days: i64, _tools: bool, _models: bool, project: Option<&str>) -> Result<()> {
    use orion_core::stats::{format_snapshot, StatsFilter, StatsStore};
    let store = StatsStore::user_default().context("opening stats DB")?;
    let mut filter = StatsFilter::last_n_days(days);
    if let Some(p) = project {
        filter.project = Some(p.to_string());
    }
    let snap = store.snapshot(&filter)?;
    print!("{}", format_snapshot(&snap));
    Ok(())
}

async fn cmd_share(
    session_id: &str,
    sanitize: bool,
    out_path: Option<&PathBuf>,
    gist: bool,
    public: bool,
) -> Result<()> {
    use orion_core::share;
    let catalog = ModelCatalog::new()?;
    let session = share::export_session(&catalog, session_id, sanitize)?;
    if gist {
        let token = std::env::var("GITHUB_TOKEN")
            .with_context(|| "GITHUB_TOKEN env var required for --gist")?;
        let url = share::publish_gist(&session, &token, public).await?;
        println!("✓ published gist: {}", url);
        return Ok(());
    }
    if let Some(path) = out_path {
        share::write_to_file(&session, path)?;
        println!("✓ wrote {}", path.display());
    } else {
        let json = serde_json::to_string_pretty(&session)?;
        println!("{json}");
    }
    Ok(())
}

async fn cmd_import(path: &PathBuf, title: Option<&str>) -> Result<()> {
    use orion_core::share;
    let catalog = ModelCatalog::new()?;
    let session = share::read_from_file(path)?;
    let new_id = share::import_session(&catalog, &session, title)?;
    println!("✓ imported as session {new_id}");
    Ok(())
}

async fn cmd_memory(
    action: &str,
    arg: Option<&str>,
    scope: &str,
    tags: Option<&str>,
) -> Result<()> {
    use orion_core::memory::team::{project_team_memory_path, TeamMemory, TeamMemoryEntry};

    let path = project_team_memory_path();
    let mut mem = TeamMemory::load_from_file(&path)?;

    match action {
        "add" => {
            let text = arg.ok_or_else(|| anyhow::anyhow!("Fact text required for 'add'"))?;
            let author = std::env::var("USER")
                .or_else(|_| std::env::var("USERNAME"))
                .unwrap_or_else(|_| "anonymous".to_string());
            let tags_vec = tags
                .map(|t| t.split(',').map(|s| s.trim().to_string()).collect())
                .unwrap_or_default();
            let entry = TeamMemoryEntry {
                id: uuid::Uuid::new_v4().to_string(),
                author,
                scope: scope.to_string(),
                tags: tags_vec,
                text: text.to_string(),
                ts: chrono::Utc::now().to_rfc3339(),
            };
            mem.append_to_file(&path, entry)?;
            println!("✓ fact added to team memory ({})", path.display());
        }
        "search" => {
            let query = arg.ok_or_else(|| anyhow::anyhow!("Query string required for 'search'"))?;
            let hits = mem.search(query);
            if hits.is_empty() {
                println!("No facts matching '{query}'.");
            } else {
                println!("Found {} matching fact(s):", hits.len());
                for hit in hits {
                    let tags_str = if hit.tags.is_empty() {
                        String::new()
                    } else {
                        format!(" [{}]", hit.tags.join(", "))
                    };
                    println!(
                        "  [{}] ({}) by {}{}: {}",
                        &hit.id[..8.min(hit.id.len())],
                        hit.scope,
                        hit.author,
                        tags_str,
                        hit.text
                    );
                }
            }
        }
        "list" => {
            let list = mem.list();
            if list.is_empty() {
                println!("No team memory facts found at {}", path.display());
            } else {
                println!("{} team memory fact(s) loaded from {}:", list.len(), path.display());
                for hit in list {
                    let tags_str = if hit.tags.is_empty() {
                        String::new()
                    } else {
                        format!(" [{}]", hit.tags.join(", "))
                    };
                    println!(
                        "  [{}] ({}) by {}{}: {}",
                        &hit.id[..8.min(hit.id.len())],
                        hit.scope,
                        hit.author,
                        tags_str,
                        hit.text
                    );
                }
            }
        }
        other => anyhow::bail!("Unknown memory action '{other}'. Valid: add, search, list"),
    }
    Ok(())
}

async fn cmd_sessions() -> Result<()> {
    let catalog = ModelCatalog::new()?;
    let sessions = catalog.list_sessions();
    if sessions.is_empty() {
        println!("No sessions yet. Start one with `orion-tui` or the desktop app.");
        return Ok(());
    }
    println!("{:<10}  {:<32}  {:<10}  {}", "ID", "TITLE", "MSGS", "UPDATED");
    println!("{}", "-".repeat(72));
    for s in sessions.iter().take(20) {
        println!(
            "{:<10}  {:<32}  {:<10}  {}",
            &s.id[..8.min(s.id.len())],
            truncate(&s.title, 30),
            s.message_count,
            s.updated_at,
        );
    }
    if sessions.len() > 20 {
        println!("… and {} more", sessions.len() - 20);
    }
    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

fn init_tracing() {
    use tracing_subscriber::EnvFilter;
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "warn".into()))
        .try_init();
}