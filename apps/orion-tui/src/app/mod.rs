pub mod event;
pub mod update;

use crate::ui::chat::Message;
use anyhow::Result;
use chrono::Utc;
use orion_core::core::dispatch::{DispatchEvent, Dispatcher};
use orion_core::providers::registry::ProviderRegistry;
use orion_core::providers::traits::Message as LlmMessage;
use std::sync::Arc;
use tokio::sync::mpsc;

use self::event::InputEvent;

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
    pub is_processing: bool,
    pub cancel_requested: bool,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            input_buffer: String::new(),
            messages: Vec::new(),
            active_panel: None,
            scroll_offset: 0,
            connected_mcps: vec!["filesystem".to_string()],
            current_model: String::new(),
            token_count: 0,
            cost_total: 0.0,
            is_processing: false,
            cancel_requested: false,
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
    dispatch_rx: Option<mpsc::UnboundedReceiver<DispatchEvent>>,
}

impl App {
    pub async fn new(config: orion_core::config::Config) -> Result<Self> {
        let mut state = AppState::new();
        state.current_model = config.default_model.clone();
        state.add_message(
            "system",
            "ORION ready. Type /help for available commands.".to_string(),
        );
        Ok(Self {
            state,
            dispatch_rx: None,
        })
    }

    /// Handle key events — manages input_buffer.
    pub fn handle_key_event(&mut self, input: InputEvent) {
        match input {
            InputEvent::Char(c) => {
                self.state.input_buffer.push(c);
            }
            InputEvent::Enter => {}
            InputEvent::Backspace => {
                self.state.input_buffer.pop();
            }
            InputEvent::CtrlC | InputEvent::CtrlQ => {}
            InputEvent::Unknown => {}
        }
    }

    /// Called when Enter is pressed. Spawns the dispatcher if the input is not a command.
    pub async fn handle_submit(
        &mut self,
        registry: &Arc<ProviderRegistry>,
        dispatcher: &Arc<Dispatcher>,
    ) {
        if self.state.is_processing {
            return;
        }

        let input = self.state.input_buffer.trim().to_string();
        if input.is_empty() {
            return;
        }
        self.state.input_buffer.clear();

        if input.starts_with('/') {
            self.state.add_message("user", input.clone());
            self.execute_command(&input);
            return;
        }

        self.state.add_message("user", input.clone());
        self.state.add_message("assistant", String::new());
        self.state.is_processing = true;
        self.state.cancel_requested = false;

        let (tx, rx) = mpsc::unbounded_channel();
        self.dispatch_rx = Some(rx);

        let dispatcher = dispatcher.clone();
        let registry = registry.clone();

        tokio::spawn(async move {
            let active = registry.catalog().get_default_model();
            let (provider_id, model_id) = match active {
                Some(m) => (m.provider_id.clone(), m.model_id.clone()),
                None => {
                    let _ = tx.send(DispatchEvent::Error(
                        "No active model configured".to_string(),
                    ));
                    return;
                }
            };

            let provider = match registry.get_or_create(&provider_id) {
                Some(p) => p,
                None => {
                    let _ = tx.send(DispatchEvent::Error(format!(
                        "Provider not available: {provider_id}"
                    )));
                    return;
                }
            };

            let messages = vec![LlmMessage {
                role: "user".to_string(),
                content: input,
                ..Default::default()
            }];

            let events = match dispatcher
                .run(provider, &provider_id, &model_id, messages)
                .await
            {
                Ok(e) => e,
                Err(e) => {
                    let _ = tx.send(DispatchEvent::Error(e.to_string()));
                    return;
                }
            };

            for ev in events {
                if tx.send(ev).is_err() {
                    break;
                }
            }
        });
    }

    /// Poll the dispatch channel and apply events. Called from the main loop.
    pub fn drain_dispatch_events(&mut self) {
        let Some(rx) = &mut self.dispatch_rx else { return };

        // Collect available events first to avoid borrow conflicts.
        let mut events = Vec::new();
        loop {
            match rx.try_recv() {
                Ok(ev) => events.push(ev),
                Err(mpsc::error::TryRecvError::Empty) => break,
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    self.dispatch_rx = None;
                    break;
                }
            }
        }

        for ev in events {
            self.apply_dispatch_event(ev);
        }
    }

    fn apply_dispatch_event(&mut self, ev: DispatchEvent) {
        match ev {
            DispatchEvent::Token(text) => {
                let token_count = text.len() / 4;
                let msgs = &mut self.state.messages;
                if let Some(idx) = msgs.iter().rposition(|m| m.role == "assistant") {
                    msgs[idx].content.push_str(&text);
                } else {
                    self.state.add_message("assistant", text);
                }
                self.state.token_count += token_count;
            }
            DispatchEvent::ToolCall(call) => {
                self.state
                    .add_message("system", format!("🔧 tool: {} ({})", call.name, call.id));
            }
            DispatchEvent::ToolResult {
                content,
                is_error,
                ..
            } => {
                let prefix = if is_error { "❌" } else { "✓" };
                let truncated = if content.len() > 200 {
                    format!("{}... [{} bytes]", &content[..200], content.len())
                } else {
                    content
                };
                self.state
                    .add_message("system", format!("{prefix} result: {truncated}"));
            }
            DispatchEvent::StepSnapshot(_) => {}
            DispatchEvent::Undoable { .. } => {}
            DispatchEvent::Retrying { attempt, delay_secs, .. } => {
                self.state.add_message(
                    "system",
                    format!("⏳ provider busy — retry {attempt} in {delay_secs}s…"),
                );
            }
            DispatchEvent::LimitReached { retry_after_secs, message } => {
                let when = retry_after_secs
                    .map(|s| format!(" Resets in ~{s}s."))
                    .unwrap_or_default();
                self.state.add_message(
                    "system",
                    format!("🛑 Usage limit reached.{when} Work checkpointed — resume later. ({message})"),
                );
                self.state.is_processing = false;
            }
            DispatchEvent::Done { final_text, .. } => {
                if !final_text.is_empty() {
                    self.state.add_message("assistant", final_text);
                }
                self.state.is_processing = false;
            }
            DispatchEvent::Error(msg) => {
                self.state.add_message("error", msg);
                self.state.is_processing = false;
            }
        }
    }

    fn execute_command(&mut self, cmd: &str) {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        match parts[0] {
            "/clear" => self.state.messages.clear(),
            "/help" => {
                self.state.add_message(
                    "system",
                    "Available commands:\n\
                     /help - Show this help\n\
                     /clear - Clear chat\n\
                     /providers - List providers\n\
                     /model - Show active model"
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
