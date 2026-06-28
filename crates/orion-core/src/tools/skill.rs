//! `skill` tool — load a SKILL.md on demand.

use super::{PermissionKind, Tool, ToolContext, ToolResult};
use crate::skills::{render_skill_summary, SkillRegistry};
use async_trait::async_trait;
use parking_lot::Mutex;
use serde_json::{json, Value};
use std::sync::Arc;

/// Tool that exposes the SkillRegistry to the agent.
pub struct SkillTool {
    registry: Arc<Mutex<SkillRegistry>>,
}

impl SkillTool {
    pub fn new(registry: SkillRegistry) -> Self {
        Self {
            registry: Arc::new(Mutex::new(registry)),
        }
    }

    pub fn registry(&self) -> SkillRegistry {
        self.registry.lock().clone()
    }
}

#[async_trait]
impl Tool for SkillTool {
    fn name(&self) -> &str {
        "skill"
    }

    fn description(&self) -> &str {
        "Load a SKILL.md by name and return its instructions. Use this when the user asks for specialized guidance (testing, refactoring, security review, etc.)."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["load", "list", "search"],
                    "description": "Action to perform."
                },
                "name": {
                    "type": "string",
                    "description": "Skill name (required for load)."
                },
                "query": {
                    "type": "string",
                    "description": "Search query (required for search)."
                }
            },
            "required": ["action"]
        })
    }

    fn requires_permission(&self) -> PermissionKind {
        PermissionKind::None
    }

    async fn execute(&self, args: Value, _ctx: &ToolContext) -> anyhow::Result<ToolResult> {
        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("'action' is required"))?;
        let reg = self.registry.lock().clone();

        match action {
            "load" => {
                let name = args
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("'name' required for load"))?;
                let skill = reg
                    .get(name)
                    .ok_or_else(|| anyhow::anyhow!("skill '{name}' not found"))?;
                Ok(ToolResult {
                    tool_call_id: String::new(),
                    content: render_skill_summary(skill),
                    is_error: false,
                })
            }
            "list" => {
                if reg.is_empty() {
                    return Ok(ToolResult {
                        tool_call_id: String::new(),
                        content: "No skills registered. Place SKILL.md files in .opencode/skills/ or ~/.claude/skills/.".into(),
                        is_error: false,
                    });
                }
                let lines: Vec<String> = reg
                    .list()
                    .iter()
                    .map(|s| format!("- **{}**: {}", s.name, s.description))
                    .collect();
                Ok(ToolResult {
                    tool_call_id: String::new(),
                    content: format!("{} skill(s):\n{}", reg.len(), lines.join("\n")),
                    is_error: false,
                })
            }
            "search" => {
                let query = args
                    .get("query")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("'query' required for search"))?;
                let hits = reg.search(query);
                if hits.is_empty() {
                    return Ok(ToolResult {
                        tool_call_id: String::new(),
                        content: format!("No skills match '{query}'."),
                        is_error: false,
                    });
                }
                let lines: Vec<String> = hits
                    .iter()
                    .take(10)
                    .map(|s| format!("- **{}**: {}", s.name, s.description))
                    .collect();
                Ok(ToolResult {
                    tool_call_id: String::new(),
                    content: lines.join("\n"),
                    is_error: false,
                })
            }
            other => Ok(ToolResult {
                tool_call_id: String::new(),
                content: format!("Unknown action '{other}'. Valid: load, list, search"),
                is_error: true,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skills::Skill;

    fn build_registry() -> SkillRegistry {
        let mut reg = SkillRegistry::new();
        reg.register(Skill {
            name: "rust-tests".into(),
            description: "Write Rust tests".into(),
            license: None,
            compatibility: None,
            audience: None,
            path: std::path::PathBuf::from("/tmp"),
            body: "Use cargo test.".into(),
        });
        reg
    }

    #[tokio::test]
    async fn load_skill() {
        let tool = SkillTool::new(build_registry());
        let ctx = ToolContext::new(std::path::PathBuf::from("/tmp"));
        let r = tool
            .execute(json!({"action": "load", "name": "rust-tests"}), &ctx)
            .await
            .unwrap();
        assert!(!r.is_error);
        assert!(r.content.contains("cargo test"));
    }

    #[tokio::test]
    async fn load_unknown_errors() {
        let tool = SkillTool::new(build_registry());
        let ctx = ToolContext::new(std::path::PathBuf::from("/tmp"));
        let r = tool
            .execute(json!({"action": "load", "name": "nope"}), &ctx)
            .await;
        assert!(r.is_err());
    }

    #[tokio::test]
    async fn list_skills() {
        let tool = SkillTool::new(build_registry());
        let ctx = ToolContext::new(std::path::PathBuf::from("/tmp"));
        let r = tool
            .execute(json!({"action": "list"}), &ctx)
            .await
            .unwrap();
        assert!(r.content.contains("rust-tests"));
        assert!(r.content.contains("1 skill"));
    }

    #[tokio::test]
    async fn list_empty() {
        let tool = SkillTool::new(SkillRegistry::new());
        let ctx = ToolContext::new(std::path::PathBuf::from("/tmp"));
        let r = tool
            .execute(json!({"action": "list"}), &ctx)
            .await
            .unwrap();
        assert!(r.content.contains("No skills"));
    }

    #[tokio::test]
    async fn search_finds() {
        let tool = SkillTool::new(build_registry());
        let ctx = ToolContext::new(std::path::PathBuf::from("/tmp"));
        let r = tool
            .execute(json!({"action": "search", "query": "rust"}), &ctx)
            .await
            .unwrap();
        assert!(r.content.contains("rust-tests"));
    }

    #[tokio::test]
    async fn search_no_match() {
        let tool = SkillTool::new(build_registry());
        let ctx = ToolContext::new(std::path::PathBuf::from("/tmp"));
        let r = tool
            .execute(json!({"action": "search", "query": "kotlin"}), &ctx)
            .await
            .unwrap();
        assert!(r.content.contains("No skills match"));
    }

    #[tokio::test]
    async fn unknown_action_errors() {
        let tool = SkillTool::new(build_registry());
        let ctx = ToolContext::new(std::path::PathBuf::from("/tmp"));
        let r = tool
            .execute(json!({"action": "bogus"}), &ctx)
            .await
            .unwrap();
        assert!(r.is_error);
    }

    #[test]
    fn parameters_are_valid() {
        let tool = SkillTool::new(SkillRegistry::new());
        let p = tool.parameters();
        assert!(p["properties"]["action"].is_object());
    }
}
