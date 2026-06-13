use std::sync::Arc;
use anyhow::Result;
use crate::providers::traits::{LlmProvider, ChatRequest};

pub struct FallbackChain {
    providers: Vec<(String, Arc<dyn LlmProvider>)>,
}

impl FallbackChain {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    pub fn add(&mut self, model_id: &str, provider: Arc<dyn LlmProvider>) {
        self.providers.push((model_id.to_string(), provider));
    }

    pub async fn execute(&self, request: ChatRequest) -> Result<String> {
        let mut last_error = None;

        for (model_id, provider) in &self.providers {
            let mut req = request.clone();
            req.model = model_id.clone();

            match provider.chat_stream(req).await {
                Ok(stream) => return Ok(stream.content),
                Err(e) => {
                    last_error = Some(e);
                    continue;
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("All providers failed")))
    }
}

impl Default for FallbackChain {
    fn default() -> Self {
        Self::new()
    }
}
