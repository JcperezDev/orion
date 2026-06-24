use std::sync::Arc;
use anyhow::Result;
use crate::providers::traits::{LlmProvider, ChatRequest};
use futures::StreamExt;

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
                Ok(mut stream) => {
                    let mut out = String::new();
                    let mut errored = false;
                    while let Some(chunk) = stream.next().await {
                        match chunk {
                            Ok(s) if s.is_empty() => break,
                            Ok(s) => out.push_str(&s),
                            Err(e) => {
                                last_error = Some(e);
                                errored = true;
                                break;
                            }
                        }
                    }
                    if !errored {
                        return Ok(out);
                    }
                }
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
