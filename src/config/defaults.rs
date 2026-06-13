use crate::config::{Config, McpServerConfig};

pub fn get_default_config() -> Config {
    Config {
        default_model: "claude-3-5-sonnet-20241022".to_string(),
        max_tokens: 4096,
        max_history_messages: 100,
        api_url: "https://api.anthropic.com/v1/messages".to_string(),
        watch_folder: None,
        theme: "dark".to_string(),
        mcp_servers: vec![
            McpServerConfig {
                name: "filesystem".to_string(),
                url: "http://localhost:3000".to_string(),
                enabled: false,
            },
            McpServerConfig {
                name: "github".to_string(),
                url: "http://localhost:3001".to_string(),
                enabled: false,
            },
        ],
    }
}
