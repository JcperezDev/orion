use crate::config::Config;
use anyhow::Result;
use rusqlite::{params, Connection};
use std::path::PathBuf;

#[allow(dead_code)]
pub struct Memory {
    conn: Connection,
}

#[allow(dead_code)]
impl Memory {
    pub async fn new(config: &Config) -> Result<Self> {
        let db_path = Self::get_db_path(config)?;
        let conn = Connection::open(&db_path)?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS memories (
                id TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                created_at TEXT NOT NULL,
                tags TEXT
            )",
            [],
        )?;

        Ok(Self { conn })
    }

    fn get_db_path(_config: &Config) -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("orion");
        std::fs::create_dir_all(&config_dir)?;
        Ok(config_dir.join("memory.db"))
    }

    pub async fn add(&self, content: &str) {
        let id = uuid::Uuid::new_v4().to_string();
        let created_at = chrono::Utc::now().to_rfc3339();
        let tags = "";

        self.conn
            .execute(
                "INSERT INTO memories (id, content, created_at, tags) VALUES (?1, ?2, ?3, ?4)",
                params![id, content, created_at, tags],
            )
            .ok();
    }

    pub async fn list(&self) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT content FROM memories ORDER BY created_at DESC LIMIT 50")?;
        let rows = stmt.query_map([], |row| row.get(0))?;

        let mut memories = Vec::new();
        for row in rows {
            if let Ok(content) = row {
                memories.push(content);
            }
        }
        Ok(memories)
    }

    pub async fn search(&self, query: &str) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT content FROM memories WHERE content LIKE ?1 ORDER BY created_at DESC LIMIT 20",
        )?;
        let pattern = format!("%{}%", query);
        let rows = stmt.query_map([&pattern], |row| row.get(0))?;

        let mut results = Vec::new();
        for row in rows {
            if let Ok(content) = row {
                results.push(content);
            }
        }
        Ok(results)
    }

    pub async fn clear(&self) -> Result<()> {
        self.conn.execute("DELETE FROM memories", [])?;
        Ok(())
    }

    pub async fn delete(&self, id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM memories WHERE id = ?1", [id])?;
        Ok(())
    }
}
