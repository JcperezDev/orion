use crate::tools::read::resolve_path;
use crate::tools::{PermissionKind, Tool, ToolContext, ToolResult};
use anyhow::{Context, Result};
use async_trait::async_trait;
use globset::Glob;
use serde::Deserialize;
use std::time::SystemTime;

pub struct GlobTool;

#[derive(Debug, Deserialize)]
struct GlobArgs {
    pattern: String,
    #[serde(default)]
    path: Option<String>,
}

#[async_trait]
impl Tool for GlobTool {
    fn name(&self) -> &str {
        "glob"
    }
    fn description(&self) -> &str {
        "Find files matching a glob. Returns paths relative to the search root, newest first."
    }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {"type": "string", "description": "Glob pattern, e.g. \"**/*.rs\" or \"src/**/*.ts\"."},
                "path": {"type": "string", "description": "Directory to search (defaults to cwd)."}
            },
            "required": ["pattern"]
        })
    }
    fn requires_permission(&self) -> PermissionKind {
        PermissionKind::Filesystem
    }
    fn action_summary(&self, args: &serde_json::Value) -> String {
        args.get("pattern")
            .and_then(|v| v.as_str())
            .unwrap_or("?")
            .to_string()
    }

    async fn execute(
        &self,
        args: serde_json::Value,
        ctx: &ToolContext,
    ) -> Result<ToolResult> {
        let args: GlobArgs = serde_json::from_value(args)
            .context("invalid args for glob tool")?;
        let root = match &args.path {
            Some(p) => resolve_path(&ctx.cwd, p),
            None => ctx.cwd.clone(),
        };
        let matcher = Glob::new(&args.pattern)
            .with_context(|| format!("invalid glob: {}", args.pattern))?
            .compile_matcher();

        let mut hits: Vec<(SystemTime, std::path::PathBuf)> = Vec::new();
        for entry in walkdir::WalkDir::new(&root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                !(name.starts_with('.')
                    && e.depth() > 0
                    && (name == "node_modules" || name == "target" || name == "dist"))
            })
        {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            if !entry.file_type().is_file() {
                continue;
            }
            if matcher.is_match(entry.path()) {
                let mtime = entry
                    .metadata()
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .unwrap_or(SystemTime::UNIX_EPOCH);
                hits.push((mtime, entry.path().to_path_buf()));
            }
        }

        hits.sort_by(|a, b| b.0.cmp(&a.0));
        let lines: Vec<String> = hits
            .into_iter()
            .take(500)
            .map(|(_, p)| {
                p.strip_prefix(&root)
                    .map(|r| r.to_string_lossy().to_string())
                    .unwrap_or_else(|_| p.to_string_lossy().to_string())
            })
            .collect();

        Ok(ToolResult {
            tool_call_id: String::new(),
            content: if lines.is_empty() {
                "no matches".into()
            } else {
                lines.join("\n")
            },
            is_error: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn finds_files() {
        let dir = std::env::temp_dir().join(format!(
            "orion-glob-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(dir.join("a.rs"), "").unwrap();
        std::fs::write(dir.join("sub").join("b.rs"), "").unwrap();
        std::fs::write(dir.join("c.txt"), "").unwrap();

        let ctx = ToolContext::new(dir.clone());
        let r = GlobTool
            .execute(
                serde_json::json!({"pattern": "**/*.rs", "path": dir.to_string_lossy()}),
                &ctx,
            )
            .await
            .unwrap();
        assert!(!r.is_error);
        assert!(r.content.contains("a.rs"));
        assert!(r.content.contains("b.rs"));
        assert!(!r.content.contains("c.txt"));
    }
}
