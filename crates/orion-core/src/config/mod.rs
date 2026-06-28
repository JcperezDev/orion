use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub default_model: String,
    pub max_tokens: usize,
    pub max_history_messages: usize,
    pub provider: String,
    pub api_url: String,
    pub watch_folder: Option<std::path::PathBuf>,
    pub theme: String,
    pub mcp_servers: Vec<McpServerConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    pub url: String,
    pub enabled: bool,
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let config_path = Self::config_path()?;

        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            let config: Config = toml::from_str(&content)?;
            Ok(config)
        } else {
            let config = Self::default_config();
            config.save()?;
            Ok(config)
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::config_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    fn config_path() -> anyhow::Result<std::path::PathBuf> {
        let dir = dirs::config_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("orion");
        Ok(dir.join("config.toml"))
    }

    fn default_config() -> Self {
        Self {
            default_model: "claude-3-5-sonnet-20241022".to_string(),
            max_tokens: 4096,
            max_history_messages: 100,
            provider: "anthropic".to_string(),
            api_url: "".to_string(),
            watch_folder: None,
            theme: "dark".to_string(),
            mcp_servers: vec![],
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::default_config()
    }
}
