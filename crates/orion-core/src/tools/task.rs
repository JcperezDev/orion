use crate::core::dispatch::{DispatchConfig, Dispatcher};
use crate::core::spill::SpillManager;
use crate::permissions::PermissionEngine;
use crate::providers::traits::{LlmProvider, Message};
use crate::tools::{PermissionKind, Tool, ToolContext, ToolResult, ToolRegistry};
use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;
use std::sync::Arc;

/// A tool that spawns a sub-agent to handle a subtask.
/// The sub-agent uses the same provider and tool registry as the parent,
/// runs for a limited number of steps, and returns the result text.
pub struct TaskTool {
    provider: Arc<dyn LlmProvider>,
    provider_id: String,
    model: String,
    registry: Arc<ToolRegistry>,
    permissions: Arc<PermissionEngine>,
    spill: Option<SpillManager>,
}

impl TaskTool {
    pub fn new(
        provider: Arc<dyn LlmProvider>,
        provider_id: &str,
        model: &str,
        registry: Arc<ToolRegistry>,
        permissions: Arc<PermissionEngine>,
    ) -> Self {
        Self {
            provider,
            provider_id: provider_id.to_string(),
            model: model.to_string(),
            registry,
            permissions,
            spill: None,
        }
    }

    pub fn with_spill(mut self, spill: SpillManager) -> Self {
        self.spill = Some(spill);
        self
    }
}

#[derive(Debug, Deserialize)]
struct TaskArgs {
    prompt: String,
    #[serde(default = "default_max_steps")]
    max_steps: usize,
}

fn default_max_steps() -> usize {
    10
}

#[async_trait]
impl Tool for TaskTool {
    fn name(&self) -> &str {
        "task"
    }

    fn description(&self) -> &str {
        "Delegate a subtask to a sub-agent. The agent will use tools to complete the task and return the result. Use for multi-step work that can be parallelised or for focused sub-problems."
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "prompt": {
                    "type": "string",
                    "description": "The instruction for the sub-agent. Be specific about what to do and what to return."
                },
                "max_steps": {
                    "type": "integer",
                    "description": "Maximum tool-use steps for the sub-agent (default 10)."
                }
            },
            "required": ["prompt"]
        })
    }

    fn requires_permission(&self) -> PermissionKind {
        PermissionKind::Interactive
    }

    async fn execute(
        &self,
        args: serde_json::Value,
        _ctx: &ToolContext,
    ) -> Result<ToolResult> {
        let task_args: TaskArgs = serde_json::from_value(args)?;

        let config = DispatchConfig {
            max_steps: task_args.max_steps,
            cwd: _ctx.cwd.clone(),
            approval: _ctx.approval.lock().clone().unwrap_or_else(|| {
                Arc::new(crate::core::dispatch::NoopApproval)
            }),
            plan_mode: false,
            spill: self.spill.clone(),
            agent: None,
            learned: None,
            full_access: false,
            max_retries: 5,
            max_backoff: std::time::Duration::from_secs(30),
        };

        let dispatcher = Dispatcher::new(
            self.registry.clone(),
            self.permissions.clone(),
            config,
        );

        let messages = vec![Message {
            role: "user".into(),
            content: task_args.prompt,
            ..Default::default()
        }];

        let events = dispatcher
            .run(self.provider.clone(), &self.provider_id, &self.model, messages)
            .await?;

        let mut text = String::new();
        let mut steps = 0;
        for event in &events {
            match event {
                crate::core::dispatch::DispatchEvent::Token(t) => text.push_str(t),
                crate::core::dispatch::DispatchEvent::ToolResult { content, .. } => {
                    if !text.is_empty() {
                        text.push_str("\n");
                    }
                    text.push_str(content);
                }
                crate::core::dispatch::DispatchEvent::Done { steps: s, .. } => {
                    steps = *s;
                }
                _ => {}
            }
        }

        let done_text = events.iter().find_map(|e| match e {
            crate::core::dispatch::DispatchEvent::Done { final_text, .. } => {
                Some(final_text.clone())
            }
            _ => None,
        }).unwrap_or_default();

        Ok(ToolResult {
            tool_call_id: String::new(),
            content: format!(
                "[task completed in {steps} steps]\n{}\n{}",
                text,
                if done_text.is_empty() || text.contains(&done_text) {
                    String::new()
                } else {
                    format!("\nFinal response:\n{done_text}")
                }
            ),
            is_error: false,
        })
    }
}
