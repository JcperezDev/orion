use crate::providers::traits::Message;
use crate::server::sessions::Session;
use anyhow::Result;
use parking_lot::Mutex;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageRecord {
    pub role: String,
    pub content: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecord {
    pub id: String,
    pub title: String,
    pub provider: String,
    pub model: String,
    pub created_at: String,
    pub updated_at: String,
    pub messages: Vec<MessageRecord>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub default_provider: Option<String>,
    #[serde(default)]
    pub default_model: Option<String>,
    #[serde(default)]
    pub theme: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub auto_accept_permissions: Option<bool>,
    #[serde(default)]
    pub show_reasoning: Option<bool>,
    #[serde(default)]
    pub sound_effects: Option<bool>,
    #[serde(default)]
    pub notifications: Option<bool>,
    #[serde(default)]
    pub token_budget_per_session: Option<u32>,
    #[serde(default)]
    pub auto_compress_threshold: Option<f32>,
    #[serde(default)]
    pub mcp_enabled: Option<bool>,
    #[serde(default)]
    pub keybindings: Option<HashMap<String, String>>,
    #[serde(default)]
    pub permissions: Option<HashMap<String, String>>,
    #[serde(default)]
    pub expand_shell_output: Option<bool>,
    #[serde(default)]
    pub expand_edit_output: Option<bool>,
    #[serde(default)]
    pub session_progress_bar: Option<bool>,
    #[serde(default)]
    pub show_file_tree: Option<bool>,
    #[serde(default)]
    pub command_palette_button: Option<bool>,
    #[serde(default)]
    pub token_god_auto_compress: Option<bool>,
    #[serde(default)]
    pub terminal_shell: Option<String>,
    #[serde(default)]
    pub color_scheme: Option<String>,
    #[serde(default)]
    pub ui_font: Option<String>,
    #[serde(default)]
    pub code_font: Option<String>,
    #[serde(default)]
    pub custom_theme: Option<serde_json::Value>,
    #[serde(default)]
    pub agent_response_language: Option<String>,
    #[serde(default)]
    pub date_format: Option<String>,
    #[serde(default)]
    pub project_memory_enabled: Option<bool>,
    #[serde(default)]
    pub auto_summarize_sessions: Option<bool>,
    #[serde(default)]
    pub user_preferences_enabled: Option<bool>,
    #[serde(default)]
    pub permission_read: Option<String>,
    #[serde(default)]
    pub permission_write: Option<String>,
    #[serde(default)]
    pub permission_shell: Option<String>,
    #[serde(default)]
    pub permission_network: Option<String>,
    #[serde(default)]
    pub permission_delete: Option<String>,
    #[serde(default)]
    pub permission_git: Option<String>,
    #[serde(default)]
    pub permission_mcp: Option<String>,
    #[serde(default)]
    pub working_dir: Option<String>,
    #[serde(default)]
    pub session_count: Option<u32>,
    #[serde(default)]
    pub memory_size_kb: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMemory {
    pub working_dir: String,
    pub summary: String,
    pub facts: Vec<String>,
    pub updated_at: String,
}

pub struct MemoryStore {
    conn: Arc<Mutex<Connection>>,
    sessions: Arc<Mutex<HashMap<String, SessionRecord>>>,
    mcp_servers: Arc<Mutex<Vec<crate::server::mcp::McpServerView>>>,
}

impl MemoryStore {
    pub fn new() -> Result<Self> {
        let path = Self::db_path()?;
        let conn = Connection::open(&path)?;
        Self::init_schema(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            sessions: Arc::new(Mutex::new(HashMap::new())),
            mcp_servers: Arc::new(Mutex::new(Self::default_mcp_servers())),
        })
    }

    fn db_path() -> Result<PathBuf> {
        if let Ok(p) = std::env::var("ORION_MEMORY_DB") {
            let path = PathBuf::from(p);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            return Ok(path);
        }
        let dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("orion");
        std::fs::create_dir_all(&dir)?;
        Ok(dir.join("memory.db"))
    }

    fn init_schema(conn: &Connection) -> Result<()> {
        let _ = conn.pragma_update(None, "journal_mode", "WAL");
        let _ = conn.pragma_update(None, "synchronous", "NORMAL");
        let _ = conn.pragma_update(None, "busy_timeout", "5000");
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS project_memory (
                working_dir TEXT PRIMARY KEY,
                summary TEXT NOT NULL DEFAULT '',
                facts TEXT NOT NULL DEFAULT '[]',
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS session_messages (
                session_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at TEXT NOT NULL,
                seq INTEGER PRIMARY KEY AUTOINCREMENT
            );
            CREATE INDEX IF NOT EXISTS idx_session_messages
                ON session_messages(session_id, seq);
            "#,
        )?;
        Ok(())
    }

    fn default_mcp_servers() -> Vec<crate::server::mcp::McpServerView> {
        vec![
            crate::server::mcp::McpServerView {
                id: "token-god".into(),
                name: "Token God".into(),
                transport: "stdio".into(),
                status: "configured".into(),
                tools: vec![
                    "compress_context".into(),
                    "summarize_history".into(),
                    "analyze_budget".into(),
                ],
            },
            crate::server::mcp::McpServerView {
                id: "filesystem".into(),
                name: "Filesystem".into(),
                transport: "stdio".into(),
                status: "active".into(),
                tools: vec![
                    "read_file".into(),
                    "write_file".into(),
                    "list_directory".into(),
                ],
            },
        ]
    }

    pub async fn create_session(
        &self,
        id: &str,
        title: &str,
        provider: &str,
        model: &str,
    ) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        let record = SessionRecord {
            id: id.to_string(),
            title: title.to_string(),
            provider: provider.to_string(),
            model: model.to_string(),
            created_at: now.clone(),
            updated_at: now,
            messages: Vec::new(),
        };
        self.sessions.lock().insert(id.to_string(), record);
        Ok(())
    }

    pub async fn get_session(&self, id: &str) -> Option<serde_json::Value> {
        let mut session = self.sessions.lock().get(id).cloned()?;
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare(
                "SELECT role, content, created_at FROM session_messages
                 WHERE session_id = ?1 ORDER BY seq ASC",
            )
            .ok()?;
        let rows = stmt
            .query_map(params![id], |row| {
                Ok(MessageRecord {
                    role: row.get(0)?,
                    content: row.get(1)?,
                    created_at: row.get(2)?,
                })
            })
            .ok()?;
        session.messages = rows.filter_map(|r| r.ok()).collect();
        Some(serde_json::to_value(session).ok()?)
    }

    pub async fn delete_session(&self, id: &str) -> Result<()> {
        self.sessions.lock().remove(id);
        let conn = self.conn.lock();
        conn.execute(
            "DELETE FROM session_messages WHERE session_id = ?1",
            params![id],
        )?;
        Ok(())
    }

    pub async fn list_sessions(&self) -> Vec<Session> {
        let map = self.sessions.lock();
        let conn = self.conn.lock();

        map.values()
            .map(|s| {
                let count: i64 = conn
                    .query_row(
                        "SELECT COUNT(*) FROM session_messages WHERE session_id = ?1",
                        params![s.id],
                        |row| row.get(0),
                    )
                    .unwrap_or(0);
                Session {
                    id: s.id.clone(),
                    title: s.title.clone(),
                    provider: s.provider.clone(),
                    model: s.model.clone(),
                    created_at: s.created_at.clone(),
                    message_count: count as usize,
                }
            })
            .collect()
    }

    pub async fn record_message(&self, session_id: &str, role: &str, content: &str) {
        let now = chrono::Utc::now().to_rfc3339();
        let conn = self.conn.lock();
        let _ = conn.execute(
            "INSERT INTO session_messages (session_id, role, content, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![session_id, role, content, now],
        );
    }

    /// Save a project-level summary (the LLM-generated digest of past work).
    pub async fn save_project_summary(&self, working_dir: &str, summary: &str) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        let conn = self.conn.lock();
        conn.execute(
            "INSERT OR REPLACE INTO project_memory (working_dir, summary, facts, updated_at)
             VALUES (?1, ?2, '[]', ?3)",
            params![working_dir, summary, now],
        )?;
        Ok(())
    }

    /// Build a summarization prompt from session messages.
    pub async fn build_summary_prompt(&self, session_id: &str) -> Option<String> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare(
                "SELECT role, content FROM session_messages
                 WHERE session_id = ?1 ORDER BY seq ASC LIMIT 80",
            )
            .ok()?;
        let rows = stmt
            .query_map(params![session_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .ok()?;
        let msgs: Vec<(String, String)> = rows.filter_map(|r| r.ok()).collect();
        drop(stmt);
        drop(conn);
        if msgs.is_empty() {
            return None;
        }
        let mut prompt = String::from(
            "Summarize this conversation in 3-5 short bullet points. \
             Focus on: decisions made, files touched, errors encountered, \
             and the next concrete step. Output only the bullets.\n\n",
        );
        for (role, content) in msgs.iter().take(40) {
            let trimmed = if content.len() > 500 {
                format!("{}…", &content[..500])
            } else {
                content.clone()
            };
            prompt.push_str(&format!("[{}] {}\n\n", role, trimmed));
        }
        Some(prompt)
    }

    /// Inject project + global context into outgoing messages.
    /// For now: prepend a single system-style message at index 0 if missing.
    pub async fn inject_context(&self, session_id: &str, messages: &mut Vec<Message>) {
        let summary = self.get_project_summary().await;
        if summary.is_empty() {
            return;
        }
        if let Some(first) = messages.first_mut() {
            if first.role == "system" {
                first.content.push_str(&format!("\n\nProject context:\n{summary}"));
                return;
            }
        }
        messages.insert(
            0,
            Message {
                role: "system".into(),
                content: format!("Project context:\n{summary}"),
            },
        );
        let _ = session_id;
    }

    pub async fn get_context(&self) -> String {
        self.get_project_summary().await
    }

    async fn get_project_summary(&self) -> String {
        let conn = self.conn.lock();
        let working_dir = std::env::current_dir()
            .ok()
            .and_then(|p| p.to_str().map(String::from))
            .unwrap_or_default();

        let summary: String = conn
            .query_row(
                "SELECT summary FROM project_memory WHERE working_dir = ?1",
                params![working_dir],
                |row| row.get(0),
            )
            .unwrap_or_default();

        let _ = working_dir;
        summary
    }

    pub async fn get_settings(&self) -> Settings {
        let conn = self.conn.lock();
        let mut stmt = match conn.prepare("SELECT value FROM settings WHERE key = 'config'") {
            Ok(s) => s,
            Err(_) => return Settings::default(),
        };
        let value: Option<String> = stmt
            .query_row([], |row| row.get(0))
            .ok();
        match value.and_then(|v| serde_json::from_str(&v).ok()) {
            Some(s) => s,
            None => Settings::default(),
        }
    }

    pub async fn save_settings(&self, s: &Settings) -> Result<()> {
        let json = serde_json::to_string(s)?;
        let conn = self.conn.lock();
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('config', ?1)",
            params![json],
        )?;
        Ok(())
    }

    pub async fn list_mcp_servers(&self) -> Vec<crate::server::mcp::McpServerView> {
        self.mcp_servers.lock().clone()
    }

    /// Read-only context: working_dir, summary, session_count, last_updated.
    pub async fn context_snapshot(&self) -> ContextSnapshot {
        let working_dir = std::env::current_dir()
            .ok()
            .and_then(|p| p.to_str().map(String::from))
            .unwrap_or_default();

        let conn = self.conn.lock();
        let (summary, last_updated): (String, Option<String>) = conn
            .query_row(
                "SELECT COALESCE(summary, ''), updated_at FROM project_memory
                 WHERE working_dir = ?1",
                rusqlite::params![working_dir],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
            )
            .unwrap_or((String::new(), None));

        let session_count: i64 = conn
            .query_row(
                "SELECT COUNT(DISTINCT session_id) FROM session_messages",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let db_dir = dirs::data_local_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("orion");
        let mut total: u64 = 0;
        if let Ok(entries) = std::fs::read_dir(&db_dir) {
            for entry in entries.flatten() {
                if let Ok(meta) = entry.metadata() {
                    total += meta.len();
                }
            }
        }

        ContextSnapshot {
            working_dir,
            summary,
            last_updated,
            session_count: session_count as u32,
            memory_size_kb: (total as f64) / 1024.0,
        }
    }

    pub async fn upsert_mcp_server(&self, srv: crate::server::mcp::McpServerView) {
        let mut list = self.mcp_servers.lock();
        if let Some(existing) = list.iter_mut().find(|s| s.id == srv.id) {
            *existing = srv;
        } else {
            list.push(srv);
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ContextSnapshot {
    pub working_dir: String,
    pub summary: String,
    pub last_updated: Option<String>,
    pub session_count: u32,
    pub memory_size_kb: f64,
}
