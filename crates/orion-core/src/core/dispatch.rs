use crate::providers::traits::{ChatRequest, LlmProvider, Message, RequestedToolCall};
use crate::tools::{
    ApprovalChannel, ApprovalRequest, ApprovalResponse, StreamChunk, Tool, ToolCall, ToolContext,
    ToolRegistry,
};
use crate::permissions::{Action, PermissionEngine};
use anyhow::Result;
use futures::StreamExt;
use std::sync::Arc;

pub struct DispatchConfig {
    pub max_steps: usize,
    pub cwd: std::path::PathBuf,
    pub approval: Arc<dyn ApprovalChannel>,
}

impl DispatchConfig {
    pub fn new(cwd: std::path::PathBuf) -> Self {
        Self {
            max_steps: 25,
            cwd,
            approval: Arc::new(NoopApproval),
        }
    }

    pub fn with_approval(mut self, approval: Arc<dyn ApprovalChannel>) -> Self {
        self.approval = approval;
        self
    }
}

struct NoopApproval;

#[async_trait::async_trait]
impl ApprovalChannel for NoopApproval {
    async fn request_approval(&self, _request: ApprovalRequest) -> ApprovalResponse {
        ApprovalResponse::Allow
    }
}

pub enum DispatchEvent {
    Token(String),
    ToolCall(ToolCall),
    ToolResult { tool_call_id: String, content: String, is_error: bool },
    Done { steps: usize, final_text: String },
    Error(String),
}

pub struct Dispatcher {
    pub registry: Arc<ToolRegistry>,
    pub permissions: Arc<PermissionEngine>,
    pub config: DispatchConfig,
}

impl Dispatcher {
    pub fn new(
        registry: Arc<ToolRegistry>,
        permissions: Arc<PermissionEngine>,
        config: DispatchConfig,
    ) -> Self {
        Self { registry, permissions, config }
    }

    pub async fn run(
        &self,
        provider: Arc<dyn LlmProvider>,
        provider_id: &str,
        model: &str,
        messages: Vec<Message>,
    ) -> Result<Vec<DispatchEvent>> {
        let mut events: Vec<DispatchEvent> = Vec::new();
        let mut current_messages = messages;
        let tool_defs_openai = self.registry.openai_tools();
        let tool_defs_anthropic = self.registry.anthropic_tools();
        let use_anthropic = provider_id == "anthropic";

        for step in 0..self.config.max_steps {
            let request = ChatRequest {
                model: model.to_string(),
                messages: current_messages.clone(),
                temperature: None,
                max_tokens: None,
                tools: None,
            };
            let tool_defs = if use_anthropic {
                tool_defs_anthropic.clone()
            } else {
                tool_defs_openai.clone()
            };

            let mut stream = if !provider.supports_tools(model) {
                // Provider has no tool support; fall back to text-only path.
                let mut s = provider.chat_stream(request).await?;
                let mut out: Vec<DispatchEvent> = Vec::new();
                while let Some(item) = s.next().await {
                    match item {
                        Ok(text) => {
                            if !text.is_empty() {
                                out.push(DispatchEvent::Token(text));
                            }
                        }
                        Err(e) => {
                            out.push(DispatchEvent::Error(e.to_string()));
                            return Ok(out);
                        }
                    }
                }
                out.push(DispatchEvent::Done {
                    steps: step,
                    final_text: String::new(),
                });
                events.extend(out);
                return Ok(events);
            } else {
                // Pass tools (possibly empty) — provider can still stream text-only completion.
                provider.chat_with_tools(request, tool_defs).await?
            };

            let mut step_text = String::new();
            let mut step_tool_calls: Vec<ToolCall> = Vec::new();
            let mut step_error: Option<String> = None;

            while let Some(item) = stream.next().await {
                match item {
                    Ok(StreamChunk::Token { text }) => {
                        if !text.is_empty() {
                            step_text.push_str(&text);
                            events.push(DispatchEvent::Token(text));
                        }
                    }
                    Ok(StreamChunk::ToolCall { call }) => {
                        step_tool_calls.push(call);
                    }
                    Ok(StreamChunk::Done { .. }) => break,
                    Ok(StreamChunk::Error { message }) => {
                        step_error = Some(message);
                        break;
                    }
                    Err(e) => {
                        step_error = Some(e.to_string());
                        break;
                    }
                }
            }

            if let Some(msg) = step_error {
                events.push(DispatchEvent::Error(msg));
                return Ok(events);
            }

            if step_tool_calls.is_empty() {
                events.push(DispatchEvent::Done {
                    steps: step + 1,
                    final_text: step_text,
                });
                return Ok(events);
            }

            // Append the assistant message (with tool_calls) to the running history.
            current_messages.push(Message {
                role: "assistant".into(),
                content: step_text,
                tool_calls: Some(
                    step_tool_calls
                        .iter()
                        .map(|c| RequestedToolCall {
                            id: c.id.clone(),
                            name: c.name.clone(),
                            arguments: c.arguments.clone(),
                        })
                        .collect(),
                ),
                ..Default::default()
            });

            // Execute each tool and append tool messages.
            let ctx = ToolContext::new(self.config.cwd.clone()).with_approval(self.config.approval.clone());
            for call in step_tool_calls {
                events.push(DispatchEvent::ToolCall(call.clone()));

                let decision = self
                    .permissions
                    .check(call.name.split('.').next().unwrap_or(call.name.as_str()), &self.action_desc_for(&call));
                match decision {
                    Action::Deny => {
                        let content = "denied by permissions".to_string();
                        current_messages.push(Message {
                            role: "tool".into(),
                            content: content.clone(),
                            tool_call_id: Some(call.id.clone()),
                            is_error: true,
                            ..Default::default()
                        });
                        events.push(DispatchEvent::ToolResult {
                            tool_call_id: call.id,
                            content,
                            is_error: true,
                        });
                        continue;
                    }
                    Action::Allow | Action::Ask => {}
                }

                if let Some(tool) = self.registry.get(&call.name) {
                    match tool.execute(call.arguments.clone(), &ctx).await {
                        Ok(result) => {
                            let content = if result.content.len() > 50_000 {
                                format!(
                                    "{}...\n[truncated]",
                                    &result.content[..50_000]
                                )
                            } else {
                                result.content.clone()
                            };
                            current_messages.push(Message {
                                role: "tool".into(),
                                content: content.clone(),
                                tool_call_id: Some(call.id.clone()),
                                is_error: result.is_error,
                                ..Default::default()
                            });
                            events.push(DispatchEvent::ToolResult {
                                tool_call_id: call.id,
                                content,
                                is_error: result.is_error,
                            });
                        }
                        Err(e) => {
                            let content = format!("error: {e}");
                            current_messages.push(Message {
                                role: "tool".into(),
                                content: content.clone(),
                                tool_call_id: Some(call.id.clone()),
                                is_error: true,
                                ..Default::default()
                            });
                            events.push(DispatchEvent::ToolResult {
                                tool_call_id: call.id,
                                content,
                                is_error: true,
                            });
                        }
                    }
                } else {
                    let content = format!("unknown tool: {}", call.name);
                    current_messages.push(Message {
                        role: "tool".into(),
                        content: content.clone(),
                        tool_call_id: Some(call.id.clone()),
                        is_error: true,
                        ..Default::default()
                    });
                    events.push(DispatchEvent::ToolResult {
                        tool_call_id: call.id,
                        content,
                        is_error: true,
                    });
                }
            }
        }

        events.push(DispatchEvent::Error(format!(
            "agent exceeded max_steps={}",
            self.config.max_steps
        )));
        Ok(events)
    }

    fn action_desc_for(&self, call: &ToolCall) -> String {
        if let Some(tool) = self.registry.get(&call.name) {
            tool.action_summary(&call.arguments)
        } else {
            call.name.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::permissions::PermissionConfig;
    use crate::tools::{Tool, ToolContext, ToolResult};
    use async_trait::async_trait;

    /// A minimal tool that returns its arguments verbatim. Used to assert the
    /// dispatch loop delivers tool calls and feeds results back.
    struct EchoTool;

    #[async_trait]
    impl Tool for EchoTool {
        fn name(&self) -> &str { "echo" }
        fn description(&self) -> &str { "echo arguments" }
        fn parameters(&self) -> serde_json::Value {
            serde_json::json!({
                "type": "object",
                "properties": {"text": {"type": "string"}},
                "required": ["text"]
            })
        }
        fn requires_permission(&self) -> crate::tools::PermissionKind {
            crate::tools::PermissionKind::None
        }
        async fn execute(
            &self,
            args: serde_json::Value,
            _ctx: &ToolContext,
        ) -> Result<ToolResult> {
            Ok(ToolResult {
                tool_call_id: String::new(),
                content: args.to_string(),
                is_error: false,
            })
        }
    }

    /// A canned LLM provider that returns one assistant message containing a
    /// tool call, then a follow-up text-only message.
    use crate::providers::traits::{ChunkStream, TokenStream};
    use futures::stream;
    use parking_lot::Mutex;
    use std::pin::Pin;

    struct ScriptedProvider {
        scripts: Mutex<Vec<Vec<StreamChunk>>>,
    }

    impl ScriptedProvider {
        fn new(scripts: Vec<Vec<StreamChunk>>) -> Arc<Self> {
            Arc::new(Self { scripts: Mutex::new(scripts) })
        }
    }

    #[async_trait]
    impl LlmProvider for ScriptedProvider {
        async fn chat_stream(&self, _request: ChatRequest) -> Result<TokenStream> {
            unreachable!()
        }
        async fn chat_with_tools(
            &self,
            _request: ChatRequest,
            _tools: Vec<serde_json::Value>,
        ) -> Result<ChunkStream> {
            let next = {
                let mut s = self.scripts.lock();
                if s.is_empty() {
                    return Ok(Box::pin(stream::empty()));
                }
                s.remove(0)
            };
            Ok(Box::pin(stream::iter(next.into_iter().map(Ok))))
        }
        fn provider_id(&self) -> &str { "scripted" }
        fn supports_tools(&self, _model: &str) -> bool { true }
        fn base_url(&self) -> &str { "" }
        fn api_key_env(&self) -> &str { "" }
    }

    #[tokio::test]
    async fn dispatches_tool_call_then_completes() {
        // Provider script: one tool call, then text-only completion.
        let call = ToolCall {
            id: "c1".into(),
            name: "echo".into(),
            arguments: serde_json::json!({"text": "hello"}),
        };
        let provider = ScriptedProvider::new(vec![
            vec![
                StreamChunk::ToolCall { call: call.clone() },
                StreamChunk::Done { stop_reason: Some("tool_use".into()) },
            ],
            vec![
                StreamChunk::Token { text: "all done".into() },
                StreamChunk::Done { stop_reason: Some("stop".into()) },
            ],
        ]);

        let mut reg = ToolRegistry::new();
        reg.register(Arc::new(EchoTool));
        let dispatcher = Dispatcher::new(
            Arc::new(reg),
            Arc::new(PermissionEngine::new(PermissionConfig::permissive())),
            DispatchConfig::new(std::env::temp_dir()),
        );

        let events = dispatcher
            .run(
                provider,
                "scripted",
                "test-model",
                vec![Message {
                    role: "user".into(),
                    content: "go".into(),
                    ..Default::default()
                }],
            )
            .await
            .unwrap();

        let tool_results: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, DispatchEvent::ToolResult { .. }))
            .collect();
        assert_eq!(tool_results.len(), 1);
        match &tool_results[0] {
            DispatchEvent::ToolResult { content, is_error, .. } => {
                assert!(!is_error);
                assert!(content.contains("hello"));
            }
            _ => unreachable!(),
        }
        assert!(matches!(events.last(), Some(DispatchEvent::Done { .. })));
    }

    #[tokio::test]
    async fn unknown_tool_returns_error_result() {
        let call = ToolCall {
            id: "c1".into(),
            name: "no_such_tool".into(),
            arguments: serde_json::json!({}),
        };
        let provider = ScriptedProvider::new(vec![
            vec![
                StreamChunk::ToolCall { call: call.clone() },
                StreamChunk::Done { stop_reason: Some("tool_use".into()) },
            ],
            vec![StreamChunk::Done { stop_reason: Some("stop".into()) }],
        ]);

        let dispatcher = Dispatcher::new(
            Arc::new(ToolRegistry::new()),
            Arc::new(PermissionEngine::new(PermissionConfig::permissive())),
            DispatchConfig::new(std::env::temp_dir()),
        );

        let events = dispatcher
            .run(
                provider,
                "scripted",
                "test-model",
                vec![Message {
                    role: "user".into(),
                    content: "go".into(),
                    ..Default::default()
                }],
            )
            .await
            .unwrap();

        let r = events
            .iter()
            .find_map(|e| match e {
                DispatchEvent::ToolResult { is_error, content, .. } => {
                    Some((*is_error, content.clone()))
                }
                _ => None,
            })
            .expect("should produce a tool result");
        assert!(r.0, "tool result should be an error");
        assert!(r.1.contains("unknown tool"), "got: {}", r.1);
    }
}
