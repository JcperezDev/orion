mod analytics;
mod app;
mod config;
mod core;
mod images;
mod mcp;
mod models;
mod providers;
mod router;
mod ui;

use anyhow::Result;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use app::event::{EventType, InputEvent};
use app::{App, EventLoop};
use config::Config;
use core::agent::Agent;

pub struct Orion {
    pub app: App,
    pub agent: Agent,
    pub config: Config,
}

impl Orion {
    pub async fn new() -> Result<Self> {
        let config = Config::load()?;
        let agent = Agent::new(config.clone()).await?;
        let app = App::new(config.clone()).await?;

        Ok(Self { app, agent, config })
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

            match EventLoop::new().next_event().await {
                EventType::Input(input) => {
                    if matches!(input, InputEvent::CtrlC | InputEvent::CtrlQ) {
                        self.app.state.add_message("system", "Goodbye!".to_string());
                        break;
                    }
                    self.app.handle_input(input);
                }
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
