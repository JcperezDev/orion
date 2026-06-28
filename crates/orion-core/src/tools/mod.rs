use anyhow::Result;
use async_trait::async_trait;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PermissionKind {
    None,
    Filesystem,
    Bash,
    Network,
    Interactive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
    pub requires_permission: PermissionKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_call_id: String,
    pub content: String,
    pub is_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamChunk {
    Token { text: String },
    ToolCall { call: ToolCall },
    Done { stop_reason: Option<String> },
    Error { message: String },
}

#[derive(Debug, Clone)]
pub struct ApprovalRequest {
    pub tool_name: String,
    pub action: String,
    pub matched_pattern: Option<String>,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalResponse {
    Allow,
    AllowAlways,
    Deny,
}

#[async_trait]
pub trait ApprovalChannel: Send + Sync {
    async fn request_approval(&self, request: ApprovalRequest) -> ApprovalResponse;
}

pub struct NoopApproval;

#[async_trait]
impl ApprovalChannel for NoopApproval {
    async fn request_approval(&self, _request: ApprovalRequest) -> ApprovalResponse {
        ApprovalResponse::Deny
    }
}

pub struct ToolContext {
    pub cwd: std::path::PathBuf,
    pub approval: Mutex<Option<Arc<dyn ApprovalChannel>>>,
}

// Safety: ApprovalChannel is Send + Sync (required by trait), so wrapping
// in Mutex<Option<Arc<dyn ApprovalChannel>>> is also Send + Sync.
unsafe impl Send for ToolContext {}
unsafe impl Sync for ToolContext {}

impl ToolContext {
    pub fn new(cwd: std::path::PathBuf) -> Self {
        Self {
            cwd,
            approval: Mutex::new(None),
        }
    }

    pub fn with_approval(self, channel: Arc<dyn ApprovalChannel>) -> Self {
        *self.approval.lock() = Some(channel);
        self
    }

    pub async fn ask(&self, request: ApprovalRequest) -> ApprovalResponse {
        let channel = self
            .approval
            .lock()
            .as_ref()
            .map(|a| a.clone());
        match channel {
            Some(ch) => ch.request_approval(request).await,
            None => ApprovalResponse::Deny,
        }
    }
}

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> serde_json::Value;
    fn requires_permission(&self) -> PermissionKind;

    fn action_summary(&self, _args: &serde_json::Value) -> String {
        self.name().to_string()
    }

    async fn execute(
        &self,
        args: serde_json::Value,
        ctx: &ToolContext,
    ) -> Result<ToolResult>;
}

#[derive(Clone)]
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    pub fn list(&self) -> Vec<ToolDefinition> {
        self.tools
            .values()
            .map(|t| ToolDefinition {
                name: t.name().to_string(),
                description: t.description().to_string(),
                parameters: t.parameters(),
                requires_permission: t.requires_permission(),
            })
            .collect()
    }

    pub fn openai_tools(&self) -> Vec<serde_json::Value> {
        self.tools
            .values()
            .map(|t| {
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": t.name(),
                        "description": t.description(),
                        "parameters": t.parameters(),
                    }
                })
            })
            .collect()
    }

    pub fn anthropic_tools(&self) -> Vec<serde_json::Value> {
        self.tools
            .values()
            .map(|t| {
                serde_json::json!({
                    "name": t.name(),
                    "description": t.description(),
                    "input_schema": t.parameters(),
                })
            })
            .collect()
    }

    pub fn names(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    pub fn len(&self) -> usize {
        self.tools.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub fn builtin_registry() -> ToolRegistry {
    use std::path::PathBuf;

    let mut reg = ToolRegistry::new();
    reg.register(Arc::new(read::ReadTool));
    reg.register(Arc::new(write::WriteTool));
    reg.register(Arc::new(edit::EditTool));
    reg.register(Arc::new(bash::BashTool::new(PathBuf::from("."))));
    reg.register(Arc::new(grep::GrepTool));
    reg.register(Arc::new(glob::GlobTool));
    reg.register(Arc::new(todowrite::TodowriteTool::default()));
    reg.register(Arc::new(webfetch::WebFetchTool::new()));
    reg.register(Arc::new(websearch::WebSearchTool::new(None, None)));
    reg.register(Arc::new(question::QuestionTool));
    reg.register(Arc::new(apply_patch::ApplyPatchTool::new(PathBuf::from("."))));
    reg.register(Arc::new(lsp::LspTool::new()));
    reg.register(Arc::new(pty::PtyTool::new()));

    let mut skills_reg = crate::skills::SkillRegistry::new();
    let _ = skills_reg.auto_discover();
    reg.register(Arc::new(skill::SkillTool::new(skills_reg)));

    reg
}

/// Register a TaskTool into a ToolRegistry.
/// TaskTool requires a provider — call this after the provider is selected.
pub fn register_task_tool(
    reg: &mut ToolRegistry,
    provider: Arc<dyn crate::providers::traits::LlmProvider>,
    provider_id: &str,
    model: &str,
    permissions: Arc<crate::permissions::PermissionEngine>,
) {
    let tool = task::TaskTool::new(
        provider,
        provider_id,
        model,
        Arc::new(reg.clone()),
        permissions,
    );
    reg.register(Arc::new(tool));
}

pub mod apply_patch;
pub mod bash;
pub mod bash_parser;
pub mod skill;
pub mod task;
pub mod edit;
pub mod glob;
pub mod grep;
pub mod lsp;
pub mod pty;
pub mod question;
pub mod read;
pub mod todowrite;
pub mod websearch;
pub mod webfetch;
pub mod write;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_register_and_lookup() {
        struct Dummy;
        #[async_trait]
        impl Tool for Dummy {
            fn name(&self) -> &str {
                "dummy"
            }
            fn description(&self) -> &str {
                "d"
            }
            fn parameters(&self) -> serde_json::Value {
                serde_json::json!({"type": "object"})
            }
            fn requires_permission(&self) -> PermissionKind {
                PermissionKind::None
            }
            async fn execute(
                &self,
                _args: serde_json::Value,
                _ctx: &ToolContext,
            ) -> Result<ToolResult> {
                Ok(ToolResult {
                    tool_call_id: String::new(),
                    content: "ok".into(),
                    is_error: false,
                })
            }
        }

        let mut reg = ToolRegistry::new();
        reg.register(Arc::new(Dummy));
        assert!(reg.get("dummy").is_some());
        assert_eq!(reg.len(), 1);
        assert!(reg.list()[0].name == "dummy");
    }

    #[test]
    fn openai_tools_shape() {
        let reg = builtin_registry();
        let tools = reg.openai_tools();
        assert!(!tools.is_empty());
        let t = &tools[0];
        assert_eq!(t["type"], "function");
        assert!(t["function"]["name"].is_string());
        assert!(t["function"]["parameters"].is_object());
    }
}
