use anyhow::Result;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Token-optimizer middleware. Wires the token-god MCP server into the
/// request pipeline. Today: in-memory stats and config; ready to call
/// `compress_context` / `summarize_history` via MCP once that client is wired.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenStats {
    pub used: u64,
    pub budget: u64,
    pub saved: u64,
    pub by_session: HashMap<String, u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizerConfig {
    pub enabled: bool,
    pub max_context_tokens: u64,
    pub auto_compress_threshold: f32,
    pub budget_per_session: u64,
}

impl Default for OptimizerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_context_tokens: 100_000,
            auto_compress_threshold: 0.75,
            budget_per_session: 200_000,
        }
    }
}

pub struct TokenOptimizer {
    config: Mutex<OptimizerConfig>,
    stats: Mutex<TokenStats>,
}

impl TokenOptimizer {
    pub fn new() -> Result<Self> {
        Ok(Self {
            config: Mutex::new(OptimizerConfig::default()),
            stats: Mutex::new(TokenStats {
                used: 0,
                budget: 200_000,
                saved: 0,
                by_session: HashMap::new(),
            }),
        })
    }

    pub fn config(&self) -> OptimizerConfig {
        self.config.lock().clone()
    }

    pub fn set_config(&self, cfg: OptimizerConfig) {
        *self.config.lock() = cfg.clone();
        self.stats.lock().budget = cfg.budget_per_session;
    }

    pub fn stats(&self) -> TokenStats {
        self.stats.lock().clone()
    }

    pub fn record(&self, session_id: &str, tokens: u64) {
        let mut s = self.stats.lock();
        s.used = s.used.saturating_add(tokens);
        *s.by_session.entry(session_id.to_string()).or_insert(0) += tokens;
    }

    /// Compress context if over the configured threshold.
    /// Wire point: when the MCP client is integrated, call
    /// `mcp_client.call("token-god", "compress_context", &payload)` here.
    pub fn maybe_compress(&self, _session_id: &str, _context_tokens: u64) -> bool {
        let cfg = self.config.lock();
        if !cfg.enabled {
            return false;
        }
        let limit = (cfg.max_context_tokens as f32 * cfg.auto_compress_threshold) as u64;
        _context_tokens > limit
    }
}
