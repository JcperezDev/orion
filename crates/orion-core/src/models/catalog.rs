use anyhow::Result;
use parking_lot::Mutex;
use rusqlite::{params, Connection};
use std::sync::Arc;

/// Service name under which API keys are stored in the OS keyring.
const KEYRING_SERVICE: &str = "orion";

/// Build a keyring entry for a provider's API key. Returns `None` when no
/// keyring backend is available (e.g. headless CI), so callers fall back to
/// the DB. Honors `ORION_DISABLE_KEYRING=1` to force the DB path (tests).
fn keyring_entry(provider_id: &str) -> Option<keyring::Entry> {
    if std::env::var("ORION_DISABLE_KEYRING").as_deref() == Ok("1") {
        return None;
    }
    keyring::Entry::new(KEYRING_SERVICE, provider_id).ok()
}

#[derive(Debug, Clone)]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub kind: ProviderKind,
    pub base_url: Option<String>,
    pub api_key_env: Option<String>,
    pub enabled: bool,
    pub supports_streaming: bool,
    pub supports_tools: bool,
    pub supports_vision: bool,
    pub last_sync_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderKind {
    OpenAICompatible,
    Anthropic,
    Ollama,
    Google,
    Custom,
}

impl ProviderKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProviderKind::OpenAICompatible => "openai_compatible",
            ProviderKind::Anthropic => "anthropic",
            ProviderKind::Ollama => "ollama",
            ProviderKind::Google => "google",
            ProviderKind::Custom => "custom",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "openai_compatible" => ProviderKind::OpenAICompatible,
            "anthropic" => ProviderKind::Anthropic,
            "ollama" => ProviderKind::Ollama,
            "google" => ProviderKind::Google,
            _ => ProviderKind::Custom,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub id: String,
    pub provider_id: String,
    pub model_id: String,
    pub display_name: String,
    pub source: Option<String>,
    pub context_window: Option<u32>,
    pub max_output: Option<u32>,
    pub input_price: Option<f64>,
    pub output_price: Option<f64>,
    pub supports_vision: bool,
    pub supports_tools: bool,
    pub supports_reasoning: bool,
    pub supports_structured_output: bool,
    pub enabled: bool,
    pub is_free: bool,
    pub is_local: bool,
    pub is_available: bool,
    pub rank_overall: Option<u32>,
    pub rank_coding: Option<u32>,
    pub rank_vision: Option<u32>,
    pub updated_at: Option<String>,
}

impl ModelInfo {
    pub fn full_id(&self) -> String {
        format!("{}:{}", self.provider_id, self.model_id)
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Capability {
    pub model_id: String,
    pub capability: String,
    pub value: bool,
}

#[derive(Debug, Clone)]
pub struct ModelSource {
    pub id: String,
    pub name: String,
    pub url: String,
    pub enabled: bool,
    pub last_sync_at: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Session {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
    pub message_count: i64,
    pub active_model: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StoredMessage {
    pub role: String,
    pub content: String,
    pub created_at: String,
}

pub struct ModelCatalog {
    conn: Arc<Mutex<Connection>>,
}

impl ModelCatalog {
    pub fn new() -> Result<Self> {
        let db_path = Self::db_path()?;
        let conn = Connection::open(&db_path)?;

        // Enable WAL so concurrent readers and one writer can coexist (tests + app).
        let _ = conn.pragma_update(None, "journal_mode", "WAL");
        let _ = conn.pragma_update(None, "synchronous", "NORMAL");
        let _ = conn.pragma_update(None, "busy_timeout", "5000");

        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS providers (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                kind TEXT NOT NULL DEFAULT 'openai_compatible',
                base_url TEXT,
                api_key_env TEXT,
                enabled INTEGER NOT NULL DEFAULT 1,
                supports_streaming INTEGER NOT NULL DEFAULT 1,
                supports_tools INTEGER NOT NULL DEFAULT 1,
                supports_vision INTEGER NOT NULL DEFAULT 0,
                last_sync_at TEXT
            );

            CREATE TABLE IF NOT EXISTS models (
                id TEXT PRIMARY KEY,
                provider_id TEXT NOT NULL,
                model_id TEXT NOT NULL,
                display_name TEXT,
                context_window INTEGER,
                max_output INTEGER,
                input_price REAL,
                output_price REAL,
                supports_vision INTEGER DEFAULT 0,
                supports_tools INTEGER DEFAULT 0,
                supports_reasoning INTEGER DEFAULT 0,
                supports_structured_output INTEGER DEFAULT 0,
                enabled INTEGER DEFAULT 1,
                is_free INTEGER DEFAULT 0,
                is_local INTEGER DEFAULT 0,
                is_available INTEGER DEFAULT 1,
                rank_overall INTEGER,
                rank_coding INTEGER,
                rank_vision INTEGER,
                updated_at TEXT,
                FOREIGN KEY(provider_id) REFERENCES providers(id)
            );

            CREATE TABLE IF NOT EXISTS model_sources (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                url TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                last_sync_at TEXT,
                last_error TEXT
            );

            CREATE TABLE IF NOT EXISTS config (
                key TEXT PRIMARY KEY,
                value TEXT
            );

            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL DEFAULT 'New session',
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                message_count INTEGER NOT NULL DEFAULT 0,
                active_model TEXT
            );

            CREATE TABLE IF NOT EXISTS messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE INDEX IF NOT EXISTS idx_models_provider ON models(provider_id);
            CREATE INDEX IF NOT EXISTS idx_models_rank ON models(rank_overall);
            CREATE INDEX IF NOT EXISTS idx_sessions_updated ON sessions(updated_at DESC);
            CREATE INDEX IF NOT EXISTS idx_messages_session ON messages(session_id, id);
            "#,
        )?;

        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS provider_migrations (
                version INTEGER PRIMARY KEY
            );
            INSERT OR IGNORE INTO provider_migrations (version) VALUES (1);
            "#,
        )?;

        let catalog = Self {
            conn: Arc::new(Mutex::new(conn)),
        };

        catalog.run_migrations()?;
        catalog.init_default_providers()?;
        catalog.seed_default_models()?;
        catalog.init_default_sources()?;
        Ok(catalog)
    }

    fn db_path() -> Result<std::path::PathBuf> {
        if let Ok(p) = std::env::var("ORION_CATALOG_DB") {
            let path = std::path::PathBuf::from(p);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            return Ok(path);
        }
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("orion");
        std::fs::create_dir_all(&config_dir)?;
        Ok(config_dir.join("catalog.db"))
    }

    fn column_exists(conn: &rusqlite::Connection, table: &str, column: &str) -> Result<bool> {
        let mut stmt = conn.prepare(&format!("PRAGMA table_info({})", table))?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let name: String = row.get(1)?;
            if name == column {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn add_column_if_missing(
        conn: &rusqlite::Connection,
        table: &str,
        column: &str,
        definition: &str,
    ) -> Result<()> {
        if !Self::column_exists(conn, table, column)? {
            conn.execute(
                &format!("ALTER TABLE {} ADD COLUMN {} {}", table, column, definition),
                [],
            )?;
        }
        Ok(())
    }

    fn run_migrations(&self) -> Result<()> {
        let conn = self.conn.lock();
        let version: i32 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM provider_migrations",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        if version < 1 {
            Self::add_column_if_missing(&conn, "providers", "supports_streaming", "INTEGER NOT NULL DEFAULT 1")?;
            Self::add_column_if_missing(&conn, "providers", "supports_tools", "INTEGER NOT NULL DEFAULT 1")?;
            Self::add_column_if_missing(&conn, "providers", "supports_vision", "INTEGER NOT NULL DEFAULT 0")?;
            conn.execute("INSERT INTO provider_migrations (version) VALUES (1)", [])?;
        }

        if version < 2 {
            Self::add_column_if_missing(&conn, "models", "source", "TEXT")?;
            Self::add_column_if_missing(&conn, "models", "is_free", "INTEGER DEFAULT 0")?;
            Self::add_column_if_missing(&conn, "models", "is_local", "INTEGER DEFAULT 0")?;
            Self::add_column_if_missing(&conn, "models", "is_available", "INTEGER DEFAULT 1")?;

            conn.execute(
                r#"
                CREATE TABLE IF NOT EXISTS model_sources (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    url TEXT NOT NULL,
                    enabled INTEGER NOT NULL DEFAULT 1,
                    last_sync_at TEXT,
                    last_error TEXT
                );
                "#,
                [],
            )?;
            conn.execute("INSERT INTO provider_migrations (version) VALUES (2)", [])?;
        }

        if version < 3 {
            conn.execute(
                r#"
                CREATE TABLE IF NOT EXISTS sessions (
                    id TEXT PRIMARY KEY,
                    title TEXT NOT NULL DEFAULT 'New session',
                    created_at TEXT NOT NULL DEFAULT (datetime('now')),
                    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                    message_count INTEGER NOT NULL DEFAULT 0,
                    active_model TEXT
                );
                CREATE INDEX IF NOT EXISTS idx_sessions_updated ON sessions(updated_at DESC);
                "#,
                [],
            )?;
            conn.execute("INSERT INTO provider_migrations (version) VALUES (3)", [])?;
        }

        Ok(())
    }

    fn init_default_providers(&self) -> Result<()> {
        let conn = self.conn.lock();

        let defaults = vec![
            (
                "openrouter",
                "OpenRouter",
                "openai_compatible",
                "https://openrouter.ai/api/v1",
                Some("OPENROUTER_API_KEY"),
                true,
                true,
                true,
            ),
            (
                "openai",
                "OpenAI",
                "openai_compatible",
                "https://api.openai.com/v1",
                Some("OPENAI_API_KEY"),
                true,
                true,
                true,
            ),
            (
                "anthropic",
                "Anthropic",
                "anthropic",
                "",
                Some("ANTHROPIC_API_KEY"),
                true,
                true,
                true,
            ),
            (
                "ollama",
                "Ollama",
                "ollama",
                "http://localhost:11434",
                None,
                true,
                true,
                false,
            ),
            (
                "deepseek",
                "DeepSeek",
                "openai_compatible",
                "https://api.deepseek.com",
                Some("DEEPSEEK_API_KEY"),
                true,
                true,
                false,
            ),
            (
                "groq",
                "Groq",
                "openai_compatible",
                "https://api.groq.com/openai/v1",
                Some("GROQ_API_KEY"),
                true,
                true,
                false,
            ),
            (
                "mistral",
                "Mistral",
                "openai_compatible",
                "https://api.mistral.ai/v1",
                Some("MISTRAL_API_KEY"),
                true,
                true,
                false,
            ),
            (
                "together",
                "Together AI",
                "openai_compatible",
                "https://api.together.xyz/v1",
                Some("TOGETHER_API_KEY"),
                true,
                true,
                false,
            ),
            (
                "perplexity",
                "Perplexity",
                "openai_compatible",
                "https://api.perplexity.ai",
                Some("PERPLEXITY_API_KEY"),
                true,
                true,
                true,
            ),
            (
                "minimax",
                "MiniMax",
                "openai_compatible",
                "https://api.minimaxi.chat/v1",
                Some("MINIMAX_API_KEY"),
                true,
                true,
                false,
            ),
            (
                "google",
                "Google Gemini",
                "openai_compatible",
                "https://generativelanguage.googleapis.com/v1beta/openai",
                Some("GOOGLE_API_KEY"),
                true,
                true,
                true,
            ),
            (
                "qwen",
                "Qwen",
                "openai_compatible",
                "https://dashscope.aliyuncs.com/api-api/v1",
                Some("DASHSCOPE_API_KEY"),
                true,
                true,
                false,
            ),
            (
                "ernie",
                "Ernie",
                "openai_compatible",
                "https://qianfan.baidubce.com/v2/app/conversation",
                Some("ERNIE_API_KEY"),
                true,
                false,
                false,
            ),
            (
                "kimi",
                "Kimi",
                "openai_compatible",
                "https://api.moonshot.cn/v1",
                Some("KIMI_API_KEY"),
                true,
                true,
                false,
            ),
            (
                "hunyuan",
                "Hunyuan",
                "openai_compatible",
                "https://api.hunyuan.cloud.tencent.com/v1",
                Some("HUNYUAN_API_KEY"),
                true,
                true,
                false,
            ),
        ];

        for (
            id,
            name,
            kind,
            base_url,
            api_key_env,
            supports_streaming,
            supports_tools,
            supports_vision,
        ) in defaults
        {
            conn.execute(
                "INSERT OR IGNORE INTO providers (id, name, kind, base_url, api_key_env, supports_streaming, supports_tools, supports_vision) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![id, name, kind, base_url, api_key_env, supports_streaming as i32, supports_tools as i32, supports_vision as i32],
            )?;
        }

        Ok(())
    }

    /// Seed one sensible default model per built-in provider so a freshly
    /// connected provider is immediately usable (the model menu is never empty
    /// and `send_message` can auto-select). OpenRouter/Ollama are skipped — they
    /// populate via sync / local discovery. Idempotent (INSERT OR IGNORE).
    fn seed_default_models(&self) -> Result<()> {
        // (provider_id, model_id, display_name, context_window)
        let defaults: &[(&str, &str, &str, i64)] = &[
            ("minimax", "MiniMax-Text-01", "MiniMax Text 01", 1_000_000),
            ("openai", "gpt-4o-mini", "GPT-4o mini", 128_000),
            ("anthropic", "claude-3-5-sonnet-20241022", "Claude 3.5 Sonnet", 200_000),
            ("deepseek", "deepseek-chat", "DeepSeek Chat", 64_000),
            ("google", "gemini-2.0-flash", "Gemini 2.0 Flash", 1_000_000),
            ("groq", "llama-3.3-70b-versatile", "Llama 3.3 70B", 128_000),
            ("mistral", "mistral-large-latest", "Mistral Large", 128_000),
            ("together", "meta-llama/Llama-3.3-70B-Instruct-Turbo", "Llama 3.3 70B Turbo", 128_000),
            ("perplexity", "sonar", "Sonar", 128_000),
            ("qwen", "qwen-plus", "Qwen Plus", 131_000),
            ("kimi", "moonshot-v1-32k", "Kimi (Moonshot v1 32k)", 32_000),
            ("ernie", "ernie-4.0-8k", "ERNIE 4.0", 8_000),
            ("hunyuan", "hunyuan-pro", "Hunyuan Pro", 32_000),
        ];

        let conn = self.conn.lock();
        for (provider_id, model_id, display_name, ctx) in defaults {
            let full_id = format!("{}:{}", provider_id, model_id);
            conn.execute(
                "INSERT OR IGNORE INTO models \
                 (id, provider_id, model_id, display_name, context_window, supports_tools, enabled, is_available) \
                 VALUES (?1, ?2, ?3, ?4, ?5, 1, 1, 1)",
                params![full_id, provider_id, model_id, display_name, ctx],
            )?;
        }
        Ok(())
    }

    pub fn init_default_sources(&self) -> Result<()> {
        let conn = self.conn.lock();

        let defaults = vec![
            (
                "models_dev",
                "Models.dev",
                "https://models.dev/api/v1/models",
                true,
            ),
            (
                "openrouter",
                "OpenRouter",
                "https://openrouter.ai/api/v1/models",
                true,
            ),
        ];

        for (id, name, url, enabled) in defaults {
            conn.execute(
                "INSERT OR IGNORE INTO model_sources (id, name, url, enabled) VALUES (?1, ?2, ?3, ?4)",
                params![id, name, url, enabled as i32],
            )?;
        }

        Ok(())
    }

    pub fn list_sources(&self) -> Vec<ModelSource> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare("SELECT id, name, url, enabled, last_sync_at, last_error FROM model_sources")
            .unwrap();

        let rows = stmt
            .query_map([], |row| {
                Ok(ModelSource {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    url: row.get(2)?,
                    enabled: row.get::<_, i32>(3)? != 0,
                    last_sync_at: row.get(4)?,
                    last_error: row.get(5)?,
                })
            })
            .unwrap();
        rows.filter_map(|r| r.ok()).collect()
    }

    pub fn get_source(&self, id: &str) -> Option<ModelSource> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT id, name, url, enabled, last_sync_at, last_error FROM model_sources WHERE id = ?1",
            [id],
            |row| {
                Ok(ModelSource {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    url: row.get(2)?,
                    enabled: row.get::<_, i32>(3)? != 0,
                    last_sync_at: row.get(4)?,
                    last_error: row.get(5)?,
                })
            },
        )
        .ok()
    }

    pub fn update_source_sync(
        &self,
        source_id: &str,
        last_sync_at: &str,
        last_error: Option<&str>,
    ) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE model_sources SET last_sync_at = ?1, last_error = ?2 WHERE id = ?3",
            params![last_sync_at, last_error, source_id],
        )?;
        Ok(())
    }

    pub fn list_providers(&self) -> Vec<ProviderInfo> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, name, kind, base_url, api_key_env, enabled, supports_streaming, supports_tools, supports_vision, last_sync_at FROM providers ORDER BY name"
        ).unwrap();

        let rows = stmt
            .query_map([], |row| {
                Ok(ProviderInfo {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    kind: ProviderKind::from_str(&row.get::<_, String>(2)?),
                    base_url: row.get(3)?,
                    api_key_env: row.get(4)?,
                    enabled: row.get::<_, i32>(5)? != 0,
                    supports_streaming: row.get::<_, i32>(6)? != 0,
                    supports_tools: row.get::<_, i32>(7)? != 0,
                    supports_vision: row.get::<_, i32>(8)? != 0,
                    last_sync_at: row.get(9)?,
                })
            })
            .unwrap();

        rows.filter_map(|r| r.ok()).collect()
    }

    pub fn get_provider(&self, id: &str) -> Option<ProviderInfo> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT id, name, kind, base_url, api_key_env, enabled, supports_streaming, supports_tools, supports_vision, last_sync_at FROM providers WHERE id = ?1",
            [id],
            |row| Ok(ProviderInfo {
                id: row.get(0)?,
                name: row.get(1)?,
                kind: ProviderKind::from_str(&row.get::<_, String>(2)?),
                base_url: row.get(3)?,
                api_key_env: row.get(4)?,
                enabled: row.get::<_, i32>(5)? != 0,
                supports_streaming: row.get::<_, i32>(6)? != 0,
                supports_tools: row.get::<_, i32>(7)? != 0,
                supports_vision: row.get::<_, i32>(8)? != 0,
                last_sync_at: row.get(9)?,
            })
        ).ok()
    }

    pub fn list_models(&self, provider_id: Option<&str>) -> Vec<ModelInfo> {
        let conn = self.conn.lock();

        let models = if let Some(pid) = provider_id {
            let mut stmt = conn.prepare(
                "SELECT id, provider_id, model_id, display_name, source, context_window, max_output, input_price, output_price, supports_vision, supports_tools, supports_reasoning, supports_structured_output, enabled, is_free, is_local, is_available, rank_overall, rank_coding, rank_vision, updated_at FROM models WHERE provider_id = ?1 ORDER BY rank_overall NULLS LAST, display_name"
            ).unwrap();
            let rows = stmt
                .query_map([pid], |row| Self::row_to_model(row))
                .unwrap();
            rows.filter_map(|r| r.ok()).collect()
        } else {
            let mut stmt = conn.prepare(
                "SELECT id, provider_id, model_id, display_name, source, context_window, max_output, input_price, output_price, supports_vision, supports_tools, supports_reasoning, supports_structured_output, enabled, is_free, is_local, is_available, rank_overall, rank_coding, rank_vision, updated_at FROM models ORDER BY rank_overall NULLS LAST, display_name"
            ).unwrap();
            let rows = stmt.query_map([], |row| Self::row_to_model(row)).unwrap();
            rows.filter_map(|r| r.ok()).collect()
        };

        models
    }

    fn row_to_model(row: &rusqlite::Row) -> rusqlite::Result<ModelInfo> {
        Ok(ModelInfo {
            id: row.get(0)?,
            provider_id: row.get(1)?,
            model_id: row.get(2)?,
            display_name: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
            source: row.get(4)?,
            context_window: row.get(5)?,
            max_output: row.get(6)?,
            input_price: row.get(7)?,
            output_price: row.get(8)?,
            supports_vision: row.get::<_, i32>(9)? != 0,
            supports_tools: row.get::<_, i32>(10)? != 0,
            supports_reasoning: row.get::<_, i32>(11)? != 0,
            supports_structured_output: row.get::<_, i32>(12)? != 0,
            enabled: row.get::<_, i32>(13)? != 0,
            is_free: row.get::<_, i32>(14)? != 0,
            is_local: row.get::<_, i32>(15)? != 0,
            is_available: row.get::<_, i32>(16)? != 0,
            rank_overall: row.get(17)?,
            rank_coding: row.get(18)?,
            rank_vision: row.get(19)?,
            updated_at: row.get(20)?,
        })
    }

    pub fn get_model(&self, full_id: &str) -> Option<ModelInfo> {
        let parts: Vec<&str> = full_id.splitn(2, ':').collect();
        if parts.len() != 2 {
            return None;
        }

        let conn = self.conn.lock();
        conn.query_row(
            "SELECT id, provider_id, model_id, display_name, source, context_window, max_output, input_price, output_price, supports_vision, supports_tools, supports_reasoning, supports_structured_output, enabled, is_free, is_local, is_available, rank_overall, rank_coding, rank_vision, updated_at FROM models WHERE provider_id = ?1 AND model_id = ?2",
            params![parts[0], parts[1]],
            |row| Self::row_to_model(row),
        ).ok()
    }

    pub fn search(&self, query: &str) -> Vec<ModelInfo> {
        let conn = self.conn.lock();
        let pattern = format!("%{}%", query.to_lowercase());

        let mut stmt = conn.prepare(
            "SELECT id, provider_id, model_id, display_name, source, context_window, max_output, input_price, output_price, supports_vision, supports_tools, supports_reasoning, supports_structured_output, enabled, is_free, is_local, is_available, rank_overall, rank_coding, rank_vision, updated_at FROM models WHERE LOWER(display_name) LIKE ?1 OR LOWER(model_id) LIKE ?1 ORDER BY rank_overall NULLS LAST LIMIT 50"
        ).unwrap();

        let rows = stmt
            .query_map([&pattern], |row| Self::row_to_model(row))
            .unwrap();
        rows.filter_map(|r| r.ok()).collect()
    }

    pub fn add_model(&self, model: &ModelInfo) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT OR REPLACE INTO models (id, provider_id, model_id, display_name, source, context_window, max_output, input_price, output_price, supports_vision, supports_tools, supports_reasoning, supports_structured_output, enabled, is_free, is_local, is_available, rank_overall, rank_coding, rank_vision, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, datetime('now'))",
            params![
                model.id,
                model.provider_id,
                model.model_id,
                model.display_name,
                model.source,
                model.context_window,
                model.max_output,
                model.input_price,
                model.output_price,
                model.supports_vision as i32,
                model.supports_tools as i32,
                model.supports_reasoning as i32,
                model.supports_structured_output as i32,
                model.enabled as i32,
                model.is_free as i32,
                model.is_local as i32,
                model.is_available as i32,
                model.rank_overall,
                model.rank_coding,
                model.rank_vision,
            ],
        )?;
        Ok(())
    }

    pub fn upsert_model(
        &self,
        provider_id: &str,
        model_id: &str,
        display_name: &str,
        attrs: &[(&str, String)],
    ) -> Result<()> {
        let full_id = format!("{}:{}", provider_id, model_id);
        let conn = self.conn.lock();

        let mut model = ModelInfo {
            id: full_id.clone(),
            provider_id: provider_id.to_string(),
            model_id: model_id.to_string(),
            display_name: display_name.to_string(),
            source: None,
            context_window: None,
            max_output: None,
            input_price: None,
            output_price: None,
            supports_vision: false,
            supports_tools: false,
            supports_reasoning: false,
            supports_structured_output: false,
            enabled: true,
            is_free: false,
            is_local: provider_id == "ollama",
            is_available: true,
            rank_overall: None,
            rank_coding: None,
            rank_vision: None,
            updated_at: None,
        };

        for (key, value) in attrs {
            match *key {
                "context_window" => model.context_window = value.parse().ok(),
                "max_output" => model.max_output = value.parse().ok(),
                "input_price" => model.input_price = value.parse().ok(),
                "output_price" => model.output_price = value.parse().ok(),
                "supports_vision" => model.supports_vision = value.as_str() == "true",
                "supports_tools" => model.supports_tools = value.as_str() == "true",
                "supports_reasoning" => model.supports_reasoning = value.as_str() == "true",
                "source" => model.source = Some(value.clone()),
                "is_free" => model.is_free = value.as_str() == "true",
                "rank_overall" => model.rank_overall = value.parse().ok(),
                "rank_coding" => model.rank_coding = value.parse().ok(),
                "rank_vision" => model.rank_vision = value.parse().ok(),
                _ => {}
            }
        }

        drop(conn);
        self.add_model(&model)
    }

    /// Pick a usable model from a connected provider (has an API key or is
    /// ollama), preferring tool-capable models. Used to auto-select a model so
    /// a connected provider is chattable without manual selection.
    pub fn pick_connected_model(&self) -> Option<ModelInfo> {
        let connected: std::collections::HashSet<String> = self
            .list_providers()
            .into_iter()
            .filter(|p| {
                p.id == "ollama"
                    || self.get_api_key(&p.id).is_some()
                    || p.api_key_env
                        .as_ref()
                        .and_then(|k| std::env::var(k).ok())
                        .is_some()
            })
            .map(|p| p.id)
            .collect();
        let mut candidates: Vec<ModelInfo> = self
            .list_models(None)
            .into_iter()
            .filter(|m| connected.contains(&m.provider_id) && m.is_available)
            .collect();
        candidates.sort_by_key(|m| !m.supports_tools);
        candidates.into_iter().next()
    }

    pub fn get_default_model(&self) -> Option<ModelInfo> {
        let conn = self.conn.lock();
        if let Ok(full_id) = conn.query_row(
            "SELECT value FROM config WHERE key = 'default_model'",
            [],
            |row| row.get::<_, String>(0),
        ) {
            drop(conn);
            return self.get_model(&full_id);
        }

        conn.query_row(
            "SELECT id, provider_id, model_id, display_name, context_window, max_output, input_price, output_price, supports_vision, supports_tools, supports_reasoning, supports_structured_output, enabled, rank_overall, rank_coding, rank_vision, updated_at FROM models WHERE enabled = 1 ORDER BY rank_overall NULLS LAST LIMIT 1",
            [],
            |row| Self::row_to_model(row),
        ).ok()
    }

    pub fn set_default_model(&self, full_id: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT OR REPLACE INTO config (key, value) VALUES ('default_model', ?1)",
            [full_id],
        )?;
        Ok(())
    }

    pub fn update_provider_sync_time(&self, provider_id: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE providers SET last_sync_at = datetime('now') WHERE id = ?1",
            [provider_id],
        )?;
        Ok(())
    }

    pub fn get_best_coding(&self) -> Vec<ModelInfo> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, provider_id, model_id, display_name, context_window, max_output, input_price, output_price, supports_vision, supports_tools, supports_reasoning, supports_structured_output, enabled, rank_overall, rank_coding, rank_vision, updated_at FROM models WHERE enabled = 1 AND rank_coding IS NOT NULL ORDER BY rank_coding LIMIT 10"
        ).unwrap();
        let rows = stmt.query_map([], |row| Self::row_to_model(row)).unwrap();
        rows.filter_map(|r| r.ok()).collect()
    }

    pub fn get_best_vision(&self) -> Vec<ModelInfo> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, provider_id, model_id, display_name, context_window, max_output, input_price, output_price, supports_vision, supports_tools, supports_reasoning, supports_structured_output, enabled, rank_overall, rank_coding, rank_vision, updated_at FROM models WHERE enabled = 1 AND supports_vision = 1 AND rank_vision IS NOT NULL ORDER BY rank_vision LIMIT 10"
        ).unwrap();
        let rows = stmt.query_map([], |row| Self::row_to_model(row)).unwrap();
        rows.filter_map(|r| r.ok()).collect()
    }

    pub fn get_best_overall(&self) -> Vec<ModelInfo> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, provider_id, model_id, display_name, context_window, max_output, input_price, output_price, supports_vision, supports_tools, supports_reasoning, supports_structured_output, enabled, rank_overall, rank_coding, rank_vision, updated_at FROM models WHERE enabled = 1 AND rank_overall IS NOT NULL ORDER BY rank_overall LIMIT 10"
        ).unwrap();
        let rows = stmt.query_map([], |row| Self::row_to_model(row)).unwrap();
        rows.filter_map(|r| r.ok()).collect()
    }

    pub fn get_local_models(&self) -> Vec<ModelInfo> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, provider_id, model_id, display_name, context_window, max_output, input_price, output_price, supports_vision, supports_tools, supports_reasoning, supports_structured_output, enabled, rank_overall, rank_coding, rank_vision, updated_at FROM models WHERE provider_id = 'ollama' AND enabled = 1 ORDER BY display_name"
        ).unwrap();
        let rows = stmt.query_map([], |row| Self::row_to_model(row)).unwrap();
        rows.filter_map(|r| r.ok()).collect()
    }

    pub fn get_cheap_models(&self) -> Vec<ModelInfo> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, provider_id, model_id, display_name, context_window, max_output, input_price, output_price, supports_vision, supports_tools, supports_reasoning, supports_structured_output, enabled, rank_overall, rank_coding, rank_vision, updated_at FROM models WHERE enabled = 1 AND output_price IS NOT NULL ORDER BY output_price LIMIT 10"
        ).unwrap();
        let rows = stmt.query_map([], |row| Self::row_to_model(row)).unwrap();
        rows.filter_map(|r| r.ok()).collect()
    }

    /// Generic key/value config, shared across CLI + desktop (same DB).
    pub fn set_config(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT OR REPLACE INTO config (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn get_config(&self, key: &str) -> Option<String> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT value FROM config WHERE key = ?1",
            [key],
            |row| row.get(0),
        )
        .ok()
    }

    /// Read a boolean config flag (e.g. the `full_access` master switch).
    pub fn get_bool_config(&self, key: &str) -> bool {
        self.get_config(key).as_deref() == Some("true")
    }

    /// Store an API key. Prefers the OS keyring (encrypted, outside the DB);
    /// falls back to the config table when no keyring is available (CI/headless).
    pub fn save_api_key(&self, provider_id: &str, api_key: &str) -> Result<()> {
        if let Some(entry) = keyring_entry(provider_id) {
            if entry.set_password(api_key).is_ok() {
                // Drop any plaintext copy lingering in the DB.
                let conn = self.conn.lock();
                let _ = conn.execute(
                    "DELETE FROM config WHERE key = ?1",
                    [format!("api_key:{}", provider_id)],
                );
                return Ok(());
            }
        }
        let conn = self.conn.lock();
        conn.execute(
            "INSERT OR REPLACE INTO config (key, value) VALUES (?1, ?2)",
            params![format!("api_key:{}", provider_id), api_key],
        )?;
        Ok(())
    }

    pub fn get_api_key(&self, provider_id: &str) -> Option<String> {
        // Keyring first.
        if let Some(entry) = keyring_entry(provider_id) {
            if let Ok(pw) = entry.get_password() {
                return Some(pw);
            }
        }
        // DB fallback — and best-effort migrate any plaintext key to the keyring.
        let db_key: Option<String> = {
            let conn = self.conn.lock();
            conn.query_row(
                "SELECT value FROM config WHERE key = ?1",
                [format!("api_key:{}", provider_id)],
                |row| row.get(0),
            )
            .ok()
        };
        if let Some(ref key) = db_key {
            if let Some(entry) = keyring_entry(provider_id) {
                if entry.set_password(key).is_ok() {
                    let conn = self.conn.lock();
                    let _ = conn.execute(
                        "DELETE FROM config WHERE key = ?1",
                        [format!("api_key:{}", provider_id)],
                    );
                }
            }
        }
        db_key
    }

    pub fn delete_api_key(&self, provider_id: &str) -> Result<()> {
        if let Some(entry) = keyring_entry(provider_id) {
            let _ = entry.delete_credential();
        }
        let conn = self.conn.lock();
        conn.execute(
            "DELETE FROM config WHERE key = ?1",
            [format!("api_key:{}", provider_id)],
        )?;
        Ok(())
    }

    pub fn set_provider_enabled(&self, provider_id: &str, enabled: bool) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE providers SET enabled = ?1 WHERE id = ?2",
            params![enabled as i32, provider_id],
        )?;
        Ok(())
    }

    fn row_to_session(row: &rusqlite::Row) -> rusqlite::Result<Session> {
        Ok(Session {
            id: row.get(0)?,
            title: row.get(1)?,
            created_at: row.get(2)?,
            updated_at: row.get(3)?,
            message_count: row.get(4)?,
            active_model: row.get(5)?,
        })
    }

    pub fn list_sessions(&self) -> Vec<Session> {
        let conn = self.conn.lock();
        let mut stmt = match conn.prepare(
            "SELECT id, title, created_at, updated_at, message_count, active_model FROM sessions ORDER BY updated_at DESC"
        ) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        stmt.query_map([], Self::row_to_session)
            .map(|rows| rows.filter_map(|r| r.ok()).collect())
            .unwrap_or_default()
    }

    pub fn get_session(&self, id: &str) -> Option<Session> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT id, title, created_at, updated_at, message_count, active_model FROM sessions WHERE id = ?1",
            [id],
            Self::row_to_session,
        )
        .ok()
    }

    /// Best-effort: list messages for a session.
    ///
    /// The catalog schema does not store full message transcripts (those live
    /// in `MemoryStore`). This method returns `None` when the catalog can't
    /// supply them, so callers can fall back to the memory store.
    pub fn list_messages_for_session(
        &self,
        _session_id: &str,
    ) -> Option<Vec<(String, String, Option<String>, Option<String>)>> {
        None
    }

    pub fn create_session(&self, title: Option<&str>) -> Result<Session> {
        let id = uuid::Uuid::new_v4().to_string();
        let title = title.unwrap_or("New session");
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO sessions (id, title) VALUES (?1, ?2)",
            params![&id, title],
        )?;
        drop(conn);
        Ok(self
            .get_session(&id)
            .unwrap_or(Session {
                id: id.clone(),
                title: title.to_string(),
                created_at: String::new(),
                updated_at: String::new(),
                message_count: 0,
                active_model: None,
            }))
    }

    pub fn delete_session(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM messages WHERE session_id = ?1", [id])?;
        conn.execute("DELETE FROM sessions WHERE id = ?1", [id])?;
        Ok(())
    }

    /// Persist a chat message and bump the session's counter/timestamp.
    pub fn add_message(&self, session_id: &str, role: &str, content: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO messages (session_id, role, content) VALUES (?1, ?2, ?3)",
            params![session_id, role, content],
        )?;
        conn.execute(
            "UPDATE sessions SET message_count = message_count + 1, updated_at = datetime('now') WHERE id = ?1",
            [session_id],
        )?;
        Ok(())
    }

    /// Load a session's messages in chronological order.
    pub fn get_messages(&self, session_id: &str) -> Vec<StoredMessage> {
        let conn = self.conn.lock();
        let mut stmt = match conn.prepare(
            "SELECT role, content, created_at FROM messages WHERE session_id = ?1 ORDER BY id ASC",
        ) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        let rows = stmt.query_map([session_id], |row| {
            Ok(StoredMessage {
                role: row.get(0)?,
                content: row.get(1)?,
                created_at: row.get(2)?,
            })
        });
        match rows {
            Ok(r) => r.filter_map(|m| m.ok()).collect(),
            Err(_) => Vec::new(),
        }
    }

    pub fn rename_session(&self, id: &str, title: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE sessions SET title = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![title, id],
        )?;
        Ok(())
    }

    pub fn touch_session(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE sessions SET updated_at = datetime('now') WHERE id = ?1",
            [id],
        )?;
        Ok(())
    }

    pub fn get_active_session(&self) -> Option<Session> {
        let conn = self.conn.lock();
        let id: Option<String> = conn
            .query_row(
                "SELECT value FROM config WHERE key = 'active_session'",
                [],
                |row| row.get(0),
            )
            .ok();
        drop(conn);
        id.and_then(|sid| self.get_session(&sid))
    }

    pub fn set_active_session(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT OR REPLACE INTO config (key, value) VALUES ('active_session', ?1)",
            [id],
        )?;
        Ok(())
    }
}
