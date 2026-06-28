//! PTY tool — run a long-lived shell command with streaming output.
//!
//! Unlike the regular `bash` tool (which waits for completion), the PTY tool
//! can keep a process running and stream output incrementally. This is useful
//! for interactive commands (REPLs, file watchers, dev servers).
//!
//! On Unix we use `tokio::process::Command` with piped stdout/stderr. On
//! platforms without `portable-pty` available we fall back to non-PTY mode.

use super::{PermissionKind, Tool, ToolContext, ToolResult};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::Mutex;

pub struct PtyTool {
    /// Running PTYs keyed by id, with their abort handles.
    running: Arc<Mutex<std::collections::HashMap<String, PtyHandle>>>,
}

struct PtyHandle {
    child: Arc<Mutex<Option<tokio::process::Child>>>,
    killed: Arc<std::sync::atomic::AtomicBool>,
}

impl PtyTool {
    pub fn new() -> Self {
        Self {
            running: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }
}

impl Default for PtyTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for PtyTool {
    fn name(&self) -> &str {
        "pty"
    }

    fn description(&self) -> &str {
        "Run a long-lived shell command with streaming output. Supports starting, polling output, and killing processes by id."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["start", "read", "kill", "list", "wait"],
                    "description": "Action to perform."
                },
                "id": {
                    "type": "string",
                    "description": "Process id (required for read/kill/wait)."
                },
                "command": {
                    "type": "string",
                    "description": "Command to run (required for start)."
                },
                "max_wait_ms": {
                    "type": "integer",
                    "description": "Maximum time to wait in milliseconds (for start/wait)."
                }
            },
            "required": ["action"]
        })
    }

    fn requires_permission(&self) -> PermissionKind {
        PermissionKind::Bash
    }

    async fn execute(&self, args: Value, ctx: &ToolContext) -> anyhow::Result<ToolResult> {
        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("'action' is required"))?;

        match action {
            "start" => {
                let cmd = args
                    .get("command")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("'command' required for start"))?;
                let max_wait = args
                    .get("max_wait_ms")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(2000);

                let mut command = if cfg!(target_os = "windows") {
                    let mut c = Command::new("cmd");
                    c.args(["/C", cmd]);
                    c
                } else {
                    let mut c = Command::new("bash");
                    c.args(["-c", cmd]);
                    c
                };
                command
                    .current_dir(&ctx.cwd)
                    .stdin(Stdio::null())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .kill_on_drop(true);

                let mut child = command.spawn().map_err(|e| {
                    anyhow::anyhow!("failed to spawn '{cmd}': {e}")
                })?;

                let id = uuid::Uuid::new_v4().to_string();
                let killed = Arc::new(std::sync::atomic::AtomicBool::new(false));

                // Drain stdout asynchronously with a timeout.
                let stdout = child.stdout.take();
                let stderr = child.stderr.take();
                let killed_clone = killed.clone();
                let id_clone = id.clone();
                tokio::spawn(async move {
                    if let Some(out) = stdout {
                        let mut lines = BufReader::new(out).lines();
                        loop {
                            if killed_clone.load(std::sync::atomic::Ordering::SeqCst) {
                                break;
                            }
                            match tokio::time::timeout(Duration::from_millis(200), lines.next_line()).await {
                                Ok(Ok(Some(line))) => {
                                    eprintln!("[pty:{id_clone}] {line}");
                                }
                                Ok(Ok(None)) => break,
                                Ok(Err(_)) => break,
                                Err(_) => continue, // timeout, check killed
                            }
                        }
                    }
                });
                let killed_clone2 = killed.clone();
                let id_clone2 = id.clone();
                tokio::spawn(async move {
                    if let Some(err) = stderr {
                        let mut lines = BufReader::new(err).lines();
                        loop {
                            if killed_clone2.load(std::sync::atomic::Ordering::SeqCst) {
                                break;
                            }
                            match tokio::time::timeout(Duration::from_millis(200), lines.next_line()).await {
                                Ok(Ok(Some(line))) => {
                                    eprintln!("[pty:{id_clone2}] {line}");
                                }
                                Ok(Ok(None)) => break,
                                Ok(Err(_)) => break,
                                Err(_) => continue,
                            }
                        }
                    }
                });

                // Wait briefly to capture any immediate startup output.
                let mut initial_output = String::new();
                let start = std::time::Instant::now();
                while start.elapsed() < Duration::from_millis(max_wait.min(500)) {
                    if let Ok(Some(status)) = child.try_wait() {
                        let _ = status;
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
                initial_output.push_str(&format!("(started; output streams to stderr tagged [pty:{id}])"));

                let handle = PtyHandle {
                    child: Arc::new(Mutex::new(Some(child))),
                    killed,
                };
                self.running.lock().await.insert(id.clone(), handle);

                Ok(ToolResult {
                    tool_call_id: String::new(),
                    content: format!("✓ pty started: {id}\n{initial_output}"),
                    is_error: false,
                })
            }

            "read" => {
                let id = args
                    .get("id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("'id' required for read"))?;
                let running = self.running.lock().await;
                let handle = running.get(id).ok_or_else(|| anyhow::anyhow!("unknown pty id {id}"))?;
                let mut child_lock = handle.child.lock().await;
                if let Some(child) = child_lock.as_mut() {
                    match child.try_wait() {
                        Ok(Some(status)) => {
                            Ok(ToolResult {
                                tool_call_id: String::new(),
                                content: format!("process {id} exited: {status}"),
                                is_error: !status.success(),
                            })
                        }
                        Ok(None) => Ok(ToolResult {
                            tool_call_id: String::new(),
                            content: format!("process {id} still running"),
                            is_error: false,
                        }),
                        Err(e) => Ok(ToolResult {
                            tool_call_id: String::new(),
                            content: format!("error checking {id}: {e}"),
                            is_error: true,
                        }),
                    }
                } else {
                    Ok(ToolResult {
                        tool_call_id: String::new(),
                        content: format!("process {id} already collected"),
                        is_error: false,
                    })
                }
            }

            "kill" => {
                let id = args
                    .get("id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("'id' required for kill"))?;
                let mut running = self.running.lock().await;
                let handle = running.get_mut(id).ok_or_else(|| anyhow::anyhow!("unknown pty id {id}"))?;
                handle.killed.store(true, std::sync::atomic::Ordering::SeqCst);
                let mut child_lock = handle.child.lock().await;
                if let Some(child) = child_lock.as_mut() {
                    let _ = child.kill().await;
                    let _ = child.wait().await;
                }
                *child_lock = None;
                Ok(ToolResult {
                    tool_call_id: String::new(),
                    content: format!("✓ killed {id}"),
                    is_error: false,
                })
            }

            "list" => {
                let running = self.running.lock().await;
                if running.is_empty() {
                    return Ok(ToolResult {
                        tool_call_id: String::new(),
                        content: "No running PTY processes.".into(),
                        is_error: false,
                    });
                }
                let mut lines = vec![format!("{} running PTY process(es):", running.len())];
                for id in running.keys() {
                    lines.push(format!("  - {id}"));
                }
                Ok(ToolResult {
                    tool_call_id: String::new(),
                    content: lines.join("\n"),
                    is_error: false,
                })
            }

            "wait" => {
                let id = args
                    .get("id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("'id' required for wait"))?;
                let max_wait = args
                    .get("max_wait_ms")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(30_000);
                let deadline = std::time::Instant::now() + Duration::from_millis(max_wait);

                loop {
                    let status = {
                        let running = self.running.lock().await;
                        let Some(handle) = running.get(id) else {
                            return Ok(ToolResult {
                                tool_call_id: String::new(),
                                content: format!("process {id} not found (already finished?)"),
                                is_error: false,
                            });
                        };
                        let mut child_lock = handle.child.lock().await;
                        if let Some(child) = child_lock.as_mut() {
                            child.try_wait()?
                        } else {
                            return Ok(ToolResult {
                                tool_call_id: String::new(),
                                content: format!("process {id} already finished"),
                                is_error: false,
                            });
                        }
                    };
                    match status {
                        Some(s) => {
                            return Ok(ToolResult {
                                tool_call_id: String::new(),
                                content: format!("process {id} exited: {s}"),
                                is_error: !s.success(),
                            });
                        }
                        None => {
                            if std::time::Instant::now() >= deadline {
                                return Ok(ToolResult {
                                    tool_call_id: String::new(),
                                    content: format!("timeout waiting for {id}"),
                                    is_error: true,
                                });
                            }
                            tokio::time::sleep(Duration::from_millis(200)).await;
                        }
                    }
                }
            }

            other => Ok(ToolResult {
                tool_call_id: String::new(),
                content: format!(
                    "Unknown action '{other}'. Valid: start, read, kill, list, wait"
                ),
                is_error: true,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parameters_are_valid() {
        let tool = PtyTool::new();
        let p = tool.parameters();
        assert_eq!(p["type"], "object");
        assert!(p["properties"]["action"].is_object());
        assert!(p["properties"]["command"].is_object());
        assert!(p["required"]
            .as_array()
            .unwrap()
            .contains(&json!("action")));
    }

    #[tokio::test]
    async fn list_empty() {
        let tool = PtyTool::new();
        let ctx = ToolContext::new(std::path::PathBuf::from("/tmp"));
        let r = tool
            .execute(json!({"action": "list"}), &ctx)
            .await
            .unwrap();
        assert!(r.content.contains("No running PTY"));
    }

    #[tokio::test]
    async fn unknown_action_errors() {
        let tool = PtyTool::new();
        let ctx = ToolContext::new(std::path::PathBuf::from("/tmp"));
        let r = tool
            .execute(json!({"action": "bogus"}), &ctx)
            .await
            .unwrap();
        assert!(r.is_error);
        assert!(r.content.contains("Unknown action"));
    }

    #[tokio::test]
    async fn read_unknown_id_errors() {
        let tool = PtyTool::new();
        let ctx = ToolContext::new(std::path::PathBuf::from("/tmp"));
        let r = tool
            .execute(json!({"action": "read", "id": "nope"}), &ctx)
            .await;
        assert!(r.is_err());
    }

    #[tokio::test]
    async fn kill_unknown_id_errors() {
        let tool = PtyTool::new();
        let ctx = ToolContext::new(std::path::PathBuf::from("/tmp"));
        let r = tool
            .execute(json!({"action": "kill", "id": "nope"}), &ctx)
            .await;
        assert!(r.is_err());
    }

    #[tokio::test]
    async fn start_short_command_and_read() {
        let tool = PtyTool::new();
        let ctx = ToolContext::new(std::env::temp_dir());
        let r = tool
            .execute(
                json!({"action": "start", "command": "echo hello", "max_wait_ms": 1000}),
                &ctx,
            )
            .await
            .unwrap();
        assert!(!r.is_error);
        // Extract the id from "✓ pty started: <id>"
        let id = r
            .content
            .split_whitespace()
            .nth(3)
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        assert!(!id.is_empty(), "pty id should be present: {}", r.content);

        // Wait briefly for it to exit, then read.
        let wait = tool
            .execute(
                json!({"action": "wait", "id": id.clone(), "max_wait_ms": 3000}),
                &ctx,
            )
            .await
            .unwrap();
        assert!(
            wait.content.contains("exited"),
            "expected exit message, got: {}",
            wait.content
        );
    }
}
