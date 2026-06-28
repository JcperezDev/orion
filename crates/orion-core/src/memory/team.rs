//! Team memory synchronization — shared, append-only JSONL fact/convention store.
//!
//! Compatible with the Token God / Claude memory scope concept. Allows teams
//! to persist shared project facts/conventions in a single file
//! (e.g. `.orion/team_memory.jsonl` in the project root) that is easy to
//! commit, sync via git, and merge across team members.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Individual team memory fact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMemoryEntry {
    /// Unique fact ID.
    pub id: String,
    /// Author display name (e.g. "jcperez").
    pub author: String,
    /// Fact scope/namespace (e.g. "orion-core").
    pub scope: String,
    /// Tags for categorization (e.g. ["db", "convention"]).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// The actual fact text (e.g. "Use WAL mode for all SQLite databases").
    pub text: String,
    /// ISO 8601 creation timestamp.
    pub ts: String,
}

/// Team memory collection backed by a local or shared JSONL file.
#[derive(Debug, Default, Clone)]
pub struct TeamMemory {
    entries: Vec<TeamMemoryEntry>,
}

impl TeamMemory {
    pub fn new() -> Self {
        Self::default()
    }

    /// Load entries from a JSONL file.
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("reading team memory from {}", path.display()))?;
        let mut entries = Vec::new();
        for line in raw.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Ok(entry) = serde_json::from_str::<TeamMemoryEntry>(trimmed) {
                entries.push(entry);
            }
        }
        Ok(Self { entries })
    }

    /// Append a single fact to the JSONL file on disk (and insert in-memory).
    pub fn append_to_file(&mut self, path: impl AsRef<Path>, mut entry: TeamMemoryEntry) -> Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        if entry.id.is_empty() {
            entry.id = uuid::Uuid::new_v4().to_string();
        }
        if entry.ts.is_empty() {
            entry.ts = chrono::Utc::now().to_rfc3339();
        }
        let line = serde_json::to_string(&entry)?;
        
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        writeln!(file, "{line}")?;
        
        self.entries.push(entry);
        Ok(())
    }

    /// Merge another team memory collection, deduplicating by ID.
    pub fn merge(&mut self, other: TeamMemory) {
        let mut existing_ids: HashSet<String> = self.entries.iter().map(|e| e.id.clone()).collect();
        for entry in other.entries {
            if !existing_ids.contains(&entry.id) {
                existing_ids.insert(entry.id.clone());
                self.entries.push(entry);
            }
        }
    }

    /// Save the entire merged collection back to a JSONL file (overwrites).
    pub fn save_to_file(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let mut out = String::new();
        for entry in &self.entries {
            out.push_str(&serde_json::to_string(entry)?);
            out.push('\n');
        }
        std::fs::write(path, out)?;
        Ok(())
    }

    /// List all loaded facts.
    pub fn list(&self) -> &[TeamMemoryEntry] {
        &self.entries
    }

    /// Count loaded facts.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Search facts using case-insensitive keyword match across text, tags, and author.
    pub fn search(&self, query: &str) -> Vec<&TeamMemoryEntry> {
        let query_lower = query.to_lowercase();
        let keywords: Vec<&str> = query_lower.split_whitespace().collect();
        let mut scored: Vec<(&TeamMemoryEntry, usize)> = self
            .entries
            .iter()
            .map(|e| {
                let text = e.text.to_lowercase();
                let author = e.author.to_lowercase();
                let scope = e.scope.to_lowercase();
                let mut score = 0;
                for kw in &keywords {
                    if text.contains(kw) {
                        score += 3;
                    }
                    if author.contains(kw) {
                        score += 1;
                    }
                    if scope.contains(kw) {
                        score += 2;
                    }
                    if e.tags.iter().any(|t| t.to_lowercase().contains(kw)) {
                        score += 2;
                    }
                }
                (e, score)
            })
            .filter(|(_, s)| *s > 0)
            .collect();
        scored.sort_by(|a, b| b.1.cmp(&a.1));
        scored.into_iter().map(|(e, _)| e).collect()
    }
}

/// Auto-locate the project team memory file (`.orion/team_memory.jsonl`).
pub fn project_team_memory_path() -> PathBuf {
    std::env::current_dir()
        .map(|cwd| cwd.join(".orion").join("team_memory.jsonl"))
        .unwrap_or_else(|_| PathBuf::from(".orion").join("team_memory.jsonl"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_file() -> PathBuf {
        std::env::temp_dir().join(format!(
            "orion-team-memory-{}-{}.jsonl",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn load_nonexistent_returns_empty() {
        let path = PathBuf::from("/nonexistent/file.jsonl");
        let mem = TeamMemory::load_from_file(&path).unwrap();
        assert!(mem.is_empty());
    }

    #[test]
    fn append_and_load_roundtrip() {
        let path = temp_file();
        let mut mem = TeamMemory::new();
        mem.append_to_file(
            &path,
            TeamMemoryEntry {
                id: "f1".into(),
                author: "alice".into(),
                scope: "core".into(),
                tags: vec!["db".into()],
                text: "Use sqlite".into(),
                ts: "".into(),
            },
        )
        .unwrap();

        assert_eq!(mem.len(), 1);

        let mem2 = TeamMemory::load_from_file(&path).unwrap();
        assert_eq!(mem2.len(), 1);
        let entry = &mem2.list()[0];
        assert_eq!(entry.id, "f1");
        assert_eq!(entry.author, "alice");
        assert_eq!(entry.text, "Use sqlite");
        assert!(!entry.ts.is_empty());

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn merge_deduplicates_by_id() {
        let mut m1 = TeamMemory {
            entries: vec![TeamMemoryEntry {
                id: "dup".into(),
                author: "a".into(),
                scope: "s".into(),
                tags: vec![],
                text: "v1".into(),
                ts: "2026-01-01".into(),
            }],
        };
        let m2 = TeamMemory {
            entries: vec![
                TeamMemoryEntry {
                    id: "dup".into(),
                    author: "b".into(),
                    scope: "s".into(),
                    tags: vec![],
                    text: "v2".into(),
                    ts: "2026-01-02".into(),
                },
                TeamMemoryEntry {
                    id: "unique".into(),
                    author: "c".into(),
                    scope: "s".into(),
                    tags: vec![],
                    text: "v3".into(),
                    ts: "2026-01-03".into(),
                },
            ],
        };
        m1.merge(m2);
        assert_eq!(m1.len(), 2);
        assert_eq!(m1.list()[0].id, "dup");
        assert_eq!(m1.list()[0].author, "a"); // preserved original
        assert_eq!(m1.list()[1].id, "unique");
    }

    #[test]
    fn search_ranks_by_relevance() {
        let mem = TeamMemory {
            entries: vec![
                TeamMemoryEntry {
                    id: "1".into(),
                    author: "alice".into(),
                    scope: "core".into(),
                    tags: vec!["db".into()],
                    text: "Always use WAL mode in sqlite".into(),
                    ts: "".into(),
                },
                TeamMemoryEntry {
                    id: "2".into(),
                    author: "bob".into(),
                    scope: "tui".into(),
                    tags: vec!["ui".into()],
                    text: "Custom color themes should be configurable".into(),
                    ts: "".into(),
                },
            ],
        };
        let hits = mem.search("sqlite WAL");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "1");

        let hits2 = mem.search("bob theme");
        assert_eq!(hits2.len(), 1);
        assert_eq!(hits2[0].id, "2");
    }
}
