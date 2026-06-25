use crate::tools::{PermissionKind, Tool, ToolContext, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    pub content: String,
    pub status: String,
}

#[derive(Debug, Default)]
pub struct TodowriteStore {
    items: Mutex<HashMap<String, Vec<TodoItem>>>,
}

impl TodowriteStore {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn list(&self, session_id: &str) -> Vec<TodoItem> {
        self.items
            .lock()
            .get(session_id)
            .cloned()
            .unwrap_or_default()
    }

    pub fn set(&self, session_id: &str, items: Vec<TodoItem>) {
        self.items.lock().insert(session_id.to_string(), items);
    }
}

pub struct TodowriteTool {
    store: Arc<TodowriteStore>,
}

impl TodowriteTool {
    pub fn new(store: Arc<TodowriteStore>) -> Self {
        Self { store }
    }
}

impl Default for TodowriteTool {
    fn default() -> Self {
        Self::new(TodowriteStore::new())
    }
}

#[derive(Debug, Deserialize)]
struct TodoArgs {
    #[serde(default)]
    session_id: Option<String>,
    items: Vec<TodoItem>,
}

#[async_trait]
impl Tool for TodowriteTool {
    fn name(&self) -> &str {
        "todowrite"
    }
    fn description(&self) -> &str {
        "Update the todo list for a session. Each item has content (text) and status (pending|in_progress|completed)."
    }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "session_id": {"type": "string", "description": "Optional session id; defaults to \"default\"."},
                "items": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "content": {"type": "string"},
                            "status": {"type": "string", "enum": ["pending", "in_progress", "completed"]}
                        },
                        "required": ["content", "status"]
                    }
                }
            },
            "required": ["items"]
        })
    }
    fn requires_permission(&self) -> PermissionKind {
        PermissionKind::None
    }
    fn action_summary(&self, args: &serde_json::Value) -> String {
        let n = args
            .get("items")
            .and_then(|v| v.as_array())
            .map(|a| a.len())
            .unwrap_or(0);
        format!("set {n} todos")
    }

    async fn execute(
        &self,
        args: serde_json::Value,
        _ctx: &ToolContext,
    ) -> Result<ToolResult> {
        let args: TodoArgs = serde_json::from_value(args)?;
        let sid = args.session_id.unwrap_or_else(|| "default".into());
        self.store.set(&sid, args.items.clone());
        let summary: Vec<String> = args
            .items
            .iter()
            .map(|t| format!("[{}] {}", t.status, t.content))
            .collect();
        Ok(ToolResult {
            tool_call_id: String::new(),
            content: summary.join("\n"),
            is_error: false,
        })
    }
}
