use crate::tools::{ApprovalRequest, ApprovalResponse, PermissionKind, Tool, ToolContext, ToolResult};
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub struct ApplyPatchTool {
    cwd: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
struct ApplyPatchArgs {
    patch_text: String,
}

#[derive(Debug, Serialize)]
struct AppliedItem {
    #[serde(rename = "type")]
    kind: String,
    resource: String,
}

impl ApplyPatchTool {
    pub fn new(cwd: PathBuf) -> Self {
        Self { cwd }
    }
}

#[async_trait]
impl Tool for ApplyPatchTool {
    fn name(&self) -> &str {
        "apply_patch"
    }

    fn description(&self) -> &str {
        "Apply one patch containing add, update, and delete file operations. All targets are resolved before operations apply sequentially. Uses unified diff format."
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "patch_text": {
                    "type": "string",
                    "description": "The full patch text describing add, update, and delete operations in unified diff format"
                }
            },
            "required": ["patch_text"]
        })
    }

    fn requires_permission(&self) -> PermissionKind {
        PermissionKind::Filesystem
    }

    fn action_summary(&self, args: &serde_json::Value) -> String {
        if let Some(text) = args.get("patch_text").and_then(|t| t.as_str()) {
            let lines: Vec<&str> = text.lines().filter(|l| l.starts_with("+++") || l.starts_with("---")).collect();
            if !lines.is_empty() {
                return format!("patch with {} files", lines.len() / 2);
            }
        }
        "apply_patch".to_string()
    }

    async fn execute(&self, args: serde_json::Value, ctx: &ToolContext) -> Result<ToolResult> {
        let args: ApplyPatchArgs =
            serde_json::from_value(args).context("invalid args for apply_patch tool")?;

        let request = ApprovalRequest {
            tool_name: "apply_patch".into(),
            action: format!("apply patch"),
            matched_pattern: None,
            arguments: serde_json::to_value(&args).unwrap_or_default(),
        };
        match ctx.ask(request).await {
            ApprovalResponse::Deny => {
                return Ok(ToolResult {
                    tool_call_id: String::new(),
                    content: "denied by user".into(),
                    is_error: true,
                });
            }
            ApprovalResponse::Allow | ApprovalResponse::AllowAlways => {}
        }

        let patch = args.patch_text;
        if patch.trim().is_empty() {
            return Ok(ToolResult {
                tool_call_id: String::new(),
                content: "patch_text is required".into(),
                is_error: true,
            });
        }

        let hunks = match parse_unified_diff(&patch) {
            Ok(h) => h,
            Err(e) => {
                return Ok(ToolResult {
                    tool_call_id: String::new(),
                    content: format!("patch parse error: {e}"),
                    is_error: true,
                });
            }
        };

        if hunks.is_empty() {
            return Ok(ToolResult {
                tool_call_id: String::new(),
                content: "patch rejected: empty patch".into(),
                is_error: true,
            });
        }

        let mut applied: Vec<AppliedItem> = Vec::new();

        for hunk in &hunks {
            let path = if hunk.new_path.starts_with('/') || hunk.new_path.starts_with('.') {
                PathBuf::from(&hunk.new_path)
            } else {
                self.cwd.join(&hunk.new_path)
            };

            match hunk.kind {
                HunkKind::Add => {
                    if let Some(parent) = path.parent() {
                        tokio::fs::create_dir_all(parent).await?;
                    }
                    tokio::fs::write(&path, &hunk.content).await?;
                    applied.push(AppliedItem {
                        kind: "add".into(),
                        resource: hunk.new_path.clone(),
                    });
                }
                HunkKind::Delete => {
                    if path.exists() {
                        tokio::fs::remove_file(&path).await?;
                    }
                    applied.push(AppliedItem {
                        kind: "delete".into(),
                        resource: hunk.new_path.clone(),
                    });
                }
                HunkKind::Update => {
                    if !path.exists() {
                        return Ok(ToolResult {
                            tool_call_id: String::new(),
                            content: format!("file not found for update: {}", hunk.new_path),
                            is_error: true,
                        });
                    }
                    let current = tokio::fs::read_to_string(&path).await?;
                    let updated = apply_chunks(&current, &hunk.chunks)?;
                    tokio::fs::write(&path, &updated).await?;
                    applied.push(AppliedItem {
                        kind: "update".into(),
                        resource: hunk.new_path.clone(),
                    });
                }
            }
        }

        let output = serde_json::json!({ "applied": applied });
        Ok(ToolResult {
            tool_call_id: String::new(),
            content: format!("Applied patch:\n{}", serde_json::to_string_pretty(&output).unwrap_or_default()),
            is_error: false,
        })
    }
}

#[derive(Debug)]
enum HunkKind {
    Add,
    Delete,
    Update,
}

#[derive(Debug)]
struct Hunk {
    kind: HunkKind,
    #[allow(dead_code)]
    old_path: String,
    new_path: String,
    chunks: Vec<Chunk>,
    content: String,
}

#[derive(Debug)]
struct Chunk {
    lines: Vec<String>,
}

fn parse_unified_diff(patch: &str) -> Result<Vec<Hunk>> {
    let mut hunks = Vec::new();
    let mut lines = patch.lines().peekable();

    while let Some(line) = lines.next() {
        if line.starts_with("--- ") {
            let old_path = line[4..].trim().trim_start_matches("a/").to_string();
            let new_path = lines
                .next()
                .and_then(|l| {
                    if l.starts_with("+++ ") {
                        Some(l[4..].trim().trim_start_matches("b/").to_string())
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| old_path.clone());

            let mut chunks = Vec::new();
            let mut content_lines = Vec::new();

            while let Some(l) = lines.next() {
                if l.starts_with("--- ") {
                    content_lines.push(l.to_string());
                    if let Some(&next) = lines.peek() {
                        if next.starts_with("+++ ") {
                            break;
                        }
                    }
                    break;
                }
                content_lines.push(l.to_string());
            }

            let kind = if old_path == "/dev/null" {
                HunkKind::Add
            } else if new_path == "/dev/null" {
                HunkKind::Delete
            } else {
                HunkKind::Update
            };

            let content = content_lines.join("\n");
            let mut in_chunk = false;
            let mut chunk_lines = Vec::new();

            for cl in &content_lines {
                if cl.starts_with("@@ ") {
                    if in_chunk && !chunk_lines.is_empty() {
                        chunks.push(Chunk { lines: chunk_lines.clone() });
                        chunk_lines.clear();
                    }
                    in_chunk = true;
                    continue;
                }
                if in_chunk && (cl.starts_with(' ') || cl.starts_with('+') || cl.starts_with('-')) {
                    chunk_lines.push(cl.clone());
                }
            }
            if in_chunk && !chunk_lines.is_empty() {
                chunks.push(Chunk { lines: chunk_lines });
            }

            hunks.push(Hunk {
                kind,
                old_path,
                new_path,
                chunks,
                content,
            });
        }
    }

    Ok(hunks)
}

fn apply_chunks(original: &str, chunks: &[Chunk]) -> Result<String> {
    let mut result = original.to_string();

    for chunk in chunks.iter().rev() {
        let mut remove_lines = Vec::new();
        let mut add_lines = Vec::new();
        let mut context_before = Vec::new();
        let mut context_after = Vec::new();
        let mut in_remove = false;
        let mut in_add = false;

        for line in &chunk.lines {
            if line.starts_with('-') {
                remove_lines.push(&line[1..]);
                in_remove = true;
                in_add = false;
            } else if line.starts_with('+') {
                add_lines.push(&line[1..]);
                in_add = true;
                in_remove = false;
            } else if line.starts_with(' ') {
                if in_remove || in_add {
                    context_after.push(&line[1..]);
                } else {
                    context_before.push(&line[1..]);
                }
                in_remove = false;
                in_add = false;
            }
        }

        if remove_lines.is_empty() {
            continue;
        }

        let search: Vec<&str> = remove_lines.iter().copied().collect();
        let replacement: Vec<&str> = add_lines.iter().copied().collect();

        if let Some(pos) = find_sequence(&result, &search, &context_before, &context_after) {
            let before: String = result.lines().take(pos).collect::<Vec<_>>().join("\n");
            let after: String = result.lines().skip(pos + search.len()).collect::<Vec<_>>().join("\n");

            let mut new_parts = Vec::new();
            if !before.is_empty() { new_parts.push(before); }
            if !replacement.is_empty() {
                new_parts.push(replacement.join("\n"));
            }
            if !after.is_empty() { new_parts.push(after); }

            result = new_parts.join("\n");
        }
    }

    Ok(result)
}

fn find_sequence(content: &str, search: &[&str], before_context: &[&str], after_context: &[&str]) -> Option<usize> {
    let lines: Vec<&str> = content.lines().collect();

    if search.is_empty() {
        return None;
    }

    for i in 0..lines.len() {
        if i + search.len() <= lines.len() {
            let matches = search.iter().enumerate().all(|(j, s)| lines[i + j].trim() == s.trim());

            if matches {
                let ctx_before_match = if !before_context.is_empty() {
                    i >= before_context.len()
                        && before_context.iter().enumerate().all(|(j, s)| {
                            lines[i - before_context.len() + j].trim() == s.trim()
                        })
                } else {
                    true
                };

                let ctx_after_match = if !after_context.is_empty() {
                    i + search.len() + after_context.len() <= lines.len()
                        && after_context.iter().enumerate().all(|(j, s)| {
                            lines[i + search.len() + j].trim() == s.trim()
                        })
                } else {
                    true
                };

                if ctx_before_match && ctx_after_match {
                    return Some(i);
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_add_diff() {
        let patch = "--- /dev/null\n+++ b/new_file.txt\n@@ ... @@\n+hello world\n";
        let hunks = parse_unified_diff(patch).unwrap();
        assert_eq!(hunks.len(), 1);
        assert!(matches!(hunks[0].kind, HunkKind::Add));
    }

    #[test]
    fn parse_simple_delete_diff() {
        let patch = "--- a/file.txt\n+++ /dev/null\n@@ ... @@\n-line to remove\n";
        let hunks = parse_unified_diff(patch).unwrap();
        assert_eq!(hunks.len(), 1);
        assert!(matches!(hunks[0].kind, HunkKind::Delete));
    }

    #[test]
    fn parse_simple_update_diff() {
        let patch = "--- a/file.txt\n+++ b/file.txt\n@@ ... @@\n-old line\n+new line\n";
        let hunks = parse_unified_diff(patch).unwrap();
        assert_eq!(hunks.len(), 1);
        assert!(matches!(hunks[0].kind, HunkKind::Update));
    }

    #[test]
    fn apply_simple_replacement() {
        let content = "line1\nold line\nline3";
        let chunks = vec![Chunk {
            lines: vec!["-old line".to_string(), "+new line".to_string()],
        }];
        let result = apply_chunks(content, &chunks).unwrap();
        assert_eq!(result, "line1\nnew line\nline3");
    }
}
