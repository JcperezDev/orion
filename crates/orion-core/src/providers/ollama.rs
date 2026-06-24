use crate::providers::traits::{ChatRequest, LlmProvider, TokenStream};
use anyhow::Result;
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;

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
                            if line.is_empty() {
                                continue;
                            }
                            if let Ok(parsed) =
                                serde_json::from_str::<serde_json::Value>(line)
                            {
                                let done = parsed
                                    .get("done")
                                    .and_then(|d| d.as_bool())
                                    .unwrap_or(false);
                                if let Some(text) = parsed
                                    .get("message")
                                    .and_then(|m| m.get("content"))
                                    .and_then(|c| c.as_str())
                                {
                                    if !text.is_empty()
                                        && tx.send(Ok(text.to_string())).is_err()
                                    {
                                        return;
                                    }
                                }
                                if done {
                                    let _ = tx.send(Ok(String::new()));
                                    return;
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
