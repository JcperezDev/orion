use anyhow::Result;
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

#[derive(Debug, Clone)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub tools: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
}

pub type TokenStream = Pin<Box<dyn Stream<Item = Result<String>> + Send>>;

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn chat_stream(&self, request: ChatRequest) -> Result<TokenStream>;

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
