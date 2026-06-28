use crate::tools::bash_parser::parse_commands;
use crate::tools::{ApprovalRequest, ApprovalResponse, PermissionKind, Tool, ToolContext, ToolResult};
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::process::Command;

const MAX_OUTPUT_BYTES: usize = 100_000;
const DEFAULT_TIMEOUT_SECS: u64 = 120;

pub struct BashTool {
    cwd: PathBuf,
}

#[derive(Debug, Deserialize, Serialize)]
struct BashArgs {
    command: String,
    #[serde(default)]
    timeout_secs: Option<u64>,
    #[serde(default)]
    env: Option<std::collections::HashMap<String, String>>,
}

impl BashTool {
    pub fn new(cwd: PathBuf) -> Self {
        Self { cwd }
    }
}

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &str {
        "bash"
    }
    fn description(&self) -> &str {
        "Execute a shell command in the project directory. Output is truncated at ~100KB; long-running commands honor timeout_secs."
    }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": {"type": "string", "description": "Shell command to run (passed to /bin/sh -c)."},
                "timeout_secs": {"type": "integer", "description": "Kill the process after this many seconds (default 120)."},
                "env": {"type": "object", "description": "Extra env vars to set for this command only."}
            },
            "required": ["command"]
        })
    }
    fn requires_permission(&self) -> PermissionKind {
        PermissionKind::Bash
    }
    fn action_summary(&self, args: &serde_json::Value) -> String {
        let raw = args.get("command")
            .and_then(|v| v.as_str())
            .unwrap_or("?")
            .to_string();
        let parsed = parse_commands(&raw);
        if parsed.len() <= 1 {
            return raw;
        }
        let steps: Vec<&str> = parsed.iter().map(|c| c.full_text.as_str()).collect();
        format!("[{}]", steps.join(" | "))
    }

    async fn execute(
        &self,
        args: serde_json::Value,
        ctx: &ToolContext,
    ) -> Result<ToolResult> {
        let args: BashArgs = serde_json::from_value(args)
            .context("invalid args for bash tool")?;

        let parsed_cmds = parse_commands(&args.command);
        let action_desc = if parsed_cmds.len() <= 1 {
            args.command.clone()
        } else {
            let steps: Vec<&str> = parsed_cmds.iter().map(|c| c.full_text.as_str()).collect();
            format!("[{}]", steps.join(" | "))
        };
        let request = ApprovalRequest {
            tool_name: "bash".into(),
            action: action_desc,
            matched_pattern: None,
            arguments: serde_json::json!({
                "command": args.command,
                "parsed": parsed_cmds.iter().map(|c| serde_json::json!({
                    "command": c.command,
                    "args": c.args,
                })).collect::<Vec<_>>(),
            }),
        };
        let decision = ctx.ask(request).await;
        match decision {
            ApprovalResponse::Deny => {
                return Ok(ToolResult {
                    tool_call_id: String::new(),
                    content: "denied by user".into(),
                    is_error: true,
                });
            }
            ApprovalResponse::Allow | ApprovalResponse::AllowAlways => {}
        }

        let timeout = Duration::from_secs(args.timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS));

        let mut cmd = Command::new("/bin/sh");
        cmd.arg("-c").arg(&args.command);
        cmd.current_dir(&self.cwd);
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        if let Some(env) = args.env {
            for (k, v) in env {
                cmd.env(k, v);
            }
        }

        let child = cmd.spawn().context("spawn shell")?;
        let mut child = child;
        let mut stdout = child.stdout.take().unwrap();
        let mut stderr = child.stderr.take().unwrap();

        let (out_tx, mut out_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(8);
        let out_task = tokio::spawn(async move {
            let mut buf = Vec::with_capacity(MAX_OUTPUT_BYTES);
            let mut tmp = [0u8; 4096];
            while let Ok(n) = stdout.read(&mut tmp).await {
                if n == 0 {
                    break;
                }
                buf.extend_from_slice(&tmp[..n]);
                if buf.len() >= MAX_OUTPUT_BYTES {
                    break;
                }
            }
            let _ = out_tx.send(buf).await;
        });
        let (err_tx, mut err_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(8);
        let err_task = tokio::spawn(async move {
            let mut buf = Vec::with_capacity(MAX_OUTPUT_BYTES);
            let mut tmp = [0u8; 4096];
            while let Ok(n) = stderr.read(&mut tmp).await {
                if n == 0 {
                    break;
                }
                buf.extend_from_slice(&tmp[..n]);
                if buf.len() >= MAX_OUTPUT_BYTES {
                    break;
                }
            }
            let _ = err_tx.send(buf).await;
        });

        let wait_res = tokio::time::timeout(timeout, async {
            let status = child.wait().await?;
            out_task.await.ok();
            err_task.await.ok();
            Ok::<_, anyhow::Error>(status)
        })
        .await;

        let (out_bytes, err_bytes, status) = match wait_res {
            Ok(Ok(status)) => {
                let out = out_rx.recv().await.unwrap_or_default();
                let err = err_rx.recv().await.unwrap_or_default();
                (out, err, Some(status))
            }
            Ok(Err(e)) => {
                return Ok(ToolResult {
                    tool_call_id: String::new(),
                    content: format!("spawn error: {e}"),
                    is_error: true,
                });
            }
            Err(_) => {
                let _ = child.kill().await;
                return Ok(ToolResult {
                    tool_call_id: String::new(),
                    content: format!("timeout after {timeout:?}"),
                    is_error: true,
                });
            }
        };

        let mut combined = String::from_utf8_lossy(&out_bytes).to_string();
        if !err_bytes.is_empty() {
            if !combined.is_empty() {
                combined.push_str("\n--- stderr ---\n");
            }
            combined.push_str(&String::from_utf8_lossy(&err_bytes));
        }
        let truncated = out_bytes.len() >= MAX_OUTPUT_BYTES || err_bytes.len() >= MAX_OUTPUT_BYTES;
        if truncated {
            combined.push_str("\n[output truncated]");
        }

        let exit_code = status.and_then(|s| s.code()).unwrap_or(-1);
        let is_error = exit_code != 0;

        let prefix = if is_error {
            format!("exit {exit_code}\n")
        } else {
            String::new()
        };

        Ok(ToolResult {
            tool_call_id: String::new(),
            content: format!("{prefix}{combined}"),
            is_error,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct AllowAll;
    #[async_trait::async_trait]
    impl crate::tools::ApprovalChannel for AllowAll {
        async fn request_approval(
            &self,
            _: ApprovalRequest,
        ) -> ApprovalResponse {
            ApprovalResponse::Allow
        }
    }

    #[tokio::test]
    async fn runs_simple_command() {
        let dir = std::env::temp_dir();
        let tool = BashTool::new(dir.clone());
        let ctx = ToolContext::new(dir).with_approval(std::sync::Arc::new(AllowAll));
        let r = tool
            .execute(serde_json::json!({"command": "echo hi"}), &ctx)
            .await
            .unwrap();
        assert!(!r.is_error);
        assert!(r.content.contains("hi"));
    }

    #[tokio::test]
    async fn captures_nonzero_exit() {
        let dir = std::env::temp_dir();
        let tool = BashTool::new(dir.clone());
        let ctx = ToolContext::new(dir).with_approval(std::sync::Arc::new(AllowAll));
        let r = tool
            .execute(serde_json::json!({"command": "exit 7"}), &ctx)
            .await
            .unwrap();
        assert!(r.is_error);
        assert!(r.content.contains("exit 7"));
    }

    #[tokio::test]
    async fn deny_returns_error_result() {
        struct Deny;
        #[async_trait::async_trait]
        impl crate::tools::ApprovalChannel for Deny {
            async fn request_approval(
                &self,
                _: ApprovalRequest,
            ) -> ApprovalResponse {
                ApprovalResponse::Deny
            }
        }
        let dir = std::env::temp_dir();
        let tool = BashTool::new(dir.clone());
        let ctx = ToolContext::new(dir).with_approval(std::sync::Arc::new(Deny));
        let r = tool
            .execute(serde_json::json!({"command": "echo should-not-run"}), &ctx)
            .await
            .unwrap();
        assert!(r.is_error);
        assert!(r.content.contains("denied"));
    }
}
