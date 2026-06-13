use crate::providers::traits::{ChatRequest, ChatStream, LlmProvider};
use anyhow::Result;
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;

pub struct AnthropicProvider {
    client: Client,
    api_key: String,
}

impl AnthropicProvider {
    pub fn new(api_key: &str) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.to_string(),
        }
    }
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    async fn chat_stream(&self, request: ChatRequest) -> Result<ChatStream> {
        let messages: Vec<serde_json::Value> = request
            .messages
            .into_iter()
            .map(|m| {
                serde_json::json!({
                    "role": m.role,
                    "content": m.content
                })
            })
            .collect();

        let body = serde_json::json!({
            "model": request.model,
            "messages": messages,
            "max_tokens": request.max_tokens.unwrap_or(4096),
            "stream": true,
        });

        let res = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        let mut stream = res.bytes_stream();
        let mut full_response = String::new();

        while let Some(item) = stream.next().await {
            if let Ok(bytes) = item {
                if let Ok(text) = String::from_utf8(bytes.to_vec()) {
                    for line in text.lines() {
                        if line.starts_with("data: ") {
                            let data = &line[6..];
                            if data == "[DONE]" {
                                break;
                            }
                            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data) {
                                if let Some(content) =
                                    parsed.get("content").and_then(|c| c.as_array())
                                {
                                    for item in content {
                                        if let Some(text) =
                                            item.get("text").and_then(|t| t.as_str())
                                        {
                                            full_response.push_str(text);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(ChatStream {
            content: full_response,
        })
    }

    fn provider_id(&self) -> &'static str {
        "anthropic"
    }

    fn supports_tools(&self, _model: &str) -> bool {
        true
    }

    fn supports_vision(&self, _model: &str) -> bool {
        true
    }

    fn supports_reasoning(&self, model: &str) -> bool {
        model.contains("sonnet") || model.contains("opus")
    }

    fn base_url(&self) -> &str {
        "https://api.anthropic.com"
    }

    fn api_key_env(&self) -> &str {
        "ANTHROPIC_API_KEY"
    }
}
