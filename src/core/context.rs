use crate::config::Config;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[allow(dead_code)]
pub struct Context {
    pub messages: Vec<Message>,
    pub current_model: String,
    pub max_tokens: usize,
    pub config: Config,
}

#[allow(dead_code)]
impl Context {
    pub fn new(config: Config) -> Self {
        Self {
            messages: Vec::new(),
            current_model: config.default_model.clone(),
            max_tokens: config.max_tokens,
            config,
        }
    }

    pub fn add_message(&mut self, role: &str, content: &str) {
        self.messages.push(Message {
            role: role.to_string(),
            content: content.to_string(),
        });
    }

    pub fn get_messages(&self) -> Vec<serde_json::Value> {
        self.messages
            .iter()
            .map(|m| {
                serde_json::json!({
                    "role": m.role,
                    "content": m.content
                })
            })
            .collect()
    }

    pub async fn set_model(&mut self, model: String) {
        self.current_model = model;
    }

    pub fn trim_history(&mut self) {
        let max_messages = self.config.max_history_messages;
        if self.messages.len() > max_messages {
            let drain_count = self.messages.len() - max_messages;
            self.messages.drain(0..drain_count);
        }
    }
}
