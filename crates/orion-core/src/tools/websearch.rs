use crate::tools::{ApprovalRequest, ApprovalResponse, PermissionKind, Tool, ToolContext, ToolResult};
use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const MAX_NUM_RESULTS: u32 = 20;
const EXA_URL: &str = "https://mcp.exa.ai/mcp";
const PARALLEL_URL: &str = "https://search.parallel.ai/mcp";

pub struct WebSearchTool {
    client: Client,
    exa_api_key: Option<String>,
    parallel_api_key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct WebSearchArgs {
    query: String,
    #[serde(default = "default_num_results")]
    num_results: Option<u32>,
    #[serde(default)]
    livecrawl: Option<String>,
    #[serde(rename = "type")]
    search_type: Option<String>,
    #[serde(default)]
    context_max_characters: Option<u32>,
}

fn default_num_results() -> Option<u32> {
    None
}

impl WebSearchTool {
    pub fn new(exa_api_key: Option<String>, parallel_api_key: Option<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("orion-agent/0.1")
            .build()
            .unwrap_or_default();
        Self { client, exa_api_key, parallel_api_key }
    }

    fn select_provider(&self) -> &str {
        if self.parallel_api_key.is_some() {
            "parallel"
        } else if self.exa_api_key.is_some() {
            "exa"
        } else {
            "parallel"
        }
    }
}

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &str {
        "websearch"
    }

    fn description(&self) -> &str {
        "Search the web using the configured search provider. Use this for current information beyond knowledge cutoff. Supports Exa and Parallel backends. Returns text results with source URLs."
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Web search query"
                },
                "num_results": {
                    "type": "integer",
                    "description": "Number of search results to return (default: 8, max: 20)"
                },
                "livecrawl": {
                    "type": "string",
                    "enum": ["fallback", "preferred"],
                    "description": "Live crawl mode"
                },
                "type": {
                    "type": "string",
                    "enum": ["auto", "fast", "deep"],
                    "description": "Search type"
                },
                "context_max_characters": {
                    "type": "integer",
                    "description": "Maximum characters for context (default: 10000, max: 50000)"
                }
            },
            "required": ["query"]
        })
    }

    fn requires_permission(&self) -> PermissionKind {
        PermissionKind::Network
    }

    fn action_summary(&self, args: &serde_json::Value) -> String {
        args.get("query")
            .and_then(|v| v.as_str())
            .unwrap_or("websearch")
            .to_string()
    }

    async fn execute(&self, args: serde_json::Value, ctx: &ToolContext) -> Result<ToolResult> {
        let args: WebSearchArgs =
            serde_json::from_value(args).context("invalid args for websearch tool")?;

        let request = ApprovalRequest {
            tool_name: "websearch".into(),
            action: format!("search: {}", args.query),
            matched_pattern: None,
            arguments: serde_json::to_value(&args).unwrap_or_default(),
        };
        match ctx.ask(request).await {
            ApprovalResponse::Deny => {
                return Ok(ToolResult {
                    tool_call_id: String::new(),
                    content: "denied by user".into(),
                    is_error: true,
                });
            }
            ApprovalResponse::Allow | ApprovalResponse::AllowAlways => {}
        }

        let provider = self.select_provider();
        let num = args.num_results.unwrap_or(8).min(MAX_NUM_RESULTS);

        let result = match provider {
            "exa" => self.search_exa(&args.query, num, args.livecrawl.as_deref(), args.search_type.as_deref()).await,
            _ => self.search_parallel(&args.query, args.search_type.as_deref()).await,
        };

        match result {
            Ok(text) => Ok(ToolResult {
                tool_call_id: String::new(),
                content: format!("Search results (provider: {provider}):\n\n{text}"),
                is_error: false,
            }),
            Err(e) => Ok(ToolResult {
                tool_call_id: String::new(),
                content: format!("search error: {e}"),
                is_error: true,
            }),
        }
    }
}

impl WebSearchTool {
    async fn search_exa(
        &self,
        query: &str,
        num_results: u32,
        livecrawl: Option<&str>,
        search_type: Option<&str>,
    ) -> Result<String> {
        let api_key = self.exa_api_key.as_deref().unwrap_or("");
        let url = if api_key.is_empty() {
            EXA_URL.to_string()
        } else {
            format!("{}?exaApiKey={}", EXA_URL, api_key)
        };

        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "web_search_exa",
                "arguments": {
                    "query": query,
                    "type": search_type.unwrap_or("auto"),
                    "numResults": num_results,
                    "livecrawl": livecrawl.unwrap_or("fallback")
                }
            }
        });

        let resp = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .json(&body)
            .send()
            .await
            .context("exa request failed")?;

        let text = resp.text().await.context("exa response read failed")?;
        parse_mcp_response(&text)
    }

    async fn search_parallel(&self, query: &str, _search_type: Option<&str>) -> Result<String> {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "web_search",
                "arguments": {
                    "objective": query,
                    "search_queries": [query],
                    "session_id": "orion-session"
                }
            }
        });

        let mut req = self
            .client
            .post(PARALLEL_URL)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .json(&body);

        if let Some(key) = &self.parallel_api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }

        let resp = req.send().await.context("parallel request failed")?;
        let text = resp.text().await.context("parallel response read failed")?;
        parse_mcp_response(&text)
    }
}

fn parse_mcp_response(body: &str) -> Result<String> {
    let trimmed = body.trim();

    if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
        if let Some(content) = val.pointer("/result/content") {
            if let Some(items) = content.as_array() {
                for item in items {
                    if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                        if !text.is_empty() {
                            return Ok(text.to_string());
                        }
                    }
                }
            }
        }
    }

    for line in trimmed.lines() {
        if let Some(data) = line.strip_prefix("data: ") {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(data) {
                if let Some(text) = val
                    .pointer("/result/content")
                    .and_then(|c| c.as_array())
                    .and_then(|items| {
                        items.iter().find_map(|i| i.get("text").and_then(|t| t.as_str()))
                    })
                {
                    if !text.is_empty() {
                        return Ok(text.to_string());
                    }
                }
            }
        }
    }

    Ok("No search results found. Please try a different query.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_mcp_json_response() {
        let json = r#"{"result":{"content":[{"type":"text","text":"result text"}]}}"#;
        let result = parse_mcp_response(json).unwrap();
        assert_eq!(result, "result text");
    }

    #[test]
    fn parse_mcp_sse_response() {
        let sse = "data: {\"result\":{\"content\":[{\"type\":\"text\",\"text\":\"sse result\"}]}}\n";
        let result = parse_mcp_response(sse).unwrap();
        assert_eq!(result, "sse result");
    }
}
