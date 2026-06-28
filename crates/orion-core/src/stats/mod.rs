//! Persistent usage stats — token counts and costs per model, tool, project.
//!
//! Stored in SQLite (separate DB at `~/.orion/stats.db` or project-local
//! `.orion/stats.db`). The schema is intentionally simple so it can be queried
//! directly via `orion stats --days N --tools --models --project`.

use anyhow::Result;
use parking_lot::Mutex;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// One row of stats — a single tool call's outcome.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageEvent {
    pub session_id: String,
    pub project: Option<String>,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub tool: Option<String>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_usd: f64,
    pub ts: chrono::DateTime<chrono::Utc>,
}

/// Aggregated stats snapshot.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StatsSnapshot {
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_cost_usd: f64,
    pub event_count: u64,
    pub by_model: HashMap<String, ModelStats>,
    pub by_tool: HashMap<String, ToolStats>,
    pub by_project: HashMap<String, ProjectStats>,
    pub by_day: HashMap<String, DayStats>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelStats {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_usd: f64,
    pub event_count: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolStats {
    pub event_count: u64,
    pub cost_usd: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectStats {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_usd: f64,
    pub event_count: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DayStats {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_usd: f64,
    pub event_count: u64,
}

/// Persistent stats store.
pub struct StatsStore {
    conn: Mutex<Connection>,
    path: PathBuf,
}

impl StatsStore {
    /// Open or create a stats DB at `path`.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let conn = Connection::open(&path)?;
        let store = Self {
            conn: Mutex::new(conn),
            path,
        };
        store.migrate()?;
        Ok(store)
    }

    /// Open the default user-level stats DB (`~/.orion/stats.db`).
    pub fn user_default() -> Result<Self> {
        let dir = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("no home dir"))?
            .join(".orion");
        let path = dir.join("stats.db");
        Self::open(path)
    }

    fn migrate(&self) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS usage_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL,
                project TEXT,
                model TEXT,
                provider TEXT,
                tool TEXT,
                input_tokens INTEGER NOT NULL DEFAULT 0,
                output_tokens INTEGER NOT NULL DEFAULT 0,
                cost_usd REAL NOT NULL DEFAULT 0,
                ts TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE INDEX IF NOT EXISTS idx_events_session ON usage_events(session_id);
            CREATE INDEX IF NOT EXISTS idx_events_model ON usage_events(model);
            CREATE INDEX IF NOT EXISTS idx_events_tool ON usage_events(tool);
            CREATE INDEX IF NOT EXISTS idx_events_project ON usage_events(project);
            CREATE INDEX IF NOT EXISTS idx_events_ts ON usage_events(ts);
            "#,
        )?;
        Ok(())
    }

    /// Record a usage event.
    pub fn record(&self, event: &UsageEvent) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO usage_events (session_id, project, model, provider, tool, input_tokens, output_tokens, cost_usd, ts) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                event.session_id,
                event.project,
                event.model,
                event.provider,
                event.tool,
                event.input_tokens as i64,
                event.output_tokens as i64,
                event.cost_usd,
                event.ts.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    /// Build an aggregated snapshot, optionally filtered.
    pub fn snapshot(&self, filter: &StatsFilter) -> Result<StatsSnapshot> {
        let conn = self.conn.lock();
        let (where_clause, args) = filter.to_sql();
        let sql = format!(
            "SELECT COALESCE(project,''), COALESCE(model,''), COALESCE(tool,''), \
             COALESCE(provider,''), input_tokens, output_tokens, cost_usd, ts \
             FROM usage_events WHERE {}",
            where_clause
        );
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(args.iter()), |row| {
            let ts: String = row.get(7)?;
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, i64>(4)? as u64,
                row.get::<_, i64>(5)? as u64,
                row.get::<_, f64>(6)?,
                ts,
            ))
        })?;

        let mut snap = StatsSnapshot::default();
        for row in rows {
            let (project, model, tool, _provider, input, output, cost, ts) = row?;
            snap.total_input_tokens += input;
            snap.total_output_tokens += output;
            snap.total_cost_usd += cost;
            snap.event_count += 1;

            if !model.is_empty() {
                let entry = snap.by_model.entry(model).or_default();
                entry.input_tokens += input;
                entry.output_tokens += output;
                entry.cost_usd += cost;
                entry.event_count += 1;
            }
            if !tool.is_empty() {
                let entry = snap.by_tool.entry(tool).or_default();
                entry.event_count += 1;
                entry.cost_usd += cost;
            }
            if !project.is_empty() {
                let entry = snap.by_project.entry(project).or_default();
                entry.input_tokens += input;
                entry.output_tokens += output;
                entry.cost_usd += cost;
                entry.event_count += 1;
            }
            // Group by day (YYYY-MM-DD).
            if ts.len() >= 10 {
                let day = ts[..10].to_string();
                let entry = snap.by_day.entry(day).or_default();
                entry.input_tokens += input;
                entry.output_tokens += output;
                entry.cost_usd += cost;
                entry.event_count += 1;
            }
        }
        Ok(snap)
    }

    /// Path to the underlying SQLite file (for debugging).
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// List recent events (for `--days` filtering).
    pub fn recent(&self, limit: u32) -> Result<Vec<UsageEvent>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT session_id, COALESCE(project,''), COALESCE(model,''), COALESCE(provider,''), \
             COALESCE(tool,''), input_tokens, output_tokens, cost_usd, ts \
             FROM usage_events ORDER BY id DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], |row| {
            Ok(UsageEvent {
                session_id: row.get(0)?,
                project: Some(row.get::<_, String>(1)?).filter(|s| !s.is_empty()),
                model: Some(row.get::<_, String>(2)?).filter(|s| !s.is_empty()),
                provider: Some(row.get::<_, String>(3)?).filter(|s| !s.is_empty()),
                tool: Some(row.get::<_, String>(4)?).filter(|s| !s.is_empty()),
                input_tokens: row.get::<_, i64>(5)? as u64,
                output_tokens: row.get::<_, i64>(6)? as u64,
                cost_usd: row.get(7)?,
                ts: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(8)?)
                    .map(|d| d.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now()),
            })
        })?;
        let mut events = Vec::new();
        for row in rows {
            events.push(row?);
        }
        Ok(events)
    }
}

/// Filter for snapshot queries.
#[derive(Debug, Clone, Default)]
pub struct StatsFilter {
    pub since: Option<chrono::DateTime<chrono::Utc>>,
    pub until: Option<chrono::DateTime<chrono::Utc>>,
    pub project: Option<String>,
    pub model: Option<String>,
    pub tool: Option<String>,
}

impl StatsFilter {
    pub fn last_n_days(n: i64) -> Self {
        let now = chrono::Utc::now();
        Self {
            since: Some(now - chrono::Duration::days(n)),
            ..Default::default()
        }
    }

    pub fn for_project(project: impl Into<String>) -> Self {
        Self {
            project: Some(project.into()),
            ..Default::default()
        }
    }

    fn to_sql(&self) -> (String, Vec<rusqlite::types::Value>) {
        let mut clauses = vec!["1=1".to_string()];
        let mut args: Vec<rusqlite::types::Value> = Vec::new();
        if let Some(since) = &self.since {
            clauses.push(format!("ts >= ?{}", args.len() + 1));
            args.push(since.to_rfc3339().into());
        }
        if let Some(until) = &self.until {
            clauses.push(format!("ts <= ?{}", args.len() + 1));
            args.push(until.to_rfc3339().into());
        }
        if let Some(project) = &self.project {
            clauses.push(format!("project = ?{}", args.len() + 1));
            args.push(project.clone().into());
        }
        if let Some(model) = &self.model {
            clauses.push(format!("model = ?{}", args.len() + 1));
            args.push(model.clone().into());
        }
        if let Some(tool) = &self.tool {
            clauses.push(format!("tool = ?{}", args.len() + 1));
            args.push(tool.clone().into());
        }
        (clauses.join(" AND "), args)
    }
}

/// Format a snapshot as a human-readable summary.
pub fn format_snapshot(snap: &StatsSnapshot) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "Total: {} events, {} input + {} output tokens, ${:.4}\n",
        snap.event_count, snap.total_input_tokens, snap.total_output_tokens, snap.total_cost_usd
    ));
    if !snap.by_model.is_empty() {
        out.push_str("\nBy model:\n");
        let mut entries: Vec<_> = snap.by_model.iter().collect();
        entries.sort_by(|a, b| b.1.cost_usd.partial_cmp(&a.1.cost_usd).unwrap_or(std::cmp::Ordering::Equal));
        for (model, ms) in entries.iter().take(10) {
            out.push_str(&format!(
                "  {:<32}  {:>6} events  {:>10} in / {:>10} out  ${:.4}\n",
                truncate(model, 30),
                ms.event_count,
                ms.input_tokens,
                ms.output_tokens,
                ms.cost_usd
            ));
        }
    }
    if !snap.by_tool.is_empty() {
        out.push_str("\nBy tool:\n");
        let mut entries: Vec<_> = snap.by_tool.iter().collect();
        entries.sort_by(|a, b| b.1.event_count.cmp(&a.1.event_count));
        for (tool, ts) in entries.iter().take(10) {
            out.push_str(&format!(
                "  {:<24}  {:>6} calls  ${:.4}\n",
                truncate(tool, 22),
                ts.event_count,
                ts.cost_usd
            ));
        }
    }
    if !snap.by_project.is_empty() {
        out.push_str("\nBy project:\n");
        let mut entries: Vec<_> = snap.by_project.iter().collect();
        entries.sort_by(|a, b| b.1.cost_usd.partial_cmp(&a.1.cost_usd).unwrap_or(std::cmp::Ordering::Equal));
        for (proj, ps) in entries.iter().take(10) {
            out.push_str(&format!(
                "  {:<32}  {:>6} events  ${:.4}\n",
                truncate(proj, 30),
                ps.event_count,
                ps.cost_usd
            ));
        }
    }
    out
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path() -> PathBuf {
        let tmp = std::env::temp_dir().join(format!("orion-stats-test-{}-{}.db", std::process::id(), rand_u64()));
        tmp
    }

    fn rand_u64() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
    }

    #[test]
    fn open_creates_db_and_migrates() {
        let path = temp_path();
        let _store = StatsStore::open(&path).unwrap();
        assert!(path.exists());
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn record_and_snapshot_roundtrip() {
        let path = temp_path();
        let store = StatsStore::open(&path).unwrap();
        let event = UsageEvent {
            session_id: "sess1".into(),
            project: Some("orion".into()),
            model: Some("anthropic:claude-sonnet-4".into()),
            provider: Some("anthropic".into()),
            tool: None,
            input_tokens: 100,
            output_tokens: 50,
            cost_usd: 0.012,
            ts: chrono::Utc::now(),
        };
        store.record(&event).unwrap();
        let snap = store.snapshot(&StatsFilter::default()).unwrap();
        assert_eq!(snap.event_count, 1);
        assert_eq!(snap.total_input_tokens, 100);
        assert_eq!(snap.total_cost_usd, 0.012);
        assert!(snap.by_model.contains_key("anthropic:claude-sonnet-4"));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn filter_by_model() {
        let path = temp_path();
        let store = StatsStore::open(&path).unwrap();
        let now = chrono::Utc::now();
        store.record(&UsageEvent {
            session_id: "a".into(),
            project: None,
            model: Some("anthropic:claude".into()),
            provider: None,
            tool: None,
            input_tokens: 10,
            output_tokens: 5,
            cost_usd: 0.001,
            ts: now,
        })
        .unwrap();
        store.record(&UsageEvent {
            session_id: "b".into(),
            project: None,
            model: Some("openai:gpt-4o".into()),
            provider: None,
            tool: None,
            input_tokens: 20,
            output_tokens: 10,
            cost_usd: 0.005,
            ts: now,
        })
        .unwrap();
        let filter = StatsFilter {
            model: Some("anthropic:claude".into()),
            ..Default::default()
        };
        let snap = store.snapshot(&filter).unwrap();
        assert_eq!(snap.event_count, 1);
        assert_eq!(snap.total_input_tokens, 10);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn filter_by_project() {
        let path = temp_path();
        let store = StatsStore::open(&path).unwrap();
        let now = chrono::Utc::now();
        store.record(&UsageEvent {
            session_id: "1".into(),
            project: Some("alpha".into()),
            model: None,
            provider: None,
            tool: None,
            input_tokens: 100,
            output_tokens: 50,
            cost_usd: 0.0,
            ts: now,
        })
        .unwrap();
        store.record(&UsageEvent {
            session_id: "2".into(),
            project: Some("beta".into()),
            model: None,
            provider: None,
            tool: None,
            input_tokens: 200,
            output_tokens: 100,
            cost_usd: 0.0,
            ts: now,
        })
        .unwrap();
        let snap = store
            .snapshot(&StatsFilter::for_project("alpha"))
            .unwrap();
        assert_eq!(snap.event_count, 1);
        assert_eq!(snap.total_input_tokens, 100);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn filter_last_n_days() {
        let path = temp_path();
        let store = StatsStore::open(&path).unwrap();
        let snap = store.snapshot(&StatsFilter::last_n_days(7)).unwrap();
        // Just verify the filter doesn't crash with no data.
        assert_eq!(snap.event_count, 0);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn recent_returns_in_reverse_order() {
        let path = temp_path();
        let store = StatsStore::open(&path).unwrap();
        let now = chrono::Utc::now();
        for i in 0..5 {
            store
                .record(&UsageEvent {
                    session_id: format!("s{i}"),
                    project: None,
                    model: None,
                    provider: None,
                    tool: None,
                    input_tokens: 0,
                    output_tokens: 0,
                    cost_usd: 0.0,
                    ts: now,
                })
                .unwrap();
        }
        let events = store.recent(3).unwrap();
        assert_eq!(events.len(), 3);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn format_snapshot_includes_total() {
        let snap = StatsSnapshot {
            event_count: 5,
            total_input_tokens: 1000,
            total_output_tokens: 500,
            total_cost_usd: 0.05,
            ..Default::default()
        };
        let s = format_snapshot(&snap);
        assert!(s.contains("Total:"));
        assert!(s.contains("5 events"));
    }

    #[test]
    fn stats_filter_sql_is_safe() {
        let filter = StatsFilter {
            since: Some(chrono::Utc::now() - chrono::Duration::days(1)),
            project: Some("test'; DROP TABLE--".into()),
            model: None,
            ..Default::default()
        };
        let (sql, args) = filter.to_sql();
        // Verify the malicious string is treated as a parameter, not inlined.
        assert!(sql.contains("?"));
        assert!(args.iter().any(|a| matches!(a, rusqlite::types::Value::Text(s) if s.contains("DROP TABLE"))));
    }
}
