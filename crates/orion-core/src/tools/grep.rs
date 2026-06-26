use crate::tools::read::resolve_path;
use crate::tools::{PermissionKind, Tool, ToolContext, ToolResult};
use anyhow::{Context, Result};
use async_trait::async_trait;
use regex::Regex;
use serde::Deserialize;

pub struct GrepTool;

#[derive(Debug, Deserialize)]
struct GrepArgs {
    pattern: String,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    include: Option<String>,
    #[serde(default)]
    context: Option<usize>,
}

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &str {
        "grep"
    }
    fn description(&self) -> &str {
        "Regex search across files. Returns up to 200 matches with file:line:text format."
    }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {"type": "string", "description": "Regex pattern."},
                "path": {"type": "string", "description": "Directory or file to search (defaults to cwd)."},
                "include": {"type": "string", "description": "Glob filter for filenames (e.g. \"*.rs\")."},
                "context": {"type": "integer", "description": "Lines of context around each match."}
            },
            "required": ["pattern"]
        })
    }
    fn requires_permission(&self) -> PermissionKind {
        PermissionKind::Filesystem
    }
    fn action_summary(&self, args: &serde_json::Value) -> String {
        format!(
            "{} in {}",
            args.get("pattern").and_then(|v| v.as_str()).unwrap_or("?"),
            args.get("path").and_then(|v| v.as_str()).unwrap_or("cwd")
        )
    }

    async fn execute(
        &self,
        args: serde_json::Value,
        ctx: &ToolContext,
    ) -> Result<ToolResult> {
        let args: GrepArgs = serde_json::from_value(args)
            .context("invalid args for grep tool")?;

        let regex = Regex::new(&args.pattern)
            .with_context(|| format!("invalid regex: {}", args.pattern))?;
        let root = match &args.path {
            Some(p) => resolve_path(&ctx.cwd, p),
            None => ctx.cwd.clone(),
        };
        let include_glob = args.include.clone();
        let context_lines = args.context.unwrap_or(0);

        let mut results: Vec<String> = Vec::new();
        let max_matches = 200usize;

        for entry in walkdir::WalkDir::new(&root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                !(name.starts_with('.')
                    && name != "."
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
            if let Some(glob) = &include_glob {
                if let Ok(g) = globset::Glob::new(glob) {
                    let m = g.compile_matcher();
                    if !m.is_match(entry.file_name()) {
                        continue;
                    }
                }
            }
            let path = entry.path();
            let text = match tokio::fs::read_to_string(path).await {
                Ok(t) => t,
                Err(_) => continue,
            };
            for (i, line) in text.lines().enumerate() {
                if regex.is_match(line) {
                    if results.len() >= max_matches {
                        results.push(format!(
                            "[truncated at {max_matches} matches]"
                        ));
                        return Ok(ToolResult {
                            tool_call_id: String::new(),
                            content: results.join("\n"),
                            is_error: false,
                        });
                    }
                    let line_num = i + 1;
                    results.push(format!("{}:{}:{}", path.display(), line_num, line));
                    if context_lines > 0 {
                        let lines: Vec<&str> = text.lines().collect();
                        let start = i.saturating_sub(context_lines);
                        let end = (i + context_lines + 1).min(lines.len());
                        for ctx_line in &lines[start..end] {
                            if std::ptr::eq(*ctx_line, line) {
                                continue;
                            }
                            results.push(format!("  {ctx_line}"));
                        }
                    }
                }
            }
        }

        Ok(ToolResult {
            tool_call_id: String::new(),
            content: if results.is_empty() {
                "no matches".into()
            } else {
                results.join("\n")
            },
            is_error: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn finds_matches() {
        let dir = std::env::temp_dir().join(format!(
            "orion-grep-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.txt"), "hello\nworld\nhello again\n").unwrap();

        let ctx = ToolContext::new(dir.clone());
        let r = GrepTool
            .execute(
                serde_json::json!({"pattern": "hello", "path": dir.to_string_lossy()}),
                &ctx,
            )
            .await
            .unwrap();
        assert!(!r.is_error);
        assert_eq!(r.content.matches("hello").count(), 2);
    }
}
