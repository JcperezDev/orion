use crate::tools::{PermissionKind, Tool, ToolContext, ToolResult};
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::Deserialize;
use std::path::Path;

const MAX_BYTES: u64 = 200_000;

pub struct ReadTool;

#[derive(Debug, Deserialize)]
struct ReadArgs {
    path: String,
    #[serde(default)]
    offset: Option<usize>,
    #[serde(default)]
    limit: Option<usize>,
}

#[async_trait]
impl Tool for ReadTool {
    fn name(&self) -> &str {
        "read"
    }
    fn description(&self) -> &str {
        "Read the contents of a file. Returns up to ~200KB; for larger files use offset/limit."
    }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Absolute or relative path to the file to read."
                },
                "offset": {
                    "type": "integer",
                    "description": "Line number to start from (0-indexed)."
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of lines to read."
                }
            },
            "required": ["path"]
        })
    }
    fn requires_permission(&self) -> PermissionKind {
        PermissionKind::Filesystem
    }
    fn action_summary(&self, args: &serde_json::Value) -> String {
        args.get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("?")
            .to_string()
    }

    async fn execute(
        &self,
        args: serde_json::Value,
        ctx: &ToolContext,
    ) -> Result<ToolResult> {
        let args: ReadArgs = serde_json::from_value(args)
            .context("invalid args for read tool")?;

        let path = resolve_path(&ctx.cwd, &args.path);

        let meta = tokio::fs::metadata(&path)
            .await
            .with_context(|| format!("stat {path:?}"))?;
        if !meta.is_file() {
            anyhow::bail!("not a file: {path:?}");
        }

        let bytes_to_read = meta.len().min(MAX_BYTES);
        let mut f = tokio::fs::File::open(&path)
            .await
            .with_context(|| format!("open {path:?}"))?;
        let mut buf = vec![0u8; bytes_to_read as usize];
        use tokio::io::AsyncReadExt;
        f.read_exact(&mut buf)
            .await
            .with_context(|| format!("read {path:?}"))?;

        let mut text = String::from_utf8_lossy(&buf).to_string();
        if let (Some(off), Some(lim)) = (args.offset, args.limit) {
            let lines: Vec<&str> = text.lines().collect();
            let end = (off + lim).min(lines.len());
            if off < lines.len() {
                text = lines[off..end].join("\n");
            } else {
                text = String::new();
            }
        }

        Ok(ToolResult {
            tool_call_id: String::new(),
            content: text,
            is_error: false,
        })
    }
}

pub(crate) fn resolve_path(cwd: &Path, path: &str) -> std::path::PathBuf {
    let p = std::path::PathBuf::from(path);
    if p.is_absolute() {
        p
    } else {
        cwd.join(p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[tokio::test]
    async fn reads_existing_file() {
        let dir = tempdir_workaround();
        let f = dir.join("hello.txt");
        std::fs::write(&f, "line1\nline2\nline3\n").unwrap();

        let ctx = ToolContext::new(dir.clone());
        let r = ReadTool
            .execute(
                serde_json::json!({"path": "hello.txt"}),
                &ctx,
            )
            .await
            .unwrap();
        assert!(!r.is_error);
        assert!(r.content.contains("line2"));
    }

    #[tokio::test]
    async fn missing_file_is_error() {
        let dir = tempdir_workaround();
        let ctx = ToolContext::new(dir.clone());
        let err = ReadTool
            .execute(serde_json::json!({"path": "nope.txt"}), &ctx)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("nope.txt"));
    }

    fn tempdir_workaround() -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!(
            "orion-test-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }
}
