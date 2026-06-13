pub mod event;
pub mod update;

use crate::config::Config;
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct AppState {
    pub input_buffer: String,
    pub messages: Vec<crate::ui::chat::Message>,
    pub active_panel: Option<String>,
    pub scroll_offset: usize,
    pub connected_mcps: Vec<String>,
    pub current_model: String,
    pub token_count: usize,
    pub cost_total: f64,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            input_buffer: String::new(),
            messages: Vec::new(),
            active_panel: None,
            scroll_offset: 0,
            connected_mcps: Vec::new(),
            current_model: "claude".to_string(),
            token_count: 0,
            cost_total: 0.0,
        }
    }
}

pub struct App {
    pub state: AppState,
    pub config: Config,
}

impl App {
    pub async fn new(config: Config) -> Result<Self> {
        Ok(Self {
            state: AppState::new(),
            config,
        })
    }

    pub fn handle_input(&mut self, input: String) {
        if input.starts_with('/') {
            self.handle_command(&input);
        } else {
            self.state.input_buffer = input;
        }
    }

    fn handle_command(&mut self, cmd: &str) {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        match parts[0] {
            "/clear" => self.state.messages.clear(),
            "/tokens" => {}
            "/stats" => {}
            "/help" => {}
            _ => {}
        }
    }

    pub fn tick(&mut self) {}
}

pub use event::EventLoop;
