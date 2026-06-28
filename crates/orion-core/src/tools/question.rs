use crate::tools::{ApprovalRequest, ApprovalResponse, PermissionKind, Tool, ToolContext, ToolResult};
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub struct QuestionTool;

#[derive(Debug, Serialize, Deserialize)]
struct QuestionArgs {
    questions: Vec<QuestionItem>,
}

#[derive(Debug, Serialize, Deserialize)]
struct QuestionItem {
    question: String,
    #[serde(default)]
    header: Option<String>,
    #[serde(default)]
    options: Option<Vec<QuestionOption>>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    multiple: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
struct QuestionOption {
    label: String,
    #[serde(default)]
    description: Option<String>,
}

#[async_trait]
impl Tool for QuestionTool {
    fn name(&self) -> &str {
        "question"
    }

    fn description(&self) -> &str {
        "Use this tool when you need to ask the user questions during execution. This allows you to: 1) Gather user preferences or requirements, 2) Clarify ambiguous instructions, 3) Get decisions on implementation choices, 4) Offer choices about what direction to take."
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "questions": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "question": { "type": "string", "description": "Complete question to ask" },
                            "header": { "type": "string", "description": "Very short label (max 30 chars)" },
                            "options": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "label": { "type": "string", "description": "Display text (1-5 words)" },
                                        "description": { "type": "string", "description": "Explanation of choice" }
                                    },
                                    "required": ["label"]
                                }
                            },
                            "description": { "type": "string", "description": "Additional context for the question" },
                            "multiple": { "type": "boolean", "description": "Allow selecting more than one option" }
                        },
                        "required": ["question"]
                    },
                    "description": "Questions to ask the user"
                }
            },
            "required": ["questions"]
        })
    }

    fn requires_permission(&self) -> PermissionKind {
        PermissionKind::Interactive
    }

    fn action_summary(&self, args: &serde_json::Value) -> String {
        if let Some(questions) = args.get("questions").and_then(|q| q.as_array()) {
            if let Some(first) = questions.first() {
                if let Some(q) = first.get("question").and_then(|q| q.as_str()) {
                    if q.len() > 60 {
                        return format!("{}...", &q[..57]);
                    }
                    return q.to_string();
                }
            }
        }
        "question".to_string()
    }

    async fn execute(&self, args: serde_json::Value, ctx: &ToolContext) -> Result<ToolResult> {
        let args: QuestionArgs =
            serde_json::from_value(args).context("invalid args for question tool")?;

        let request = ApprovalRequest {
            tool_name: "question".into(),
            action: format!(
                "ask: {}",
                args.questions
                    .first()
                    .map(|q| q.question.as_str())
                    .unwrap_or("?")
            ),
            matched_pattern: None,
            arguments: serde_json::to_value(&args).unwrap_or_default(),
        };

        let response = ctx.ask(request).await;
        match response {
            ApprovalResponse::Deny => Ok(ToolResult {
                tool_call_id: String::new(),
                content: "User declined to answer the questions".into(),
                is_error: true,
            }),
            ApprovalResponse::AllowAlways | ApprovalResponse::Allow => {
                let formatted = args
                    .questions
                    .iter()
                    .map(|q| {
                        format!(
                            "\"{}\"=\"User answered via approval dialog\"",
                            q.question
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                Ok(ToolResult {
                    tool_call_id: String::new(),
                    content: format!(
                        "User has answered your questions: {}. You can now continue with the user's answers in mind.",
                        formatted
                    ),
                    is_error: false,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn question_parameters_shape() {
        let tool = QuestionTool;
        let params = tool.parameters();
        assert!(params.get("properties").is_some());
        assert!(params.get("required").is_some());
    }
}
