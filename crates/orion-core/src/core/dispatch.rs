use crate::agents::AgentSpec;
use crate::core::ratelimit;
use crate::core::snapshot::{SnapshotManager, StepSnapshot};
use crate::core::spill::SpillManager;
use crate::permissions::store::LearnedStore;
use crate::permissions::trust::{self, sticky_pattern_for};
use crate::permissions::{Action, PermissionEngine};
use crate::providers::traits::{ChatRequest, LlmProvider, Message, RequestedToolCall};
use crate::tools::{
    ApprovalChannel, ApprovalRequest, ApprovalResponse, StreamChunk, ToolCall, ToolContext,
    ToolRegistry,
};
use anyhow::Result;
use futures::StreamExt;
use std::collections::HashMap;
use std::sync::Arc;

const DOOM_LOOP_THRESHOLD: usize = 3;

pub struct DispatchConfig {
    pub max_steps: usize,
    pub cwd: std::path::PathBuf,
    pub approval: Arc<dyn ApprovalChannel>,
    pub plan_mode: bool,
    pub spill: Option<SpillManager>,
    /// Active agent, used by the Trust Engine to enforce per-agent tool gating.
    pub agent: Option<AgentSpec>,
    /// Persistent store for "always allow" decisions (per project).
    pub learned: Option<Arc<LearnedStore>>,
    /// Master switch: when on, the Trust Engine allows everything (no prompts).
    pub full_access: bool,
    /// How many times to retry a transient provider error before giving up.
    pub max_retries: usize,
    /// Longest we'll block waiting on a retry; a usage limit that resets later
    /// than this checkpoints (LimitReached) instead of busy-waiting.
    pub max_backoff: std::time::Duration,
}

impl DispatchConfig {
    pub fn new(cwd: std::path::PathBuf) -> Self {
        Self {
            max_steps: 25,
            cwd,
            approval: Arc::new(NoopApproval),
            plan_mode: false,
            spill: None,
            agent: None,
            learned: None,
            full_access: false,
            max_retries: 5,
            max_backoff: std::time::Duration::from_secs(30),
        }
    }

    pub fn with_retry(mut self, max_retries: usize, max_backoff: std::time::Duration) -> Self {
        self.max_retries = max_retries;
        self.max_backoff = max_backoff;
        self
    }

    pub fn with_approval(mut self, approval: Arc<dyn ApprovalChannel>) -> Self {
        self.approval = approval;
        self
    }

    pub fn with_plan_mode(mut self, plan: bool) -> Self {
        self.plan_mode = plan;
        self
    }

    pub fn with_spill(mut self, spill: SpillManager) -> Self {
        self.spill = Some(spill);
        self
    }

    pub fn with_agent(mut self, agent: AgentSpec) -> Self {
        self.agent = Some(agent);
        self
    }

    pub fn with_learned(mut self, learned: Arc<LearnedStore>) -> Self {
        self.learned = Some(learned);
        self
    }

    pub fn with_full_access(mut self, full_access: bool) -> Self {
        self.full_access = full_access;
        self
    }
}

pub struct NoopApproval;

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
    StepSnapshot(StepSnapshot),
    /// A reversible action (file edit inside the workspace) just ran — the UI
    /// can offer an "Undo" affordance. `before` carries each target's content
    /// prior to the edit (None = the file was newly created → undo deletes it).
    Undoable {
        tool_call_id: String,
        paths: Vec<std::path::PathBuf>,
        summary: String,
        before: Vec<(std::path::PathBuf, Option<String>)>,
    },
    /// A transient provider error (rate limit / overload / network) is being
    /// retried after `delay_secs`.
    Retrying { attempt: u32, delay_secs: u64, reason: String },
    /// A hard usage limit was hit. The work is checkpointed (the session holds
    /// the conversation so far); resume once the limit clears. `retry_after_secs`
    /// is the provider's reset hint when available.
    LimitReached { retry_after_secs: Option<u64>, message: String },
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

        // Plan mode: inject system prompt and restrict tools
        if self.config.plan_mode {
            let plan_prompt = "You are in plan mode. You can read files, search code, and explore the codebase, but you CANNOT make any edits, write files, execute shell commands, or apply patches. Only use read-only tools like `read`, `grep`, `glob`, `webfetch`, and `websearch`. If asked to make changes, explain what changes would be needed but do NOT attempt to execute them.";
            current_messages.insert(0, Message {
                role: "system".into(),
                content: plan_prompt.into(),
                ..Default::default()
            });
        }

        let all_tools_openai = self.registry.openai_tools();
        let all_tools_anthropic = self.registry.anthropic_tools();
        let use_anthropic = provider_id == "anthropic";

        // Doom loop tracker: (tool_name, args_signature) -> count
        let mut call_history: HashMap<(String, String), usize> = HashMap::new();
        let plan_active = self.config.plan_mode;

        for step in 0..self.config.max_steps {
            // Filter tools based on plan mode
            let destructive = ["write", "edit", "bash", "apply_patch"];
            let tool_defs_openai: Vec<serde_json::Value> = if plan_active {
                all_tools_openai.iter()
                    .filter(|t| {
                        t.pointer("/function/name")
                            .and_then(|n| n.as_str())
                            .map(|n| !destructive.contains(&n))
                            .unwrap_or(true)
                    })
                    .cloned()
                    .collect()
            } else {
                all_tools_openai.clone()
            };
            let tool_defs_anthropic: Vec<serde_json::Value> = if plan_active {
                all_tools_anthropic.iter()
                    .filter(|t| {
                        t.pointer("/name")
                            .and_then(|n| n.as_str())
                            .map(|n| !destructive.contains(&n))
                            .unwrap_or(true)
                    })
                    .cloned()
                    .collect()
            } else {
                all_tools_anthropic.clone()
            };

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

            // --- Provider call with rate-limit-aware retry + checkpoint ---
            //
            // Tokens stream live. We only retry when the failure happened
            // *before* any token was emitted this attempt (the typical place a
            // rate limit surfaces) — otherwise retrying would duplicate output.
            // On a hard usage limit we stop with a LimitReached checkpoint so
            // the work can resume once the limit clears (opencode-style).
            let supports_tools = provider.supports_tools(model);
            let mut step_text = String::new();
            let mut step_tool_calls: Vec<ToolCall> = Vec::new();
            let mut attempt: u32 = 0;

            loop {
                let req = request.clone();
                let mut emitted = 0usize;
                let mut toks_text = String::new();
                let mut calls: Vec<ToolCall> = Vec::new();
                let mut err: Option<String> = None;

                if supports_tools {
                    match provider.chat_with_tools(req, tool_defs.clone()).await {
                        Ok(mut stream) => {
                            while let Some(item) = stream.next().await {
                                match item {
                                    Ok(StreamChunk::Token { text }) => {
                                        if !text.is_empty() {
                                            emitted += 1;
                                            toks_text.push_str(&text);
                                            events.push(DispatchEvent::Token(text));
                                        }
                                    }
                                    Ok(StreamChunk::ToolCall { call }) => calls.push(call),
                                    Ok(StreamChunk::Done { .. }) => break,
                                    Ok(StreamChunk::Error { message }) => { err = Some(message); break; }
                                    Err(e) => { err = Some(e.to_string()); break; }
                                }
                            }
                        }
                        Err(e) => err = Some(e.to_string()),
                    }
                } else {
                    // Text-only provider (no tool support).
                    match provider.chat_stream(req).await {
                        Ok(mut s) => {
                            while let Some(item) = s.next().await {
                                match item {
                                    Ok(text) => {
                                        if !text.is_empty() {
                                            emitted += 1;
                                            toks_text.push_str(&text);
                                            events.push(DispatchEvent::Token(text));
                                        }
                                    }
                                    Err(e) => { err = Some(e.to_string()); break; }
                                }
                            }
                        }
                        Err(e) => err = Some(e.to_string()),
                    }
                }

                match err {
                    None => {
                        step_text = toks_text;
                        step_tool_calls = calls;
                        break;
                    }
                    Some(msg) => {
                        let class = ratelimit::classify_error(&msg);
                        let retry_after = match &class {
                            ratelimit::ErrorClass::RateLimited { retry_after } => *retry_after,
                            _ => None,
                        };
                        let delay = ratelimit::backoff_delay(
                            attempt,
                            retry_after,
                            self.config.max_backoff,
                        );
                        // A usage limit that resets later than we're willing to
                        // block for: checkpoint instead of busy-waiting.
                        let wait_too_long = retry_after
                            .map(|s| std::time::Duration::from_secs(s) > self.config.max_backoff)
                            .unwrap_or(false);
                        let can_retry = class.is_retryable()
                            && emitted == 0
                            && (attempt as usize) < self.config.max_retries
                            && !wait_too_long;

                        if can_retry {
                            events.push(DispatchEvent::Retrying {
                                attempt: attempt + 1,
                                delay_secs: delay.as_secs(),
                                reason: msg.clone(),
                            });
                            tokio::time::sleep(delay).await;
                            attempt += 1;
                            continue;
                        }

                        if class.is_limit() {
                            // Checkpoint: current_messages already holds the work
                            // up to here (the session persists it), so resuming
                            // simply re-runs from this point.
                            events.push(DispatchEvent::LimitReached {
                                retry_after_secs: retry_after,
                                message: msg,
                            });
                        } else {
                            events.push(DispatchEvent::Error(msg));
                        }
                        return Ok(events);
                    }
                }
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

            // Snapshot: capture file states before executing destructive tools.
            let snapshot_targets = SnapshotManager::extract_targets(&step_tool_calls, &self.config.cwd);
            let mut step_snap = SnapshotManager::new();
            step_snap.capture(&snapshot_targets);

            // Execute each tool and append tool messages.
            let ctx = ToolContext::new(self.config.cwd.clone()).with_approval(self.config.approval.clone());
            for call in step_tool_calls {
                events.push(DispatchEvent::ToolCall(call.clone()));

                // --- Doom loop detection ---
                let args_sig = normalize_args_for_doom(&call.arguments);
                let key = (call.name.clone(), args_sig);
                let count = call_history.entry(key).or_insert(0);
                *count += 1;
                if *count >= DOOM_LOOP_THRESHOLD {
                    let doom_request = ApprovalRequest {
                        tool_name: "doom_loop".into(),
                        action: format!(
                            "The model called `{}` with identical arguments {} times. Allow to continue?",
                            call.name, *count
                        ),
                        matched_pattern: None,
                        arguments: call.arguments.clone(),
                    };
                    match ctx.ask(doom_request).await {
                        ApprovalResponse::Deny => {
                            events.push(DispatchEvent::Done {
                                steps: step + 1,
                                final_text: format!(
                                    "Stopped because `{}` was called {} times with identical arguments.",
                                    call.name, *count
                                ),
                            });
                            return Ok(events);
                        }
                        ApprovalResponse::Allow | ApprovalResponse::AllowAlways => {
                            // Reset the counter so it doesn't keep asking every time
                            *count = 0;
                        }
                    }
                }

                let action_desc = self.action_desc_for(&call);
                let short_name = tool_name(&call.name);
                let decision = trust::decide(
                    &self.permissions,
                    self.config.agent.as_ref(),
                    self.config.full_access,
                    short_name,
                    &call,
                    &action_desc,
                    &self.config.cwd,
                );

                // Resolve the decision into "should we run this tool?".
                let mut deny_reason: Option<String> = None;
                match decision.action {
                    Action::Allow => {}
                    Action::Deny => {
                        deny_reason = Some(format!("denied by permissions: {}", decision.reason));
                    }
                    Action::Ask => {
                        let req = ApprovalRequest {
                            tool_name: call.name.clone(),
                            action: decision.reason.clone(),
                            matched_pattern: decision.matched_pattern.clone(),
                            arguments: call.arguments.clone(),
                        };
                        match ctx.ask(req).await {
                            ApprovalResponse::Deny => {
                                deny_reason = Some("denied by user".to_string());
                            }
                            ApprovalResponse::Allow => {}
                            ApprovalResponse::AllowAlways => {
                                // Persist a scoped rule so we never ask again.
                                let pattern = decision
                                    .matched_pattern
                                    .clone()
                                    .unwrap_or_else(|| sticky_pattern_for(&call, &action_desc));
                                let _ = self.permissions.add_rule(short_name, &pattern, Action::Allow);
                                if let Some(store) = &self.config.learned {
                                    let _ = store.add(&self.config.cwd, short_name, &pattern, Action::Allow);
                                }
                            }
                        }
                    }
                }

                if let Some(reason) = deny_reason {
                    current_messages.push(Message {
                        role: "tool".into(),
                        content: reason.clone(),
                        tool_call_id: Some(call.id.clone()),
                        is_error: true,
                        ..Default::default()
                    });
                    events.push(DispatchEvent::ToolResult {
                        tool_call_id: call.id,
                        content: reason,
                        is_error: true,
                    });
                    continue;
                }

                // The action was approved. If it is reversible (a file edit
                // inside the workspace) surface an Undo affordance.
                if decision.reversible {
                    let paths = SnapshotManager::extract_targets(
                        std::slice::from_ref(&call),
                        &self.config.cwd,
                    );
                    if !paths.is_empty() {
                        // `step_snap` captured the pre-edit content at step start.
                        let before = step_snap.captured(&paths);
                        events.push(DispatchEvent::Undoable {
                            tool_call_id: call.id.clone(),
                            paths,
                            summary: action_desc.clone(),
                            before,
                        });
                    }
                }

                if let Some(tool) = self.registry.get(&call.name) {
                    match tool.execute(call.arguments.clone(), &ctx).await {
                        Ok(result) => {
                            let content = if let Some(ref spill) = self.config.spill {
                                spill.spill(&result.content, &call.id)
                                    .ok()
                                    .flatten()
                                    .unwrap_or(result.content.clone())
                            } else if result.content.len() > 50_000 {
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

            // Compute and emit snapshot patches for this step.
            let patches = step_snap.diff();
            events.push(DispatchEvent::StepSnapshot(StepSnapshot {
                step,
                patches,
            }));

            // Context compaction: if messages exceed the budget, spill oldest.
            use crate::core::compactor::ContextCompactor;
            let compactor = ContextCompactor::default();
            if compactor.should_compact(&current_messages) {
                let marker = self.config.spill.as_ref()
                    .map(|_| "[previous output spilled to disk]")
                    .unwrap_or("[previous output truncated]");
                compactor.compact(&mut current_messages, marker);
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

    /// Check a bash invocation: explicit permission rules per segment, falling
    /// back to AST risk classification (see [`trust::bash_action`]).
    fn check_bash_permissions(&self, command_string: &str) -> Action {
        trust::bash_action(&self.permissions, command_string, &self.config.cwd).0
    }
}

fn tool_name(full_name: &str) -> &str {
    full_name.split('.').next().unwrap_or(full_name)
}

/// Normalize tool arguments for doom-loop comparison.
/// Strips non-deterministic fields (ids, timestamps) to avoid false positives.
fn normalize_args_for_doom(args: &serde_json::Value) -> String {
    let mut simplified = args.clone();
    if let Some(obj) = simplified.as_object_mut() {
        obj.remove("id");
        obj.remove("timeout_secs");
        obj.remove("num_results");
        obj.remove("tool_call_id");
    }
    simplified.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::permissions::{PermissionConfig, Rule};
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

    /// An approval channel that returns scripted responses and records which
    /// tools it was asked about.
    struct ScriptedApproval {
        responses: Mutex<Vec<ApprovalResponse>>,
        asked: Mutex<Vec<String>>,
    }

    #[async_trait]
    impl ApprovalChannel for ScriptedApproval {
        async fn request_approval(&self, request: ApprovalRequest) -> ApprovalResponse {
            self.asked.lock().push(request.tool_name.clone());
            let mut r = self.responses.lock();
            if r.is_empty() {
                ApprovalResponse::Deny
            } else {
                r.remove(0)
            }
        }
    }

    /// A tool whose name isn't auto-allowed, so the Trust Engine defaults to Ask.
    struct DangerTool;

    #[async_trait]
    impl Tool for DangerTool {
        fn name(&self) -> &str { "danger" }
        fn description(&self) -> &str { "a tool that needs approval" }
        fn parameters(&self) -> serde_json::Value {
            serde_json::json!({"type": "object", "properties": {}})
        }
        fn requires_permission(&self) -> crate::tools::PermissionKind {
            crate::tools::PermissionKind::Bash
        }
        async fn execute(&self, _args: serde_json::Value, _ctx: &ToolContext) -> Result<ToolResult> {
            Ok(ToolResult { tool_call_id: String::new(), content: "ran".into(), is_error: false })
        }
    }

    fn danger_call() -> ToolCall {
        ToolCall { id: "c1".into(), name: "danger".into(), arguments: serde_json::json!({}) }
    }

    /// A provider that fails `fails_left` times with `error_msg`, then streams
    /// `success`. Used to exercise the retry / LimitReached paths.
    struct FlakyProvider {
        fails_left: Mutex<usize>,
        error_msg: String,
        success: Vec<StreamChunk>,
    }

    #[async_trait]
    impl LlmProvider for FlakyProvider {
        async fn chat_stream(&self, _request: ChatRequest) -> Result<TokenStream> {
            unreachable!()
        }
        async fn chat_with_tools(
            &self,
            _request: ChatRequest,
            _tools: Vec<serde_json::Value>,
        ) -> Result<ChunkStream> {
            let fail = {
                let mut n = self.fails_left.lock();
                if *n > 0 { *n -= 1; true } else { false }
            };
            if fail {
                let msg = self.error_msg.clone();
                return Ok(Box::pin(stream::iter(vec![Ok(StreamChunk::Error { message: msg })])));
            }
            Ok(Box::pin(stream::iter(self.success.clone().into_iter().map(Ok))))
        }
        fn provider_id(&self) -> &str { "flaky" }
        fn supports_tools(&self, _model: &str) -> bool { true }
        fn base_url(&self) -> &str { "" }
        fn api_key_env(&self) -> &str { "" }
    }

    #[tokio::test]
    async fn hard_usage_limit_checkpoints_without_busy_waiting() {
        // A limit that resets far in the future must NOT be retried in-loop; it
        // emits LimitReached so the work can resume later.
        let provider = Arc::new(FlakyProvider {
            fails_left: Mutex::new(100),
            error_msg: "Usage limit reached, resets in 3600 seconds".into(),
            success: vec![],
        });
        let dispatcher = Dispatcher::new(
            Arc::new(ToolRegistry::new()),
            Arc::new(PermissionEngine::new(PermissionConfig::permissive())),
            DispatchConfig::new(std::env::temp_dir()),
        );
        let events = dispatcher
            .run(provider, "flaky", "m", vec![Message { role: "user".into(), content: "go".into(), ..Default::default() }])
            .await
            .unwrap();

        match events.last() {
            Some(DispatchEvent::LimitReached { retry_after_secs, .. }) => {
                assert_eq!(*retry_after_secs, Some(3600));
            }
            other => panic!("expected LimitReached, got {:?}", other.map(|_| "other")),
        }
        // No retries should have been attempted (wait too long).
        assert!(!events.iter().any(|e| matches!(e, DispatchEvent::Retrying { .. })));
    }

    #[tokio::test]
    async fn transient_error_retries_then_succeeds() {
        // Two connection resets, then a clean completion. With zero backoff the
        // retries are instant.
        let provider = Arc::new(FlakyProvider {
            fails_left: Mutex::new(2),
            error_msg: "connection reset by peer".into(),
            success: vec![
                StreamChunk::Token { text: "recovered".into() },
                StreamChunk::Done { stop_reason: Some("stop".into()) },
            ],
        });
        let dispatcher = Dispatcher::new(
            Arc::new(ToolRegistry::new()),
            Arc::new(PermissionEngine::new(PermissionConfig::permissive())),
            DispatchConfig::new(std::env::temp_dir())
                .with_retry(5, std::time::Duration::ZERO),
        );
        let events = dispatcher
            .run(provider, "flaky", "m", vec![Message { role: "user".into(), content: "go".into(), ..Default::default() }])
            .await
            .unwrap();

        let retries = events.iter().filter(|e| matches!(e, DispatchEvent::Retrying { .. })).count();
        assert_eq!(retries, 2, "should retry twice");
        let done = events.iter().any(|e| matches!(
            e, DispatchEvent::Done { final_text, .. } if final_text.contains("recovered")
        ));
        assert!(done, "should complete after recovering");
    }

    #[tokio::test]
    async fn ask_decision_actually_prompts_and_deny_blocks() {
        // Regression test: previously `Ask` silently executed without prompting.
        let approval = Arc::new(ScriptedApproval {
            responses: Mutex::new(vec![ApprovalResponse::Deny]),
            asked: Mutex::new(Vec::new()),
        });
        let provider = ScriptedProvider::new(vec![
            vec![
                StreamChunk::ToolCall { call: danger_call() },
                StreamChunk::Done { stop_reason: Some("tool_use".into()) },
            ],
            vec![StreamChunk::Done { stop_reason: Some("stop".into()) }],
        ]);

        let mut reg = ToolRegistry::new();
        reg.register(Arc::new(DangerTool));
        let dispatcher = Dispatcher::new(
            Arc::new(reg),
            Arc::new(PermissionEngine::new(PermissionConfig::safe_defaults())),
            DispatchConfig::new(std::env::temp_dir()).with_approval(approval.clone()),
        );

        let events = dispatcher
            .run(provider, "scripted", "m", vec![Message { role: "user".into(), content: "go".into(), ..Default::default() }])
            .await
            .unwrap();

        // The channel WAS consulted (the bug would have skipped it).
        assert_eq!(approval.asked.lock().len(), 1);
        // And the denied call produced an error tool result, not an execution.
        let denied = events.iter().any(|e| matches!(
            e,
            DispatchEvent::ToolResult { is_error: true, content, .. } if content.contains("denied")
        ));
        assert!(denied, "denied call should yield an error result");
    }

    #[tokio::test]
    async fn allow_always_persists_and_stops_asking() {
        let approval = Arc::new(ScriptedApproval {
            responses: Mutex::new(vec![ApprovalResponse::AllowAlways]),
            asked: Mutex::new(Vec::new()),
        });
        // Two steps each issuing the same danger call; the first should ask,
        // the second should be auto-allowed by the persisted rule.
        let provider = ScriptedProvider::new(vec![
            vec![
                StreamChunk::ToolCall { call: danger_call() },
                StreamChunk::Done { stop_reason: Some("tool_use".into()) },
            ],
            vec![
                StreamChunk::ToolCall { call: danger_call() },
                StreamChunk::Done { stop_reason: Some("tool_use".into()) },
            ],
            vec![StreamChunk::Done { stop_reason: Some("stop".into()) }],
        ]);

        let mut reg = ToolRegistry::new();
        reg.register(Arc::new(DangerTool));
        let engine = Arc::new(PermissionEngine::new(PermissionConfig::safe_defaults()));
        let dispatcher = Dispatcher::new(
            Arc::new(reg),
            engine.clone(),
            DispatchConfig::new(std::env::temp_dir()).with_approval(approval.clone()),
        );

        dispatcher
            .run(provider, "scripted", "m", vec![Message { role: "user".into(), content: "go".into(), ..Default::default() }])
            .await
            .unwrap();

        // Asked exactly once: the second identical call hit the learned rule.
        assert_eq!(approval.asked.lock().len(), 1, "should ask only once");
        assert_eq!(engine.check_explicit("danger", "danger"), Some(Action::Allow));
    }

    #[test]
    fn bash_permission_denies_compound_with_dangerous_command() {
        // Even if the permission engine defaults to Allow for bash, a rule
        // matching "rm *" should deny any bash invocation containing `rm`.
        let mut pcfg = PermissionConfig::permissive();
        pcfg.rules.insert(
            "bash".into(),
            vec![Rule {
                pattern: "rm *".into(),
                action: Action::Deny,
            }],
        );
        let eng = PermissionEngine::new(pcfg);
        let dispatcher = Dispatcher::new(
            Arc::new(ToolRegistry::new()),
            Arc::new(eng),
            DispatchConfig::new(std::env::temp_dir()),
        );

        // check_bash_permissions called with a compound command
        // We test the internal logic by calling it via the public path
        assert_eq!(
            dispatcher.check_bash_permissions("git status && rm -rf /"),
            Action::Deny,
            "compound with rm should be denied"
        );
        assert_eq!(
            dispatcher.check_bash_permissions("git status && echo hello"),
            Action::Allow,
            "compound without dangerous command should be allowed"
        );
        assert_eq!(
            dispatcher.check_bash_permissions("rm file.txt"),
            Action::Deny,
            "single rm command should be denied"
        );
    }
}
