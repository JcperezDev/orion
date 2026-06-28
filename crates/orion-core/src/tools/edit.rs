use crate::tools::{PermissionKind, Tool, ToolContext, ToolResult};
use crate::tools::read::resolve_path;
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::Deserialize;

pub struct EditTool;

#[derive(Debug, Deserialize)]
struct EditArgs {
    path: String,
    old_text: String,
    new_text: String,
    #[serde(default)]
    replace_all: Option<bool>,
}

#[async_trait]
impl Tool for EditTool {
    fn name(&self) -> &str {
        "edit"
    }
    fn description(&self) -> &str {
        "Replace exact text in a file. old_text must appear verbatim. With replace_all=true, replaces every occurrence."
    }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "File to edit."},
                "old_text": {"type": "string", "description": "Exact text to find (must be unique unless replace_all=true)."},
                "new_text": {"type": "string", "description": "Replacement text."},
                "replace_all": {"type": "boolean", "description": "Replace every occurrence instead of requiring uniqueness."}
            },
            "required": ["path", "old_text", "new_text"]
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
        let args: EditArgs = serde_json::from_value(args)
            .context("invalid args for edit tool")?;
        let path = resolve_path(&ctx.cwd, &args.path);

        let original = tokio::fs::read_to_string(&path)
            .await
            .with_context(|| format!("read {path:?}"))?;

        let replace_all = args.replace_all.unwrap_or(false);
        let new_content = if replace_all {
            original.replace(&args.old_text, &args.new_text)
        } else {
            let count = original.matches(&args.old_text).count();
            if count == 0 {
                anyhow::bail!(
                    "old_text not found in {} (0 occurrences)",
                    path.display()
                );
            }
            if count > 1 {
                anyhow::bail!(
                    "old_text appears {} times in {}; pass replace_all=true or make it unique",
                    count,
                    path.display()
                );
            }
            original.replacen(&args.old_text, &args.new_text, 1)
        };

        if new_content == original {
            anyhow::bail!("edit produced no change (old_text == new_text?)");
        }

        tokio::fs::write(&path, new_content.as_bytes())
            .await
            .with_context(|| format!("write {path:?}"))?;

        Ok(ToolResult {
            tool_call_id: String::new(),
            content: format!("Edited {}", path.display()),
            is_error: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn edits_unique_occurrence() {
        let dir = std::env::temp_dir().join(format!(
            "orion-e-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("a.txt");
        std::fs::write(&path, "hello world\n").unwrap();

        let ctx = ToolContext::new(dir.clone());
        let r = EditTool
            .execute(
                serde_json::json!({"path": "a.txt", "old_text": "hello", "new_text": "bye"}),
                &ctx,
            )
            .await
            .unwrap();
        assert!(!r.is_error);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "bye world\n");
    }

    #[tokio::test]
    async fn rejects_ambiguous_edit() {
        let dir = std::env::temp_dir().join(format!(
            "orion-e2-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("a.txt");
        std::fs::write(&path, "aaa aaa\n").unwrap();
        let ctx = ToolContext::new(dir.clone());
        let err = EditTool
            .execute(
                serde_json::json!({"path": "a.txt", "old_text": "aaa", "new_text": "b"}),
                &ctx,
            )
            .await
            .unwrap_err();
        assert!(err.to_string().contains("appears"));
    }

    #[tokio::test]
    async fn replace_all_works() {
        let dir = std::env::temp_dir().join(format!(
            "orion-e3-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("a.txt");
        std::fs::write(&path, "aaa aaa\n").unwrap();
        let ctx = ToolContext::new(dir.clone());
        EditTool
            .execute(
                serde_json::json!({"path": "a.txt", "old_text": "aaa", "new_text": "b", "replace_all": true}),
                &ctx,
            )
            .await
            .unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "b b\n");
    }
}
