use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use crate::core::context::Context;

#[derive(Clone)]
#[allow(dead_code)]
pub struct McpServer {
    pub name: String,
    pub url: String,
    pub enabled: bool,
}

#[allow(dead_code)]
pub struct McpManager {
    pub servers: HashMap<String, McpServer>,
    pub pre_request_hooks: Vec<PreRequestHook>,
}

pub type PreRequestHook = Arc<dyn Fn(&mut Context) -> Result<()> + Send + Sync>;

#[allow(dead_code)]
impl McpManager {
    pub fn new() -> Self {
        let mut manager = Self {
            servers: HashMap::new(),
            pre_request_hooks: Vec::new(),
        };
        manager.register_default_hooks();
        manager
    }

    fn register_default_hooks(&mut self) {
        let token_god_hook: PreRequestHook = Arc::new(|ctx: &mut Context| {
            let max_in_flight = 50;
            let msg_count = ctx.messages.len();
            if msg_count > max_in_flight {
                ctx.trim_history();
            }
            Ok(())
        });
        self.pre_request_hooks.push(token_god_hook);
    }

    pub fn get_pre_request_hooks(&self) -> Vec<PreRequestHook> {
        self.pre_request_hooks.clone()
    }

    pub async fn connect(&mut self, name: &str) {
        let server = McpServer {
            name: name.to_string(),
            url: format!("http://localhost:{}", self.get_default_port(name)),
            enabled: true,
        };
        self.servers.insert(name.to_string(), server);
    }

    fn get_default_port(&self, name: &str) -> u16 {
        match name {
            "filesystem" => 3000,
            "github" => 3001,
            "brave-search" => 3002,
            "postgres" => 3003,
            _ => 3000,
        }
    }

    pub fn disconnect(&mut self, name: &str) {
        self.servers.remove(name);
    }

    pub fn list_tools(&self) -> Vec<String> {
        self.servers
            .keys()
            .map(|k| format!("{} ({})", k, if self.servers.get(k).map(|s| s.enabled).unwrap_or(false) { "connected" } else { "disconnected" }))
            .collect()
    }

    pub fn is_connected(&self, name: &str) -> bool {
        self.servers.get(name).map(|s| s.enabled).unwrap_or(false)
    }
}

impl Default for McpManager {
    fn default() -> Self {
        Self::new()
    }
}
