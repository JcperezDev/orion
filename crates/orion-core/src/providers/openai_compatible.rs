use crate::providers::traits::{ChatRequest, LlmProvider, TokenStream};
use anyhow::Result;
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;

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
    async fn chat_stream(&self, request: ChatRequest) -> Result<TokenStream> {
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

        let (tx, rx) = mpsc::unbounded_channel::<Result<String>>();
        let mut stream = res.bytes_stream();
        let mut buffer = String::new();

        tokio::spawn(async move {
            while let Some(item) = stream.next().await {
                match item {
                    Ok(bytes) => {
                        buffer.push_str(&String::from_utf8_lossy(&bytes));
                        while let Some(idx) = buffer.find('\n') {
                            let line = buffer[..idx].to_string();
                            buffer = buffer[idx + 1..].to_string();
                            let line = line.trim_end_matches('\r');
                            let data = match line.strip_prefix("data: ") {
                                Some(d) => d,
                                None => continue,
                            };
                            if data == "[DONE]" {
                                let _ = tx.send(Ok(String::new()));
                                return;
                            }
                            if let Ok(parsed) =
                                serde_json::from_str::<serde_json::Value>(data)
                            {
                                if let Some(choices) =
                                    parsed.get("choices").and_then(|c| c.as_array())
                                {
                                    for choice in choices {
                                        if let Some(text) = choice
                                            .get("delta")
                                            .and_then(|d| d.get("content"))
                                            .and_then(|t| t.as_str())
                                        {
                                            if !text.is_empty()
                                                && tx.send(Ok(text.to_string())).is_err()
                                            {
                                                return;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Err(anyhow::anyhow!("stream error: {e}")));
                        return;
                    }
                }
            }
        });

        Ok(Box::pin(UnboundedReceiverStream::new(rx)))
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
