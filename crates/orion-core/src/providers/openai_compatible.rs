use crate::providers::traits::{ChatRequest, ChatStream, LlmProvider};
use anyhow::Result;
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;

pub struct OpenAICompatibleProvider {
    id: String,
    client: Client,
    base_url: String,
    api_key: String,
}

impl OpenAICompatibleProvider {
    pub fn new(id: &str, base_url: &str, api_key: &str) -> Self {
        Self {
            id: id.to_string(),
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
        }
    }
}

#[async_trait]
impl LlmProvider for OpenAICompatibleProvider {
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

        let mut body = serde_json::json!({
            "model": request.model,
            "messages": messages,
            "stream": true,
        });

        if let Some(temp) = request.temperature {
            body["temperature"] = serde_json::json!(temp);
        }

        let res = self
            .client
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
                                if let Some(choices) =
                                    parsed.get("choices").and_then(|c| c.as_array())
                                {
                                    for choice in choices {
                                        if let Some(delta) =
                                            choice.get("delta").and_then(|d| d.get("content"))
                                        {
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

        Ok(ChatStream {
            content: full_response,
        })
    }

    fn provider_id(&self) -> &str {
        &self.id
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    fn api_key_env(&self) -> &str {
        ""
    }
}
