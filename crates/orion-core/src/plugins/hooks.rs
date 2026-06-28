//! Plugin hooks system — intercept and modify behavior at key extension points.
//!
//! Hooks fire at specific points during the agent loop:
//!
//! - `tool.execute.before` — modify a tool's arguments before it runs (or veto)
//! - `tool.execute.after` — modify a tool's result after it runs
//! - `chat.message` — observe/transform a chat message
//! - `chat.system` — modify the system prompt before each turn
//! - `permission.ask` — intercept a permission prompt and answer
//! - `event` — receive a generic agent event
//!
//! Plugins register hooks via the `PluginLoader`. Hooks are pure Rust functions
//! registered in code (in v1). Future versions may support hooks loaded from
//! external scripts via stdin/stdout IPC.

use crate::tools::{ToolCall, ToolResult};
use async_trait::async_trait;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// Names of the standard hooks.
pub mod hook_names {
    pub const TOOL_EXECUTE_BEFORE: &str = "tool.execute.before";
    pub const TOOL_EXECUTE_AFTER: &str = "tool.execute.after";
    pub const CHAT_MESSAGE: &str = "chat.message";
    pub const CHAT_SYSTEM: &str = "chat.system";
    pub const PERMISSION_ASK: &str = "permission.ask";
    pub const EVENT: &str = "event";
}

/// What a hook can decide:
/// - `Continue` — let the next hook / original call proceed.
/// - `Modify` — replace the value (and let the next hook / call use the new one).
/// - `Veto` — block the operation entirely.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum HookDecision<T> {
    Continue,
    Modify(T),
    Veto { reason: String },
}

impl<T> HookDecision<T> {
    pub fn is_veto(&self) -> bool {
        matches!(self, HookDecision::Veto { .. })
    }
    pub fn is_modify(&self) -> bool {
        matches!(self, HookDecision::Modify(_))
    }
}

/// Common hook inputs.
pub struct ToolBeforeHook<'a> {
    pub tool_name: &'a str,
    pub arguments: &'a mut Value,
}

pub struct ToolAfterHook<'a> {
    pub tool_name: &'a str,
    pub arguments: &'a Value,
    pub result: &'a mut ToolResult,
}

pub struct ChatMessageHook<'a> {
    pub role: &'a str,
    pub content: &'a mut String,
}

pub struct ChatSystemHook<'a> {
    pub prompt: &'a mut String,
}

pub struct PermissionAskHook<'a> {
    pub tool: &'a str,
    pub action: &'a str,
    pub allow: &'a mut bool,
}

/// Async-safe hook trait. Each hook method can return a decision.
#[async_trait]
pub trait Hook: Send + Sync {
    fn name(&self) -> &str;

    /// Default: every method is a no-op Continue. Override what you need.
    async fn tool_execute_before(
        &self,
        _ctx: &mut ToolBeforeHook<'_>,
    ) -> HookDecision<()> {
        HookDecision::Continue
    }

    async fn tool_execute_after(
        &self,
        _ctx: &mut ToolAfterHook<'_>,
    ) -> HookDecision<()> {
        HookDecision::Continue
    }

    async fn chat_message(
        &self,
        _ctx: &mut ChatMessageHook<'_>,
    ) -> HookDecision<()> {
        HookDecision::Continue
    }

    async fn chat_system(
        &self,
        _ctx: &mut ChatSystemHook<'_>,
    ) -> HookDecision<()> {
        HookDecision::Continue
    }

    async fn permission_ask(
        &self,
        _ctx: &mut PermissionAskHook<'_>,
    ) -> HookDecision<()> {
        HookDecision::Continue
    }

    async fn event(&self, _event: &Value) -> HookDecision<()> {
        HookDecision::Continue
    }
}

/// Registry of hooks. Plugins register hooks at startup; the dispatcher calls
/// `fire_*` methods during the agent loop.
#[derive(Default, Clone)]
pub struct HookRegistry {
    /// Hooks grouped by event name.
    by_event: Arc<RwLock<HashMap<String, Vec<Arc<dyn Hook>>>>>,
    /// Tool-name → hook-id allowlist (for per-tool hooks). If empty, all hooks fire.
    tool_filters: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl HookRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a hook under all of its implemented events.
    pub fn register(&self, hook: Arc<dyn Hook>) {
        let mut map = self.by_event.write();
        map.entry(hook_names::TOOL_EXECUTE_BEFORE.to_string())
            .or_default()
            .push(hook.clone());
        map.entry(hook_names::TOOL_EXECUTE_AFTER.to_string())
            .or_default()
            .push(hook.clone());
        map.entry(hook_names::CHAT_MESSAGE.to_string())
            .or_default()
            .push(hook.clone());
        map.entry(hook_names::CHAT_SYSTEM.to_string())
            .or_default()
            .push(hook.clone());
        map.entry(hook_names::PERMISSION_ASK.to_string())
            .or_default()
            .push(hook.clone());
        map.entry(hook_names::EVENT.to_string())
            .or_default()
            .push(hook);
    }

    /// Restrict a hook to only fire for specific tools (by hook name).
    /// Once any tool is filtered for this hook, the hook is "restricted"
    /// and only fires for tools in its filter list.
    pub fn filter_for_tool(&self, tool_name: &str, hook_id: &str) {
        self.tool_filters
            .write()
            .entry(tool_name.to_string())
            .or_default()
            .push(hook_id.to_string());
    }

    /// Returns the hooks that should fire for `event` + optional `tool_name`.
    /// A hook is "restricted" if it has at least one filter entry; restricted
    /// hooks only fire for tools in their filter list. Unrestricted hooks
    /// fire for all tools.
    fn hooks_for(&self, event: &str, tool_name: Option<&str>) -> Vec<Arc<dyn Hook>> {
        let map = self.by_event.read();
        let hooks = map.get(event).cloned().unwrap_or_default();
        let filters = self.tool_filters.read();

        hooks
            .into_iter()
            .filter(|h| {
                let hook_name = h.name();
                let mut is_filtered_for_any = false;
                let mut fires_for_tool = false;
                for (tool, allowed) in filters.iter() {
                    if allowed.contains(&hook_name.to_string()) {
                        is_filtered_for_any = true;
                        if Some(tool.as_str()) == tool_name {
                            fires_for_tool = true;
                        }
                    }
                }
                if is_filtered_for_any {
                    fires_for_tool
                } else {
                    true // unrestricted: always fires
                }
            })
            .collect()
    }

    /// Fire `tool.execute.before` — returns whether to veto, otherwise the
    /// arguments have been modified in place.
    pub async fn fire_tool_before(&self, tool: &str, args: &mut Value) -> (bool, Option<String>) {
        let hooks = self.hooks_for(hook_names::TOOL_EXECUTE_BEFORE, Some(tool));
        for hook in hooks {
            let mut ctx = ToolBeforeHook {
                tool_name: tool,
                arguments: args,
            };
            let decision = hook.tool_execute_before(&mut ctx).await;
            if let HookDecision::Veto { reason } = decision {
                return (false, Some(reason));
            }
        }
        (true, None)
    }

    /// Fire `tool.execute.after` — runs hooks in order. Each can modify the
    /// result.
    pub async fn fire_tool_after(&self, tool: &str, args: &Value, result: &mut ToolResult) {
        let hooks = self.hooks_for(hook_names::TOOL_EXECUTE_AFTER, Some(tool));
        for hook in hooks {
            let mut ctx = ToolAfterHook {
                tool_name: tool,
                arguments: args,
                result,
            };
            let _ = hook.tool_execute_after(&mut ctx).await;
        }
    }

    /// Fire `chat.message` — runs all hooks. They can mutate content.
    pub async fn fire_chat_message(&self, role: &str, content: &mut String) -> bool {
        let hooks = self.hooks_for(hook_names::CHAT_MESSAGE, None);
        let mut blocked = false;
        for hook in hooks {
            let mut ctx = ChatMessageHook {
                role,
                content: unsafe { &mut *(content as *mut String) },
            };
            if let HookDecision::Veto { .. } = hook.chat_message(&mut ctx).await {
                blocked = true;
            }
        }
        blocked
    }

    /// Fire `chat.system` — runs all hooks. They can mutate the system prompt.
    pub async fn fire_chat_system(&self, prompt: &mut String) {
        let hooks = self.hooks_for(hook_names::CHAT_SYSTEM, None);
        for hook in hooks {
            let mut ctx = ChatSystemHook {
                prompt: unsafe { &mut *(prompt as *mut String) },
            };
            let _ = hook.chat_system(&mut ctx).await;
        }
    }

    /// Fire `permission.ask` — a hook can decide to allow/deny automatically.
    /// If `allow` is set to `true`, the request is approved.
    pub async fn fire_permission_ask(&self, tool: &str, action: &str, allow: &mut bool) {
        let hooks = self.hooks_for(hook_names::PERMISSION_ASK, Some(tool));
        for hook in hooks {
            let mut ctx = PermissionAskHook {
                tool,
                action,
                allow,
            };
            let _ = hook.permission_ask(&mut ctx).await;
        }
    }

    /// Fire `event` — fire-and-forget. Hooks can't modify events.
    pub async fn fire_event(&self, event: &Value) {
        let hooks = self.hooks_for(hook_names::EVENT, None);
        for hook in hooks {
            let _ = hook.event(event).await;
        }
    }

    /// Number of registered hooks.
    pub fn hook_count(&self) -> usize {
        self.by_event.read().values().map(|v| v.len()).sum()
    }

    /// Names of all hooks.
    pub fn hook_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .by_event
            .read()
            .values()
            .flat_map(|v| v.iter().map(|h| h.name().to_string()))
            .collect();
        names.sort();
        names.dedup();
        names
    }
}

// We use raw pointers in some hook contexts to satisfy the borrow checker
// (the hook function takes a `&mut` to a field that the caller still has a
// reference to). This is safe because hooks run sequentially and the caller
// doesn't access the field while the hook is running.
unsafe impl Send for ToolBeforeHook<'_> {}
unsafe impl Sync for ToolBeforeHook<'_> {}
unsafe impl Send for ToolAfterHook<'_> {}
unsafe impl Sync for ToolAfterHook<'_> {}
unsafe impl Send for ChatMessageHook<'_> {}
unsafe impl Sync for ChatMessageHook<'_> {}
unsafe impl Send for ChatSystemHook<'_> {}
unsafe impl Sync for ChatSystemHook<'_> {}

/// A simple hook that always logs events. Useful for debugging.
pub struct LoggingHook {
    name: String,
}

impl LoggingHook {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

#[async_trait]
impl Hook for LoggingHook {
    fn name(&self) -> &str {
        &self.name
    }

    async fn tool_execute_before(&self, ctx: &mut ToolBeforeHook<'_>) -> HookDecision<()> {
        tracing::debug!(
            "[hook:{}] tool.execute.before: tool={}, args={}",
            self.name,
            ctx.tool_name,
            serde_json::to_string(&*ctx.arguments).unwrap_or_default()
        );
        HookDecision::Continue
    }

    async fn tool_execute_after(&self, ctx: &mut ToolAfterHook<'_>) -> HookDecision<()> {
        tracing::debug!(
            "[hook:{}] tool.execute.after: tool={}, is_error={}, content_len={}",
            self.name,
            ctx.tool_name,
            ctx.result.is_error,
            ctx.result.content.len()
        );
        HookDecision::Continue
    }

    async fn event(&self, event: &Value) -> HookDecision<()> {
        tracing::debug!("[hook:{}] event: {}", self.name, event);
        HookDecision::Continue
    }
}

/// Hook that always vetoes `rm -rf` commands (safety guard).
pub struct NoDestructiveRmHook;

#[async_trait]
impl Hook for NoDestructiveRmHook {
    fn name(&self) -> &str {
        "no-destructive-rm"
    }

    async fn tool_execute_before(
        &self,
        ctx: &mut ToolBeforeHook<'_>,
    ) -> HookDecision<()> {
        if ctx.tool_name == "bash" {
            let cmd = ctx
                .arguments
                .get("command")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let trimmed = cmd.trim();
            if trimmed.starts_with("rm ") || trimmed.starts_with("rm\t") {
                let lower = trimmed.to_lowercase();
                if lower.contains("-rf") || lower.contains("-fr") {
                    return HookDecision::Veto {
                        reason: "rm -rf blocked by no-destructive-rm hook".into(),
                    };
                }
            }
        }
        HookDecision::Continue
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct CounterHook {
        name: String,
        before_count: Arc<std::sync::atomic::AtomicUsize>,
        after_count: Arc<std::sync::atomic::AtomicUsize>,
    }

    #[async_trait]
    impl Hook for CounterHook {
        fn name(&self) -> &str {
            &self.name
        }
        async fn tool_execute_before(
            &self,
            _ctx: &mut ToolBeforeHook<'_>,
        ) -> HookDecision<()> {
            self.before_count
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            HookDecision::Continue
        }
        async fn tool_execute_after(
            &self,
            _ctx: &mut ToolAfterHook<'_>,
        ) -> HookDecision<()> {
            self.after_count
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            HookDecision::Continue
        }
    }

    #[tokio::test]
    async fn registry_runs_before_and_after() {
        let reg = HookRegistry::new();
        let before = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let after = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        reg.register(Arc::new(CounterHook {
            name: "counter".into(),
            before_count: before.clone(),
            after_count: after.clone(),
        }));
        let mut args = json!({"x": 1});
        let (ok, _) = reg.fire_tool_before("read", &mut args).await;
        assert!(ok);
        let mut result = ToolResult {
            tool_call_id: "id".into(),
            content: "ok".into(),
            is_error: false,
        };
        reg.fire_tool_after("read", &args, &mut result).await;
        assert_eq!(before.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert_eq!(after.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn veto_blocks_execution() {
        let reg = HookRegistry::new();
        reg.register(Arc::new(NoDestructiveRmHook));
        let mut args = json!({"command": "rm -rf /"});
        let (ok, reason) = reg.fire_tool_before("bash", &mut args).await;
        assert!(!ok);
        assert!(reason.is_some());
        assert!(reason.unwrap().contains("blocked"));
    }

    #[tokio::test]
    async fn safe_rm_passes() {
        let reg = HookRegistry::new();
        reg.register(Arc::new(NoDestructiveRmHook));
        let mut args = json!({"command": "rm file.txt"});
        let (ok, _) = reg.fire_tool_before("bash", &mut args).await;
        assert!(ok);
    }

    #[tokio::test]
    async fn hook_filter_restricts_tools() {
        let reg = HookRegistry::new();
        let before = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        reg.register(Arc::new(CounterHook {
            name: "filtered".into(),
            before_count: before.clone(),
            after_count: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        }));
        reg.filter_for_tool("read", "filtered");

        let mut args = json!({});
        let _ = reg.fire_tool_before("read", &mut args).await;
        assert_eq!(before.load(std::sync::atomic::Ordering::SeqCst), 1);

        let mut args = json!({});
        let _ = reg.fire_tool_before("write", &mut args).await;
        assert_eq!(before.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn hook_count_and_names() {
        let reg = HookRegistry::new();
        reg.register(Arc::new(NoDestructiveRmHook));
        reg.register(Arc::new(LoggingHook::new("log")));
        assert_eq!(reg.hook_count(), 12); // 2 hooks × 6 events
        let names = reg.hook_names();
        assert!(names.contains(&"no-destructive-rm".to_string()));
        assert!(names.contains(&"log".to_string()));
    }

    #[tokio::test]
    async fn chat_message_hook_runs() {
        let reg = HookRegistry::new();
        reg.register(Arc::new(LoggingHook::new("chat")));
        let mut content = "hello".to_string();
        let _blocked = reg.fire_chat_message("user", &mut content).await;
        // Content unchanged unless a hook modifies it.
        assert_eq!(content, "hello");
    }

    #[tokio::test]
    async fn chat_system_hook_modifies_prompt() {
        struct InjectHook;
        #[async_trait]
        impl Hook for InjectHook {
            fn name(&self) -> &str {
                "inject"
            }
            async fn chat_system(
                &self,
                ctx: &mut ChatSystemHook<'_>,
            ) -> HookDecision<()> {
                ctx.prompt.push_str("\n[EXTRA]");
                HookDecision::Modify(())
            }
        }
        let reg = HookRegistry::new();
        reg.register(Arc::new(InjectHook));
        let mut prompt = "system".to_string();
        reg.fire_chat_system(&mut prompt).await;
        assert!(prompt.contains("[EXTRA]"));
    }

    #[tokio::test]
    async fn permission_ask_can_auto_allow() {
        struct AutoAllow;
        #[async_trait]
        impl Hook for AutoAllow {
            fn name(&self) -> &str {
                "auto-allow"
            }
            async fn permission_ask(
                &self,
                ctx: &mut PermissionAskHook<'_>,
            ) -> HookDecision<()> {
                if ctx.tool == "read" {
                    *ctx.allow = true;
                    return HookDecision::Modify(());
                }
                HookDecision::Continue
            }
        }
        let reg = HookRegistry::new();
        reg.register(Arc::new(AutoAllow));
        let mut allow = false;
        reg.fire_permission_ask("read", "read foo", &mut allow).await;
        assert!(allow);
        allow = false;
        reg.fire_permission_ask("bash", "rm x", &mut allow).await;
        assert!(!allow);
    }

    #[tokio::test]
    async fn event_fires_for_subscribers() {
        struct EventCounter(Arc<std::sync::atomic::AtomicUsize>);
        #[async_trait]
        impl Hook for EventCounter {
            fn name(&self) -> &str {
                "event-counter"
            }
            async fn event(&self, _e: &Value) -> HookDecision<()> {
                self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                HookDecision::Continue
            }
        }
        let n = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let reg = HookRegistry::new();
        reg.register(Arc::new(EventCounter(n.clone())));
        reg.fire_event(&json!({"type": "x"})).await;
        reg.fire_event(&json!({"type": "y"})).await;
        assert_eq!(n.load(std::sync::atomic::Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn hook_decision_helpers() {
        let c: HookDecision<()> = HookDecision::Continue;
        assert!(!c.is_veto());
        assert!(!c.is_modify());
        let m: HookDecision<()> = HookDecision::Modify(());
        assert!(m.is_modify());
        let v: HookDecision<()> = HookDecision::Veto { reason: "no".into() };
        assert!(v.is_veto());
    }
}
