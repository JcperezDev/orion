use crate::tools::{PermissionKind, Tool, ToolContext, ToolResult};
use crate::tools::read::resolve_path;
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::Deserialize;

pub struct WriteTool;

#[derive(Debug, Deserialize)]
struct WriteArgs {
    path: String,
    content: String,
}

#[async_trait]
impl Tool for WriteTool {
    fn name(&self) -> &str {
        "write"
    }
    fn description(&self) -> &str {
        "Create or overwrite a file with the given content."
    }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "Path to write to (absolute or relative to cwd)."},
                "content": {"type": "string", "description": "Full file contents."}
            },
            "required": ["path", "content"]
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
        let args: WriteArgs = serde_json::from_value(args)
            .context("invalid args for write tool")?;

        let path = resolve_path(&ctx.cwd, &args.path);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await.ok();
        }

        let bytes = args.content.as_bytes();
        let count = bytes.len();
        tokio::fs::write(&path, bytes)
            .await
            .with_context(|| format!("write {path:?}"))?;

        Ok(ToolResult {
            tool_call_id: String::new(),
            content: format!("Wrote {} bytes to {}", count, path.display()),
            is_error: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn writes_new_file() {
        let dir = std::env::temp_dir().join(format!(
            "orion-w-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let ctx = ToolContext::new(dir.clone());
        let r = WriteTool
            .execute(
                serde_json::json!({"path": "f.txt", "content": "hello"}),
                &ctx,
            )
            .await
            .unwrap();
        assert!(!r.is_error);
        assert!(dir.join("f.txt").exists());
    }
}
