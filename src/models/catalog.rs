use anyhow::Result;
use parking_lot::Mutex;
use rusqlite::{params, Connection};
use std::sync::Arc;

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

#[derive(Debug, Clone)]
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

pub struct ModelCatalog {
    conn: Arc<Mutex<Connection>>,
}

impl ModelCatalog {
    pub fn new() -> Result<Self> {
        let db_path = Self::db_path()?;
        let conn = Connection::open(&db_path)?;

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

            CREATE INDEX IF NOT EXISTS idx_models_provider ON models(provider_id);
            CREATE INDEX IF NOT EXISTS idx_models_rank ON models(rank_overall);
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
        catalog.init_default_sources()?;
        Ok(catalog)
    }

    fn db_path() -> Result<std::path::PathBuf> {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("orion");
        std::fs::create_dir_all(&config_dir)?;
        Ok(config_dir.join("catalog.db"))
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
            conn.execute_batch(
                r#"
                ALTER TABLE providers ADD COLUMN supports_streaming INTEGER NOT NULL DEFAULT 1;
                ALTER TABLE providers ADD COLUMN supports_tools INTEGER NOT NULL DEFAULT 1;
                ALTER TABLE providers ADD COLUMN supports_vision INTEGER NOT NULL DEFAULT 0;
                "#,
            )?;
            conn.execute("INSERT INTO provider_migrations (version) VALUES (1)", [])?;
        }

        if version < 2 {
            conn.execute_batch(
                r#"
                ALTER TABLE models ADD COLUMN source TEXT;
                ALTER TABLE models ADD COLUMN is_free INTEGER DEFAULT 0;
                ALTER TABLE models ADD COLUMN is_local INTEGER DEFAULT 0;
                ALTER TABLE models ADD COLUMN is_available INTEGER DEFAULT 1;

                CREATE TABLE IF NOT EXISTS model_sources (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    url TEXT NOT NULL,
                    enabled INTEGER NOT NULL DEFAULT 1,
                    last_sync_at TEXT,
                    last_error TEXT
                );
                "#,
            )?;
            conn.execute("INSERT INTO provider_migrations (version) VALUES (2)", [])?;
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

    pub fn init_default_sources(&self) -> Result<()> {
        let conn = self.conn.lock();

        let defaults = vec![
            ("models_dev", "Models.dev", "https://models.dev/api/v1/models", true),
            ("openrouter", "OpenRouter", "https://openrouter.ai/api/v1/models", true),
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

    pub fn update_source_sync(&self, source_id: &str, last_sync_at: &str, last_error: Option<&str>) -> Result<()> {
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
}
