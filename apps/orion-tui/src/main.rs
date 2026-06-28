mod app;
mod ui;

use anyhow::Result;
use orion_core::core::dispatch::{DispatchConfig, Dispatcher};
use orion_core::SpillManager;
use orion_core::models::catalog::ModelCatalog;
use orion_core::permissions::{PermissionConfig, PermissionEngine};
use orion_core::providers::registry::ProviderRegistry;
use orion_core::tools::builtin_registry;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::sync::Arc;

use app::event::{EventType, InputEvent};
use app::{App, EventLoop};
use orion_core::config::Config;

pub struct Orion {
    pub app: App,
    pub config: Config,
    pub registry: Arc<ProviderRegistry>,
    pub dispatcher: Arc<Dispatcher>,
}

impl Orion {
    pub async fn new() -> Result<Self> {
        let config = Config::load()?;

        let catalog = Arc::new(ModelCatalog::new().map_err(|e| anyhow::anyhow!("{}", e))?);
        let registry = Arc::new(ProviderRegistry::new(catalog.clone()));
        registry.load_from_catalog();

        let tools = Arc::new(builtin_registry());
        let permissions = Arc::new(PermissionEngine::new(PermissionConfig::safe_defaults()));
        let cwd = std::env::current_dir().unwrap_or_default();
        let full_access = catalog.get_bool_config("full_access");
        let mut dispatch_config = DispatchConfig::new(cwd.clone())
            .with_spill(SpillManager::new_temp())
            .with_full_access(full_access);
        if let Ok(store) = orion_core::LearnedStore::open() {
            let store = Arc::new(store);
            let _ = store.hydrate(&permissions, &cwd);
            dispatch_config = dispatch_config.with_learned(store);
        }
        let dispatcher = Arc::new(Dispatcher::new(tools, permissions, dispatch_config));

        let app = App::new(config.clone()).await?;

        Ok(Self {
            app,
            config,
            registry,
            dispatcher,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        setup_terminal()?;
        let result = self.run_inner().await;
        restore_terminal();
        result
    }

    async fn run_inner(&mut self) -> Result<()> {
        use std::io;

        let backend = CrosstermBackend::new(io::stdout());
        let mut terminal = Terminal::new(backend)?;

        terminal.clear()?;
        terminal.hide_cursor()?;

        loop {
            terminal.draw(|f| ui::render(f, &self.app))?;

            if self.app.state.is_processing {
                self.app.drain_dispatch_events();
            }

            match EventLoop::new().next_event().await {
                EventType::Input(input) => match input {
                    InputEvent::CtrlC | InputEvent::CtrlQ => {
                        if self.app.state.is_processing {
                            self.app.state.cancel_requested = true;
                        } else {
                            self.app.state.add_message("system", "Goodbye!".to_string());
                            break;
                        }
                    }
                    InputEvent::Enter => {
                        self.app
                            .handle_submit(&self.registry, &self.dispatcher)
                            .await;
                    }
                    other => {
                        self.app.handle_key_event(other);
                    }
                },
                EventType::Tick => {
                    self.app.tick();
                }
            }
        }

        Ok(())
    }
}

fn setup_terminal() -> Result<()> {
    use crossterm::execute;
    use crossterm::terminal::{enable_raw_mode, EnterAlternateScreen};

    enable_raw_mode()?;
    execute!(std::io::stdout(), EnterAlternateScreen)?;
    execute!(
        std::io::stdout(),
        crossterm::terminal::Clear(crossterm::terminal::ClearType::All)
    )?;

    Ok(())
}

fn restore_terminal() {
    use crossterm::execute;
    use crossterm::terminal::{disable_raw_mode, LeaveAlternateScreen};

    let _ = execute!(std::io::stdout(), LeaveAlternateScreen);
    let _ = disable_raw_mode();
    let _ = execute!(std::io::stdout(), crossterm::cursor::Show);
}

#[tokio::main]
async fn main() -> Result<()> {
    let result = Orion::new().await;

    match result {
        Ok(mut orion) => {
            if let Err(e) = orion.run().await {
                restore_terminal();
                eprintln!("Error during runtime: {}", e);
                return Err(e);
            }
        }
        Err(e) => {
            restore_terminal();
            eprintln!("Error during initialization: {}", e);
            return Err(e);
        }
    }

    restore_terminal();
    Ok(())
}
