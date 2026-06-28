use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime};

static SPILL_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Clone)]
pub struct SpillManager {
    pub spill_dir: PathBuf,
    pub max_inline_bytes: usize,
}

impl SpillManager {
    pub fn new(spill_dir: PathBuf) -> Self {
        std::fs::create_dir_all(&spill_dir).ok();
        Self {
            spill_dir,
            max_inline_bytes: 30_000,
        }
    }

    pub fn new_temp() -> Self {
        let dir = std::env::temp_dir().join("orion-spill");
        Self::new(dir)
    }

    /// Write oversized content to disk and return a reference string.
    /// Returns `None` if the content is small enough to keep inline.
    pub fn spill(&self, content: &str, tool_call_id: &str) -> Result<Option<String>> {
        if content.len() <= self.max_inline_bytes {
            return Ok(None);
        }
        let counter = SPILL_COUNTER.fetch_add(1, Ordering::Relaxed);
        let ts = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let filename = format!("{ts}-{counter}-{tool_call_id}.txt");
        let path = self.spill_dir.join(&filename);
        std::fs::write(&path, content).context("spill write")?;
        Ok(Some(format!(
            "[Output ({}) spilled to {}]",
            content.len(),
            path.display(),
        )))
    }

    /// Read a spilled file back by its reference path.
    pub fn read_spilled(&self, path: &str) -> Result<String> {
        let path = if path.starts_with('/') {
            PathBuf::from(path)
        } else {
            self.spill_dir.join(path)
        };
        std::fs::read_to_string(&path).context("read spilled")
    }

    /// Remove spilled files older than `max_age`.
    pub fn cleanup_old(&self, max_age: Duration) -> Result<usize> {
        let now = SystemTime::now();
        let mut removed = 0;
        if let Ok(entries) = std::fs::read_dir(&self.spill_dir) {
            for entry in entries.flatten() {
                if let Ok(meta) = entry.metadata() {
                    if let Ok(modified) = meta.modified() {
                        if now.duration_since(modified).unwrap_or_default() > max_age {
                            std::fs::remove_file(entry.path()).ok();
                            removed += 1;
                        }
                    }
                }
            }
        }
        Ok(removed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_content_stays_inline() {
        let mgr = SpillManager::new_temp();
        let r = mgr.spill("small", "test1").unwrap();
        assert!(r.is_none());
    }

    #[test]
    fn large_content_spills_to_disk() {
        let mgr = SpillManager {
            max_inline_bytes: 10,
            ..SpillManager::new_temp()
        };
        let big = "x".repeat(100);
        let r = mgr.spill(&big, "test2").unwrap();
        assert!(r.is_some());
        let ref_text = r.unwrap();
        assert!(ref_text.contains("spilled"));
        assert!(ref_text.contains("test2"));
    }

    #[test]
    fn spill_then_read_back() {
        let mgr = SpillManager {
            max_inline_bytes: 10,
            ..SpillManager::new_temp()
        };
        let content = "data to spill and read";
        let r = mgr.spill(content, "roundtrip").unwrap();
        assert!(r.is_some());
        let ref_text = r.unwrap();
        let path = ref_text.split("spilled to ").nth(1).unwrap().trim_end_matches(']');
        let read_back = mgr.read_spilled(path).unwrap();
        assert_eq!(read_back, content);
    }

    #[test]
    fn read_spilled_content_back() {
        let mgr = SpillManager {
            max_inline_bytes: 10,
            ..SpillManager::new_temp()
        };
        let big = "hello spill world";
        let r = mgr.spill(big, "readtest").unwrap().unwrap();
        // extract path from reference
        let path = r.split("spilled to ").nth(1).unwrap().trim_end_matches(']');
        let content = mgr.read_spilled(path).unwrap();
        assert_eq!(content, "hello spill world");
    }
}
