use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct PatchEntry {
    pub path: PathBuf,
    pub before: Option<String>,
    pub after: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StepSnapshot {
    pub step: usize,
    pub patches: Vec<PatchEntry>,
}

#[derive(Debug, Default)]
pub struct SnapshotManager {
    files: HashMap<PathBuf, String>,
}

fn resolve_path(cwd: &Path, raw: &str) -> PathBuf {
    let p = PathBuf::from(raw);
    if p.is_absolute() {
        p
    } else {
        cwd.join(&p)
    }
}

impl SnapshotManager {
    pub fn new() -> Self {
        Self { files: HashMap::new() }
    }

    pub fn clear(&mut self) {
        self.files.clear();
    }

    pub fn capture(&mut self, paths: &[PathBuf]) {
        for path in paths {
            if let Ok(content) = std::fs::read_to_string(path) {
                self.files.insert(path.clone(), content);
            }
        }
    }

    /// Return the captured "before" content for the given paths (None when the
    /// file did not exist at capture time — undoing it means deleting the file).
    pub fn captured(&self, paths: &[PathBuf]) -> Vec<(PathBuf, Option<String>)> {
        paths
            .iter()
            .map(|p| (p.clone(), self.files.get(p).cloned()))
            .collect()
    }

    pub fn diff(&self) -> Vec<PatchEntry> {
        let mut entries = Vec::new();
        for (path, before) in &self.files {
            match std::fs::read_to_string(path) {
                Ok(after) => {
                    if &after != before {
                        entries.push(PatchEntry {
                            path: path.clone(),
                            before: Some(before.clone()),
                            after: Some(after),
                        });
                    }
                }
                Err(_) => {
                    entries.push(PatchEntry {
                        path: path.clone(),
                        before: Some(before.clone()),
                        after: None,
                    });
                }
            }
        }
        entries
    }

    pub fn revert(&self) -> std::io::Result<()> {
        for (path, content) in &self.files {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(path, content)?;
        }
        Ok(())
    }

    pub fn extract_targets(calls: &[crate::tools::ToolCall], cwd: &Path) -> Vec<PathBuf> {
        let mut paths: Vec<PathBuf> = Vec::new();
        for call in calls {
            match call.name.as_str() {
                "write" | "edit" => {
                    if let Some(raw) = call.arguments.get("path").and_then(|v| v.as_str()) {
                        paths.push(resolve_path(cwd, raw));
                    }
                }
                "apply_patch" => {
                    if let Some(text) = call.arguments.get("patch_text").and_then(|v| v.as_str()) {
                        for line in text.lines() {
                            if let Some(p) = line.strip_prefix("+++ ") {
                                let p = p.trim_start_matches("b/");
                                paths.push(resolve_path(cwd, p));
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        paths
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_and_diff_modified_file() {
        let dir = std::env::temp_dir().join(format!("snap-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("f.txt");
        std::fs::write(&path, "before").unwrap();

        let mut snap = SnapshotManager::new();
        snap.capture(&[path.clone()]);
        std::fs::write(&path, "after").unwrap();

        let patches = snap.diff();
        assert_eq!(patches.len(), 1);
        assert_eq!(patches[0].before.as_deref(), Some("before"));
        assert_eq!(patches[0].after.as_deref(), Some("after"));
    }

    #[test]
    fn capture_and_diff_unchanged_file() {
        let dir = std::env::temp_dir().join(format!("snap2-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("f.txt");
        std::fs::write(&path, "same").unwrap();

        let mut snap = SnapshotManager::new();
        snap.capture(&[path.clone()]);
        let patches = snap.diff();
        assert!(patches.is_empty());
    }

    #[test]
    fn revert_restores_original() {
        let dir = std::env::temp_dir().join(format!("snap3-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("f.txt");
        std::fs::write(&path, "original").unwrap();

        let mut snap = SnapshotManager::new();
        snap.capture(&[path.clone()]);
        std::fs::write(&path, "modified").unwrap();
        snap.revert().unwrap();

        assert_eq!(std::fs::read_to_string(&path).unwrap(), "original");
    }

    #[test]
    fn captured_returns_before_content_and_none_for_new_files() {
        let dir = std::env::temp_dir().join(format!("snap-cap-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let existing = dir.join("existing.txt");
        std::fs::write(&existing, "original").unwrap();
        let brand_new = dir.join("new.txt");

        let mut snap = SnapshotManager::new();
        snap.capture(&[existing.clone(), brand_new.clone()]);

        let captured = snap.captured(&[existing.clone(), brand_new.clone()]);
        assert_eq!(captured[0], (existing, Some("original".to_string())));
        // A file that didn't exist at capture time has no before-content.
        assert_eq!(captured[1], (brand_new, None));
    }

    #[test]
    fn extract_targets_from_write_call() {
        let calls = vec![crate::tools::ToolCall {
            id: "c1".into(),
            name: "write".into(),
            arguments: serde_json::json!({"path": "src/main.rs", "content": "fn main() {}"}),
        }];
        let cwd = PathBuf::from("/project");
        let targets = SnapshotManager::extract_targets(&calls, &cwd);
        assert_eq!(targets, vec![PathBuf::from("/project/src/main.rs")]);
    }

    #[test]
    fn extract_targets_from_apply_patch() {
        let calls = vec![crate::tools::ToolCall {
            id: "c1".into(),
            name: "apply_patch".into(),
            arguments: serde_json::json!({
                "patch_text": "--- a/src/main.rs\n+++ b/src/main.rs\n@@ ... @@\n-old\n+new\n"
            }),
        }];
        let cwd = PathBuf::from("/project");
        let targets = SnapshotManager::extract_targets(&calls, &cwd);
        assert_eq!(targets, vec![PathBuf::from("/project/src/main.rs")]);
    }

    #[test]
    fn extract_targets_skips_unknown_tool() {
        let calls = vec![crate::tools::ToolCall {
            id: "c1".into(),
            name: "bash".into(),
            arguments: serde_json::json!({"command": "ls"}),
        }];
        let targets = SnapshotManager::extract_targets(&calls, &PathBuf::from("/p"));
        assert!(targets.is_empty());
    }
}
