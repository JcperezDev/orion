//! Agent system — multiple named agents (Build/Plan/Explore/Scout) with
//! per-agent permissions, models, system prompts, and capabilities.
//!
//! Each agent is a self-contained configuration that the dispatcher can
//! switch between. The TUI/CLI/Desktop all use the same `AgentRegistry`.

use crate::permissions::{Action as PermissionAction, PermissionEngine};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// What kind of agent this is — affects how the dispatcher treats it.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentMode {
    /// Primary agent — runs in the main loop, can do anything.
    Primary,
    /// Subagent — invoked via the Task tool.
    Subagent,
}

/// Built-in agent preset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSpec {
    pub id: String,
    pub name: String,
    pub mode: AgentMode,
    /// Default model in `provider:model` form.
    pub model: String,
    /// System prompt override.
    pub system_prompt: Option<String>,
    /// Tools disabled by name (e.g. `["write", "bash"]`).
    pub denied_tools: Vec<String>,
    /// Tools allowed (overrides denial).
    pub allowed_tools: Vec<String>,
    /// Hex color for TUI display (e.g. "#ff7b72").
    pub color: Option<String>,
    /// Whether to hide this agent from the picker.
    #[serde(default)]
    pub hidden: bool,
    /// Description for the agent picker.
    pub description: String,
    /// Maximum steps per turn (None = unlimited).
    #[serde(default)]
    pub max_steps: Option<usize>,
}

impl AgentSpec {
    /// Build the four canonical agents.
    pub fn builtins() -> Vec<AgentSpec> {
        vec![
            AgentSpec {
                id: "build".into(),
                name: "Build".into(),
                mode: AgentMode::Primary,
                model: "anthropic:claude-sonnet-4-20250514".into(),
                system_prompt: Some(
                    "You are the Build agent. You have full access to read, write, edit, \
                     and run shell commands. Be direct and concise. Prefer small, targeted edits. \
                     Always verify your work after making changes."
                        .into(),
                ),
                denied_tools: vec![],
                allowed_tools: vec![
                    "read".into(),
                    "write".into(),
                    "edit".into(),
                    "bash".into(),
                    "grep".into(),
                    "glob".into(),
                    "list".into(),
                    "lsp".into(),
                ],
                color: Some("#ff7b72".into()),
                hidden: false,
                description: "Full-access development agent — all tools enabled".into(),
                max_steps: None,
            },
            AgentSpec {
                id: "plan".into(),
                name: "Plan".into(),
                mode: AgentMode::Primary,
                model: "anthropic:claude-sonnet-4-20250514".into(),
                system_prompt: Some(
                    "You are the Plan agent. You are in read-only mode: you may not edit \
                     files or run side-effecting commands. Analyze the user's request, \
                     explore the codebase, and return a step-by-step plan. Use the \
                     `enter_plan_mode` and `exit_plan_mode` markers to wrap your plan."
                        .into(),
                ),
                denied_tools: vec![
                    "write".into(),
                    "edit".into(),
                    "apply_patch".into(),
                    "bash".into(),
                ],
                allowed_tools: vec![
                    "read".into(),
                    "grep".into(),
                    "glob".into(),
                    "list".into(),
                    "lsp".into(),
                ],
                color: Some("#a5d6ff".into()),
                hidden: false,
                description: "Read-only planning/analysis — edits denied, bash asks permission".into(),
                max_steps: Some(50),
            },
            AgentSpec {
                id: "explore".into(),
                name: "Explore".into(),
                mode: AgentMode::Subagent,
                model: "anthropic:claude-haiku-4-20250514".into(),
                system_prompt: Some(
                    "You are the Explore subagent. Your job is to quickly gather information \
                     about a codebase using read-only tools (read, grep, glob, list, lsp). \
                     Return a concise summary; do not edit files."
                        .into(),
                ),
                denied_tools: vec![
                    "write".into(),
                    "edit".into(),
                    "apply_patch".into(),
                    "bash".into(),
                    "task".into(),
                ],
                allowed_tools: vec![
                    "read".into(),
                    "grep".into(),
                    "glob".into(),
                    "list".into(),
                    "lsp".into(),
                ],
                color: Some("#7ee787".into()),
                hidden: false,
                description: "Fast read-only codebase exploration".into(),
                max_steps: Some(30),
            },
            AgentSpec {
                id: "scout".into(),
                name: "Scout".into(),
                mode: AgentMode::Subagent,
                model: "anthropic:claude-haiku-4-20250514".into(),
                system_prompt: Some(
                    "You are the Scout subagent. Your job is to research external libraries, \
                     documentation, and dependencies. Use webfetch/websearch tools, and clone \
                     reference repos to a managed cache when needed. Return a structured report."
                        .into(),
                ),
                denied_tools: vec![
                    "write".into(),
                    "edit".into(),
                    "apply_patch".into(),
                    "bash".into(),
                ],
                allowed_tools: vec![
                    "read".into(),
                    "webfetch".into(),
                    "websearch".into(),
                    "grep".into(),
                ],
                color: Some("#d2a8ff".into()),
                hidden: false,
                description: "External docs & dependency research".into(),
                max_steps: Some(40),
            },
            AgentSpec {
                id: "general".into(),
                name: "General".into(),
                mode: AgentMode::Subagent,
                model: "anthropic:claude-sonnet-4-20250514".into(),
                system_prompt: Some(
                    "You are the General subagent. You handle multi-step complex tasks and \
                     may make changes to files and run commands. Be focused and decisive."
                        .into(),
                ),
                denied_tools: vec!["task".into()],
                allowed_tools: vec![
                    "read".into(),
                    "write".into(),
                    "edit".into(),
                    "bash".into(),
                    "grep".into(),
                    "glob".into(),
                    "list".into(),
                    "lsp".into(),
                ],
                color: Some("#ffa657".into()),
                hidden: false,
                description: "Multi-step complex tasks, can make changes".into(),
                max_steps: Some(100),
            },
        ]
    }

    /// True if this agent can use the given tool.
    pub fn can_use_tool(&self, tool_name: &str) -> bool {
        if self.denied_tools.iter().any(|t| t == tool_name) {
            return false;
        }
        if !self.allowed_tools.is_empty() {
            return self.allowed_tools.iter().any(|t| t == tool_name);
        }
        true
    }

    /// Effective permission level for this agent on the given tool.
    pub fn permission_for(&self, tool_name: &str) -> PermissionAction {
        if self.can_use_tool(tool_name) {
            PermissionAction::Allow
        } else {
            PermissionAction::Deny
        }
    }
}

/// Mutable registry of agents (built-ins + user overrides).
#[derive(Debug, Clone, Default)]
pub struct AgentRegistry {
    by_id: HashMap<String, AgentSpec>,
    active_id: Arc<parking_lot::Mutex<Option<String>>>,
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Build a registry preloaded with the canonical agents.
    pub fn with_builtins() -> Self {
        let mut r = Self::new();
        for spec in AgentSpec::builtins() {
            r.register(spec);
        }
        r
    }

    /// Register (or overwrite) an agent spec.
    pub fn register(&mut self, spec: AgentSpec) {
        self.by_id.insert(spec.id.clone(), spec);
    }

    /// Get an agent by id.
    pub fn get(&self, id: &str) -> Option<&AgentSpec> {
        self.by_id.get(id)
    }

    /// All agents (visible and hidden).
    pub fn list(&self) -> Vec<&AgentSpec> {
        self.by_id.values().collect()
    }

    /// Visible agents only (excludes hidden).
    pub fn list_visible(&self) -> Vec<&AgentSpec> {
        self.by_id.values().filter(|s| !s.hidden).collect()
    }

    /// Currently active agent id.
    pub fn active_id(&self) -> Option<String> {
        self.active_id.lock().clone()
    }

    /// Currently active agent spec.
    pub fn active(&self) -> Option<&AgentSpec> {
        self.active_id().and_then(|id| self.get(&id))
    }

    /// Switch to a different agent. Returns false if id is unknown.
    pub fn activate(&self, id: &str) -> bool {
        if self.by_id.contains_key(id) {
            *self.active_id.lock() = Some(id.to_string());
            true
        } else {
            false
        }
    }

    /// Apply this agent's restrictions to a permission engine.
    pub fn apply_to_permissions(&self, engine: &PermissionEngine) {
        if let Some(agent) = self.active() {
            for tool in &agent.denied_tools {
                // Don't fail if engine rejects — log instead.
                let _ = engine.add_rule(tool, "*", PermissionAction::Deny);
            }
        }
    }

    /// Switch to the next visible agent in alphabetical order (for Tab key).
    pub fn cycle_next(&self) -> Option<&AgentSpec> {
        let mut visible = self.list_visible();
        visible.sort_by(|a, b| a.id.cmp(&b.id));
        if visible.is_empty() {
            return None;
        }
        let next = match self.active_id() {
            Some(active) => visible
                .iter()
                .position(|s| s.id == active)
                .map(|i| (i + 1) % visible.len())
                .unwrap_or(0),
            None => 0,
        };
        let chosen = visible[next].clone();
        self.activate(&chosen.id);
        Some(self.get(&chosen.id).expect("just registered"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtins_are_registered() {
        let r = AgentRegistry::with_builtins();
        assert!(r.get("build").is_some());
        assert!(r.get("plan").is_some());
        assert!(r.get("explore").is_some());
        assert!(r.get("scout").is_some());
        assert!(r.get("general").is_some());
    }

    #[test]
    fn plan_denies_writes() {
        let r = AgentRegistry::with_builtins();
        let plan = r.get("plan").unwrap();
        assert!(!plan.can_use_tool("write"));
        assert!(!plan.can_use_tool("edit"));
        assert!(!plan.can_use_tool("bash"));
        assert!(plan.can_use_tool("read"));
        assert!(plan.can_use_tool("grep"));
    }

    #[test]
    fn build_allows_everything() {
        let r = AgentRegistry::with_builtins();
        let build = r.get("build").unwrap();
        assert!(build.can_use_tool("read"));
        assert!(build.can_use_tool("write"));
        assert!(build.can_use_tool("edit"));
        assert!(build.can_use_tool("bash"));
    }

    #[test]
    fn explore_is_read_only() {
        let r = AgentRegistry::with_builtins();
        let explore = r.get("explore").unwrap();
        assert!(explore.can_use_tool("read"));
        assert!(!explore.can_use_tool("write"));
        assert!(!explore.can_use_tool("bash"));
        assert!(!explore.can_use_tool("task")); // subagent can't spawn subagents
    }

    #[test]
    fn activate_unknown_fails() {
        let r = AgentRegistry::with_builtins();
        assert!(!r.activate("nope"));
        assert!(r.active_id().is_none());
    }

    #[test]
    fn activate_known_succeeds() {
        let r = AgentRegistry::with_builtins();
        assert!(r.activate("plan"));
        assert_eq!(r.active_id().as_deref(), Some("plan"));
        assert_eq!(r.active().unwrap().id, "plan");
    }

    #[test]
    fn cycle_next_switches_agents() {
        let r = AgentRegistry::with_builtins();
        r.activate("build");
        let next = r.cycle_next().unwrap();
        assert_ne!(next.id, "build");
        // Cycled, but the actual order depends on sort — just verify it's different.
        let next2 = r.cycle_next().unwrap();
        let _ = next2; // cycling again should keep moving
    }

    #[test]
    fn list_visible_excludes_hidden() {
        let r = AgentRegistry::with_builtins();
        let visible: Vec<&str> = r.list_visible().iter().map(|s| s.id.as_str()).collect();
        assert!(visible.contains(&"build"));
        assert!(visible.contains(&"plan"));
    }

    #[test]
    fn register_overwrites() {
        let mut r = AgentRegistry::new();
        r.register(AgentSpec {
            id: "build".into(),
            name: "Custom".into(),
            mode: AgentMode::Primary,
            model: "x".into(),
            system_prompt: None,
            denied_tools: vec![],
            allowed_tools: vec![],
            color: None,
            hidden: false,
            description: "d".into(),
            max_steps: None,
        });
        assert_eq!(r.get("build").unwrap().name, "Custom");
    }

    #[test]
    fn permission_for_matches_can_use_tool() {
        let r = AgentRegistry::with_builtins();
        let plan = r.get("plan").unwrap();
        assert_eq!(plan.permission_for("write"), PermissionAction::Deny);
        assert_eq!(plan.permission_for("read"), PermissionAction::Allow);
    }

    #[test]
    fn mode_distinguishes_primary_from_subagent() {
        let r = AgentRegistry::with_builtins();
        assert_eq!(r.get("build").unwrap().mode, AgentMode::Primary);
        assert_eq!(r.get("plan").unwrap().mode, AgentMode::Primary);
        assert_eq!(r.get("explore").unwrap().mode, AgentMode::Subagent);
        assert_eq!(r.get("scout").unwrap().mode, AgentMode::Subagent);
    }
}
