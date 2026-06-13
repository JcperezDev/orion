use std::sync::Arc;
use anyhow::Result;
use async_trait::async_trait;
use crate::providers::traits::{LlmProvider, ChatRequest, ChatStream, Message};
use reqwest::Client;
use futures::StreamExt;

pub struct OpenRouterProvider {
    client: Client,
    base_url: String,
    api_key: String,
}

impl OpenRouterProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            base_url: "https://openrouter.ai/api/v1".to_string(),
            api_key,
        }
    }

    fn transform_messages(messages: Vec<Message>) -> Vec<serde_json::Value> {
        messages
            .into_iter()
            .map(|m| {
                serde_json::json!({
                    "role": m.role,
                    "content": m.content
                })
            })
            .collect()
    }
}

#[async_trait]
impl LlmProvider for OpenRouterProvider {
    async fn chat_stream(&self, request: ChatRequest) -> Result<ChatStream> {
        let messages = Self::transform_messages(request.messages);

        let body = serde_json::json!({
            "model": request.model,
            "messages": messages,
            "stream": true,
            "temperature": request.temperature.unwrap_or(0.7),
            "max_tokens": request.max_tokens.unwrap_or(4096),
        });

        let res = self.client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
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
                                if let Some(choices) = parsed.get("choices").and_then(|c| c.as_array()) {
                                    for choice in choices {
                                        if let Some(delta) = choice.get("delta").and_then(|d| d.get("content")) {
                                            if let Some(text) = delta.as_str() {
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
        }

        Ok(ChatStream { content: full_response })
    }

    fn provider_id(&self) -> &'static str {
        "openrouter"
    }

    fn supports_tools(&self, model: &str) -> bool {
        !model.contains("vision")
    }

    fn supports_vision(&self, model: &str) -> bool {
        model.contains("vision") || model.contains("claude") || model.contains("gpt")
    }
}
