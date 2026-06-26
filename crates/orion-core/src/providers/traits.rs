use anyhow::Result;
use async_trait::async_trait;
use futures::Stream;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::tools::StreamChunk;

#[derive(Debug, Clone)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub tools: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    #[serde(default)]
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<RequestedToolCall>>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub is_error: bool,
}

fn is_false(b: &bool) -> bool {
    !*b
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestedToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

pub type TokenStream = Pin<Box<dyn Stream<Item = Result<String>> + Send>>;
pub type ChunkStream = Pin<Box<dyn Stream<Item = Result<crate::tools::StreamChunk>> + Send>>;

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn chat_stream(&self, request: ChatRequest) -> Result<TokenStream>;

    async fn chat_with_tools(
        &self,
        request: ChatRequest,
        _tools: Vec<serde_json::Value>,
    ) -> Result<ChunkStream> {
        // Default: providers without tool support fall back to a chunk stream
        // that emits the text as a single Token and then Done. Callers should
        // check `supports_tools` before relying on tool dispatch.
        use futures::StreamExt;
        let mut stream = self.chat_stream(request).await?;
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<Result<StreamChunk>>();
        tokio::spawn(async move {
            while let Some(item) = stream.next().await {
                match item {
                    Ok(text) => {
                        if text.is_empty() {
                            continue;
                        }
                        if tx.send(Ok(crate::tools::StreamChunk::Token { text })).is_err() {
                            return;
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Err(e));
                        return;
                    }
                }
            }
            let _ = tx.send(Ok(crate::tools::StreamChunk::Done {
                stop_reason: Some("stop".into()),
            }));
        });
        Ok(Box::pin(UnboundedReceiverStream::new(rx)))
    }

    fn provider_id(&self) -> &str;

    fn supports_tools(&self, _model: &str) -> bool {
        false
    }

    fn supports_vision(&self, model: &str) -> bool {
        model.contains("vision")
            || model.contains("claude")
            || model.contains("gpt")
            || model.contains("gemini")
    }

    fn supports_reasoning(&self, model: &str) -> bool {
        model.contains("thinking") || model.contains("reasoning")
    }

    fn base_url(&self) -> &str;

    fn api_key_env(&self) -> &str;

    fn requires_api_key(&self) -> bool {
        true
    }
}
