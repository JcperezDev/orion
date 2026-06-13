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
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use app::{App, EventLoop};
use app::event::EventType;
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
        let backend = CrosstermBackend::new(std::io::stderr());
        let mut terminal = Terminal::new(backend)?;

        loop {
            terminal.draw(|f| ui::render(f, &self.app))?;

            match EventLoop::new().next_event().await {
                EventType::Input(input) => {
                    let input_clone = input.clone();
                    self.app.handle_input(input);
                    self.agent.process_input(&input_clone).await;
                }
                EventType::Tick => {
                    self.app.tick();
                }
                EventType::Quit => break,
            }
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut orion = Orion::new().await?;
    orion.run().await?;
    Ok(())
}
