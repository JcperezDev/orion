use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use orion_core::config::Config;
use orion_core::core::agent::Agent;
use orion_core::models::catalog::ModelCatalog;
use orion_core::providers::registry::ProviderRegistry;
use std::path::PathBuf;
use std::process::Command;

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

    /// List configured providers and which have API keys
    Providers,

    /// Save an API key for a provider (reads from stdin)
    Connect {
        /// Provider id (openai, anthropic, minimax, openrouter, …)
        provider: String,
    },

    /// List recent chat sessions
    Sessions,
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
        Some(Commands::Providers) => cmd_providers().await,
        Some(Commands::Connect { provider }) => cmd_connect(&provider).await,
        Some(Commands::Sessions) => cmd_sessions().await,
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