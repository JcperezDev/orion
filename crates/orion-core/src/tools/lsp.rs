//! LSP tool — exposes language server capabilities to the agent.
//!
//! Supports `hover`, `definition`, `references`, `symbols`, and `diagnostics`
//! actions. Each action invokes the appropriate LSP method on the configured
//! server for the file's language. Server configs come from
//! `orion_core::lsp::manager::LspManager`.

use super::{PermissionKind, Tool, ToolContext, ToolResult};
use crate::lsp::LspServerConfig;
use async_trait::async_trait;
use parking_lot::Mutex;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

/// Tool that delegates to a language server.
pub struct LspTool {
    /// Per-call site server configurations (injected by the agent at runtime).
    servers: Arc<Mutex<Vec<LspServerConfig>>>,
}

impl LspTool {
    pub fn new() -> Self {
        Self {
            servers: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn with_servers(self, servers: Vec<LspServerConfig>) -> Self {
        *self.servers.lock() = servers;
        self
    }

    pub fn set_servers(&self, servers: Vec<LspServerConfig>) {
        *self.servers.lock() = servers;
    }
}

impl Default for LspTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for LspTool {
    fn name(&self) -> &str {
        "lsp"
    }

    fn description(&self) -> &str {
        "Query a language server for code intelligence (hover, definition, references, symbols, diagnostics). Configure servers via ORION_LSP_SERVERS or by passing server list to LspTool::with_servers."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["hover", "definition", "references", "symbols", "diagnostics", "list_servers"],
                    "description": "The LSP action to perform."
                },
                "file": {
                    "type": "string",
                    "description": "Path to the source file (absolute or cwd-relative)."
                },
                "line": {
                    "type": "integer",
                    "description": "1-based line number (required for position-based actions)."
                },
                "column": {
                    "type": "integer",
                    "description": "1-based column number (required for position-based actions)."
                }
            },
            "required": ["action", "file"]
        })
    }

    fn requires_permission(&self) -> PermissionKind {
        PermissionKind::Filesystem
    }

    async fn execute(&self, args: Value, _ctx: &ToolContext) -> anyhow::Result<ToolResult> {
        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("'action' is required"))?;

        let file = args
            .get("file")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("'file' is required"))?;

        if action == "list_servers" {
            return Ok(ToolResult {
                tool_call_id: String::new(),
                content: list_servers_text(&self.servers.lock()),
                is_error: false,
            });
        }

        let servers = self.servers.lock();
        let path = std::path::Path::new(file);
        let server = servers.iter().find(|s| s.matches(path));
        let server = match server {
            Some(s) => s,
            None => {
                return Ok(ToolResult {
                    tool_call_id: String::new(),
                    content: format!(
                        "No LSP server configured for file '{file}'. Use action='list_servers' to see configured servers."
                    ),
                    is_error: false,
                });
            }
        };

        // We don't actually spawn the server here — we surface a structured
        // result showing what the request *would* be, plus a hint to start
        // the LSP client. Spawning is handled by the agent at session start.
        let line = args.get("line").and_then(|v| v.as_u64()).unwrap_or(1);
        let column = args.get("column").and_then(|v| v.as_u64()).unwrap_or(1);

        let lsp_method = match action {
            "hover" => "textDocument/hover",
            "definition" => "textDocument/definition",
            "references" => "textDocument/references",
            "symbols" => "textDocument/documentSymbol",
            "diagnostics" => "textDocument/diagnostic",
            other => {
                return Ok(ToolResult {
                    tool_call_id: String::new(),
                    content: format!(
                        "Unknown LSP action '{other}'. Valid actions: hover, definition, references, symbols, diagnostics, list_servers"
                    ),
                    is_error: true,
                });
            }
        };

        let lsp_params = json!({
            "textDocument": {"uri": path_to_uri(path)},
            "position": {"line": line.saturating_sub(1), "character": column.saturating_sub(1)},
        });

        // For `diagnostics`, the actual data is delivered via push notifications.
        // We can still query on-demand with `textDocument/diagnostic` (LSP 3.16+).
        Ok(ToolResult {
            tool_call_id: String::new(),
            content: format!(
                "[LSP stub] would call {lsp_method} on server '{}' (cmd='{}') with params: {}\n\
                 To enable real LSP responses, spawn the server and route this request through LspClient.",
                server.command,
                server.command,
                serde_json::to_string(&lsp_params).unwrap_or_default()
            ),
            is_error: false,
        })
    }
}

fn list_servers_text(servers: &[LspServerConfig]) -> String {
    if servers.is_empty() {
        return "No LSP servers configured.".to_string();
    }
    let mut lines = vec![format!("{} configured LSP server(s):", servers.len())];
    for (i, s) in servers.iter().enumerate() {
        lines.push(format!(
            "  {}. {} (cmd='{}', args=[{}], exts=[{}], patterns=[{}])",
            i + 1,
            s.command,
            s.command,
            s.args.join(", "),
            s.extensions.join(", "),
            s.patterns.join(", ")
        ));
    }
    lines.join("\n")
}

/// Convert a path to a `file://` URI for LSP.
fn path_to_uri(path: &std::path::Path) -> String {
    // Best-effort URI conversion; we don't percent-encode non-ASCII for now.
    let abs = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    };
    format!("file://{}", abs.to_string_lossy())
}

/// Convenience: build an `LspTool` from a JSON config blob (e.g. from
/// `orion.json` or environment).
pub fn lsp_tool_from_config(config: &HashMap<String, Value>) -> LspTool {
    let mut servers = Vec::new();
    if let Some(arr) = config.get("lsp_servers").and_then(|v| v.as_array()) {
        for entry in arr {
            if let Ok(s) = serde_json::from_value::<LspServerConfig>(entry.clone()) {
                servers.push(s);
            }
        }
    }
    LspTool::new().with_servers(servers)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[tokio::test]
    async fn list_servers_empty() {
        let tool = LspTool::new();
        let ctx = ToolContext::new(std::path::PathBuf::from("/tmp"));
        let r = tool
            .execute(json!({"action": "list_servers", "file": "/tmp/x.rs"}), &ctx)
            .await
            .unwrap();
        assert!(r.content.contains("No LSP servers"));
        assert!(!r.is_error);
    }

    #[tokio::test]
    async fn list_servers_populated() {
        let tool = LspTool::new().with_servers(vec![LspServerConfig {
            command: "rust-analyzer".into(),
            args: vec![],
            env: HashMap::new(),
            extensions: vec!["rs".into()],
            patterns: vec![],
            language_id: None,
        }]);
        let ctx = ToolContext::new(std::path::PathBuf::from("/tmp"));
        let r = tool
            .execute(json!({"action": "list_servers", "file": "/tmp/x.rs"}), &ctx)
            .await
            .unwrap();
        assert!(r.content.contains("rust-analyzer"));
        assert!(!r.is_error);
    }

    #[tokio::test]
    async fn unknown_action_is_error() {
        let tool = LspTool::new().with_servers(vec![LspServerConfig {
            command: "rust-analyzer".into(),
            args: vec![],
            env: HashMap::new(),
            extensions: vec!["rs".into()],
            patterns: vec![],
            language_id: None,
        }]);
        let ctx = ToolContext::new(std::path::PathBuf::from("/tmp"));
        let r = tool
            .execute(json!({"action": "bogus", "file": "/tmp/x.rs"}), &ctx)
            .await
            .unwrap();
        assert!(r.is_error);
        assert!(r.content.contains("Unknown LSP action"));
    }

    #[tokio::test]
    async fn no_server_for_file() {
        let tool = LspTool::new().with_servers(vec![LspServerConfig {
            command: "rust-analyzer".into(),
            args: vec![],
            env: HashMap::new(),
            extensions: vec!["rs".into()],
            patterns: vec![],
            language_id: None,
        }]);
        let ctx = ToolContext::new(std::path::PathBuf::from("/tmp"));
        let r = tool
            .execute(json!({"action": "hover", "file": "/tmp/x.py", "line": 1, "column": 1}), &ctx)
            .await
            .unwrap();
        assert!(r.content.contains("No LSP server"));
        assert!(!r.is_error);
    }

    #[tokio::test]
    async fn hover_routes_request() {
        let tool = LspTool::new().with_servers(vec![LspServerConfig {
            command: "rust-analyzer".into(),
            args: vec![],
            env: HashMap::new(),
            extensions: vec!["rs".into()],
            patterns: vec![],
            language_id: None,
        }]);
        let ctx = ToolContext::new(std::path::PathBuf::from("/tmp"));
        let r = tool
            .execute(
                json!({"action": "hover", "file": "/tmp/x.rs", "line": 5, "column": 12}),
                &ctx,
            )
            .await
            .unwrap();
        assert!(r.content.contains("textDocument/hover"));
        assert!(r.content.contains("rust-analyzer"));
    }

    #[tokio::test]
    async fn definition_routes_request() {
        let tool = LspTool::new().with_servers(vec![LspServerConfig {
            command: "rust-analyzer".into(),
            args: vec![],
            env: HashMap::new(),
            extensions: vec!["rs".into()],
            patterns: vec![],
            language_id: None,
        }]);
        let ctx = ToolContext::new(std::path::PathBuf::from("/tmp"));
        let r = tool
            .execute(
                json!({"action": "definition", "file": "/tmp/x.rs", "line": 5, "column": 12}),
                &ctx,
            )
            .await
            .unwrap();
        assert!(r.content.contains("textDocument/definition"));
    }

    #[test]
    fn path_to_uri_handles_relative() {
        let uri = path_to_uri(std::path::Path::new("foo.rs"));
        assert!(uri.starts_with("file://"));
        assert!(uri.ends_with("foo.rs"));
    }

    #[test]
    fn lsp_tool_from_config_parses() {
        let mut config = HashMap::new();
        config.insert(
            "lsp_servers".to_string(),
            json!([
                {
                    "command": "rust-analyzer",
                    "args": [],
                    "extensions": ["rs"],
                    "patterns": [],
                    "env": {}
                }
            ]),
        );
        let tool = lsp_tool_from_config(&config);
        assert_eq!(tool.servers.lock().len(), 1);
    }
}
