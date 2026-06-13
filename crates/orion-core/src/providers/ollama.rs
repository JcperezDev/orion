use crate::providers::traits::{ChatRequest, ChatStream, LlmProvider};
use anyhow::Result;
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;

pub struct OllamaProvider {
    client: Client,
    base_url: String,
}

impl OllamaProvider {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }
}

#[async_trait]
impl LlmProvider for OllamaProvider {
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
            "stream": true,
        });

        let res = self
            .client
            .post(format!("{}/api/chat", self.base_url))
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
                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(line) {
                            if let Some(message) =
                                parsed.get("message").and_then(|m| m.get("content"))
                            {
                                if let Some(text) = message.as_str() {
                                    full_response.push_str(text);
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
        "ollama"
    }

    fn supports_vision(&self, model: &str) -> bool {
        model.contains("vision") || model.contains("llava")
    }

    fn requires_api_key(&self) -> bool {
        false
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    fn api_key_env(&self) -> &str {
        ""
    }
}
