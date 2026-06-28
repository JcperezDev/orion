use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Frontmatter {
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub permission: Option<serde_json::Value>,
    #[serde(default)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct ProjectMemory {
    pub path: PathBuf,
    pub mtime: SystemTime,
    pub body: String,
    pub frontmatter: Frontmatter,
}

pub struct ProjectMemoryLoader {
    cwd: PathBuf,
}

impl ProjectMemoryLoader {
    pub fn new(cwd: PathBuf) -> Self {
        Self { cwd }
    }

    pub fn load(&self) -> Result<Vec<ProjectMemory>> {
        let mut out = Vec::new();
        let candidates = [
            self.cwd.join("AGENTS.md"),
            self.cwd.join("ORION.md"),
            home_config().join("AGENTS.md"),
            home_config().join("ORION.md"),
        ];
        for p in candidates {
            match self.load_one(&p) {
                Ok(Some(mem)) => out.push(mem),
                Ok(None) => {}
                Err(e) => tracing::warn!(path = %p.display(), "agents.md load error: {e}"),
            }
        }
        Ok(out)
    }

    fn load_one(&self, path: &Path) -> Result<Option<ProjectMemory>> {
        if !path.exists() {
            return Ok(None);
        }
        let meta = std::fs::metadata(path)
            .with_context(|| format!("stat {path:?}"))?;
        let mtime = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("read {path:?}"))?;
        let (frontmatter, body) = parse_frontmatter(&raw);
        Ok(Some(ProjectMemory {
            path: path.to_path_buf(),
            mtime,
            body,
            frontmatter,
        }))
    }
}

fn parse_frontmatter(raw: &str) -> (Frontmatter, String) {
    let trimmed = raw.trim_start_matches('\u{feff}');
    if !trimmed.starts_with("---") {
        return (Frontmatter::default(), raw.to_string());
    }
    let after = &trimmed[3..];
    let after = after.trim_start_matches('\r').trim_start_matches('\n');
    if let Some(end_idx) = after.find("\n---") {
        let yaml_block = &after[..end_idx];
        let body_start = end_idx + 4;
        let body = after[body_start..]
            .trim_start_matches('\n')
            .trim_start_matches('\r')
            .to_string();
        let frontmatter = parse_simple_yaml(yaml_block);
        (frontmatter, body)
    } else {
        (Frontmatter::default(), raw.to_string())
    }
}

fn parse_simple_yaml(block: &str) -> Frontmatter {
    let mut fm = Frontmatter::default();
    for line in block.lines() {
        let line = line.trim_end();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once(':') {
            let key = k.trim().to_string();
            let val = v.trim().trim_matches('"').trim_matches('\'');
            match key.as_str() {
                "description" => fm.description = Some(val.to_string()),
                "model" => fm.model = Some(val.to_string()),
                "permission" => {
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(val) {
                        fm.permission = Some(parsed);
                    }
                }
                _ => {
                    fm.extra.insert(key, serde_json::Value::String(val.to_string()));
                }
            }
        }
    }
    fm
}

fn home_config() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("orion")
}

pub fn merged_system_prompt(loaded: &[ProjectMemory]) -> String {
    if loaded.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    out.push_str("# Project memory (auto-loaded)\n\n");
    for mem in loaded {
        out.push_str(&format!(
            "## {} (from {})\n\n",
            mem.path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| mem.path.to_string_lossy().to_string()),
            mem.path.display()
        ));
        out.push_str(&mem.body);
        out.push_str("\n\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_frontmatter_and_body() {
        let raw = "---\ndescription: hello\nmodel: claude-3.5\n---\n# Body\n\nSome content.";
        let (fm, body) = parse_frontmatter(raw);
        assert_eq!(fm.description.as_deref(), Some("hello"));
        assert_eq!(fm.model.as_deref(), Some("claude-3.5"));
        assert!(body.starts_with("# Body"));
    }

    #[test]
    fn no_frontmatter_passes_through() {
        let raw = "# Just markdown\n\nWith body.";
        let (fm, body) = parse_frontmatter(raw);
        assert!(fm.description.is_none());
        assert!(body.contains("With body"));
    }

    #[test]
    fn merged_prompt_is_empty_when_no_files() {
        let s = merged_system_prompt(&[]);
        assert!(s.is_empty());
    }
}
