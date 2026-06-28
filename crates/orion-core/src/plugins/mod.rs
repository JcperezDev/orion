pub mod hooks;
pub mod loader;

pub use hooks::{
    hook_names, ChatMessageHook, ChatSystemHook, Hook, HookDecision, HookRegistry,
    LoggingHook, NoDestructiveRmHook, PermissionAskHook, ToolAfterHook, ToolBeforeHook,
};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::tools::{PermissionKind, Tool, ToolContext, ToolResult};

/// A tool that delegates to an external process.
/// The process receives JSON-serialized arguments on stdin
/// and must return a JSON object with `content` and `is_error` on stdout.
pub struct ExternalTool {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
    pub command: String,
    pub args: Vec<String>,
    pub permission: PermissionKind,
}

impl ExternalTool {
    pub fn new(
        name: &str,
        description: &str,
        parameters: serde_json::Value,
        command: &str,
        args: Vec<String>,
        permission: Option<PermissionKind>,
    ) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            parameters,
            command: command.to_string(),
            args,
            permission: permission.unwrap_or(PermissionKind::None),
        }
    }
}

#[async_trait]
impl Tool for ExternalTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn parameters(&self) -> serde_json::Value {
        self.parameters.clone()
    }

    fn requires_permission(&self) -> PermissionKind {
        self.permission
    }

    async fn execute(
        &self,
        args: serde_json::Value,
        ctx: &ToolContext,
    ) -> anyhow::Result<ToolResult> {
        use std::process::Stdio;
        use tokio::io::AsyncWriteExt;
        use tokio::process::Command;

        let mut cmd = Command::new(&self.command);
        cmd.args(&self.args);
        cmd.current_dir(&ctx.cwd);
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd.spawn()?;
        let mut stdin = child.stdin.take().ok_or_else(|| anyhow::anyhow!("no stdin"))?;

        let input = serde_json::to_string(&args)?;
        stdin.write_all(input.as_bytes()).await?;
        stdin.write_all(b"\n").await?;
        drop(stdin);

        let output = child.wait_with_output().await?;

        let stderr_text = String::from_utf8_lossy(&output.stderr);
        if !output.status.success() {
            return Ok(ToolResult {
                tool_call_id: String::new(),
                content: format!("exit {}: {stderr_text}", output.status.code().unwrap_or(-1)),
                is_error: true,
            });
        }

        let stdout_text = String::from_utf8_lossy(&output.stdout);
        // Try parsing as JSON; fall back to plain text
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout_text) {
            let content = json
                .get("content")
                .and_then(|c| c.as_str())
                .unwrap_or(&stdout_text)
                .to_string();
            let is_error = json
                .get("is_error")
                .and_then(|e| e.as_bool())
                .unwrap_or(false);
            return Ok(ToolResult {
                tool_call_id: String::new(),
                content,
                is_error,
            });
        }

        Ok(ToolResult {
            tool_call_id: String::new(),
            content: stdout_text.to_string(),
            is_error: false,
        })
    }
}

/// A descriptor for loading plugins from config files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDescriptor {
    pub plugin: PluginDef,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDef {
    pub name: String,
    pub description: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub parameters: serde_json::Value,
    #[serde(default)]
    pub requires_permission: Option<String>,
    #[serde(default)]
    pub mcp_server: Option<McpServerDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerDef {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: std::collections::HashMap<String, String>,
}
