//! Persistence for *learned* permissions — the rules created when a user picks
//! "always allow" in an approval dialog. Rules are scoped to the project path
//! so an approval in one repo never leaks into another, and they survive across
//! sessions. Backed by SQLite, mirroring [`crate::models::catalog`].

use crate::permissions::{Action, PermissionEngine};
use anyhow::Result;
use parking_lot::Mutex;
use rusqlite::{params, Connection};
use std::path::Path;

pub struct LearnedStore {
    conn: Mutex<Connection>,
}

fn action_to_str(a: Action) -> &'static str {
    match a {
        Action::Allow => "allow",
        Action::Ask => "ask",
        Action::Deny => "deny",
    }
}

fn action_from_str(s: &str) -> Action {
    match s {
        "allow" => Action::Allow,
        "deny" => Action::Deny,
        _ => Action::Ask,
    }
}

fn canon(project: &Path) -> String {
    project
        .canonicalize()
        .unwrap_or_else(|_| project.to_path_buf())
        .to_string_lossy()
        .into_owned()
}

impl LearnedStore {
    fn db_path() -> Result<std::path::PathBuf> {
        if let Ok(p) = std::env::var("ORION_PERMISSIONS_DB") {
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
        Ok(config_dir.join("permissions.db"))
    }

    pub fn open() -> Result<Self> {
        let conn = Connection::open(Self::db_path()?)?;
        Self::from_conn(conn)
    }

    fn from_conn(conn: Connection) -> Result<Self> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS learned_permissions (
                project_path TEXT NOT NULL,
                tool         TEXT NOT NULL,
                pattern      TEXT NOT NULL,
                action       TEXT NOT NULL,
                created_at   TEXT NOT NULL,
                PRIMARY KEY (project_path, tool, pattern)
            );",
        )?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    /// Upsert a learned rule for a project.
    pub fn add(&self, project: &Path, tool: &str, pattern: &str, action: Action) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.lock().execute(
            "INSERT INTO learned_permissions (project_path, tool, pattern, action, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(project_path, tool, pattern)
             DO UPDATE SET action = excluded.action, created_at = excluded.created_at",
            params![canon(project), tool, pattern, action_to_str(action), now],
        )?;
        Ok(())
    }

    /// List `(tool, pattern, action)` rules saved for a project.
    pub fn list(&self, project: &Path) -> Result<Vec<(String, String, Action)>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT tool, pattern, action FROM learned_permissions WHERE project_path = ?1",
        )?;
        let rows = stmt
            .query_map(params![canon(project)], |row| {
                let tool: String = row.get(0)?;
                let pattern: String = row.get(1)?;
                let action: String = row.get(2)?;
                Ok((tool, pattern, action_from_str(&action)))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// Load a project's learned rules into a permission engine.
    pub fn hydrate(&self, engine: &PermissionEngine, project: &Path) -> Result<()> {
        for (tool, pattern, action) in self.list(project)? {
            let _ = engine.add_rule(&tool, &pattern, action);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mem_store() -> LearnedStore {
        LearnedStore::from_conn(Connection::open_in_memory().unwrap()).unwrap()
    }

    #[test]
    fn add_list_roundtrip() {
        let store = mem_store();
        let project = std::path::PathBuf::from("/tmp/proj-a");
        store.add(&project, "bash", "npm test*", Action::Allow).unwrap();
        let rules = store.list(&project).unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].0, "bash");
        assert_eq!(rules[0].1, "npm test*");
        assert_eq!(rules[0].2, Action::Allow);
    }

    #[test]
    fn upsert_replaces_action() {
        let store = mem_store();
        let project = std::path::PathBuf::from("/tmp/proj-a");
        store.add(&project, "bash", "rm *", Action::Allow).unwrap();
        store.add(&project, "bash", "rm *", Action::Deny).unwrap();
        let rules = store.list(&project).unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].2, Action::Deny);
    }

    #[test]
    fn rules_are_scoped_per_project() {
        let store = mem_store();
        let a = std::path::PathBuf::from("/tmp/proj-a");
        let b = std::path::PathBuf::from("/tmp/proj-b");
        store.add(&a, "bash", "npm test*", Action::Allow).unwrap();
        assert_eq!(store.list(&a).unwrap().len(), 1);
        assert_eq!(store.list(&b).unwrap().len(), 0);
    }

    #[test]
    fn hydrate_loads_into_engine() {
        use crate::permissions::{PermissionConfig, PermissionEngine};
        let store = mem_store();
        let project = std::path::PathBuf::from("/tmp/proj-h");
        store.add(&project, "bash", "npm test*", Action::Allow).unwrap();

        let engine = PermissionEngine::new(PermissionConfig::safe_defaults());
        store.hydrate(&engine, &project).unwrap();
        assert_eq!(engine.check_explicit("bash", "npm test --watch"), Some(Action::Allow));
    }
}
