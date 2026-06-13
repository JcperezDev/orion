pub mod event;
pub mod update;

use orion_core::config::Config;
use crate::ui::chat::Message;
use anyhow::Result;
use chrono::Utc;

#[derive(Debug, Clone)]
pub struct AppState {
    pub input_buffer: String,
    pub messages: Vec<Message>,
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

    pub fn add_message(&mut self, role: &str, content: String) {
        self.messages.push(Message {
            role: role.to_string(),
            content,
            timestamp: Utc::now(),
        });
    }
}

pub struct App {
    pub state: AppState,
    pub config: Config,
}

impl App {
    pub async fn new(config: Config) -> Result<Self> {
        let mut state = AppState::new();
        state.add_message(
            "system",
            "ORION ready. Type /help for available commands.".to_string(),
        );
        Ok(Self { state, config })
    }

    pub fn handle_input(&mut self, input: event::InputEvent) {
        match input {
            event::InputEvent::Char(c) => {
                self.state.input_buffer.push(c);
            }
            event::InputEvent::Enter => {
                let input = self.state.input_buffer.trim().to_string();
                if !input.is_empty() {
                    self.state.add_message("user", input.clone());
                    self.execute_command(&input);
                }
                self.state.input_buffer.clear();
            }
            event::InputEvent::Backspace => {
                self.state.input_buffer.pop();
            }
            event::InputEvent::CtrlC | event::InputEvent::CtrlQ => {
                self.state.add_message("system", "Goodbye!".to_string());
            }
            event::InputEvent::Unknown => {}
        }
    }

    fn execute_command(&mut self, cmd: &str) {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        match parts[0] {
            "/clear" => self.state.messages.retain(|m| m.role != "user"),
            "/help" => {
                self.state.add_message(
                    "system",
                    "Available commands:\n\
                     /help - Show this help\n\
                     /clear - Clear chat\n\
                     /providers list - List providers\n\
                     /providers status - Show provider status\n\
                     /models sources - List model sources\n\
                     /models sync - Sync models\n\
                     /models list - List all models\n\
                     /models search <query> - Search models\n\
                     /models vision - Vision-capable models\n\
                     /models tools - Tool-capable models\n\
                     /models local - Local models\n\
                     /best coding - Best model for coding\n\
                     /best vision - Best model for vision\n\
                     /best local - Best local model"
                        .to_string(),
                );
            }
            _ => {
                self.state.add_message(
                    "system",
                    format!(
                        "Unknown command: {}. Type /help for available commands.",
                        cmd
                    ),
                );
            }
        }
    }

    pub fn tick(&mut self) {}
}

pub use event::EventLoop;
