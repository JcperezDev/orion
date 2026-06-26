use crate::providers::traits::{ChatRequest, LlmProvider, Message, TokenStream};
use crate::tools::StreamChunk;
use anyhow::Result;
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use std::collections::HashMap;
use std::pin::Pin;
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;

pub type ChunkStream = Pin<Box<dyn futures::Stream<Item = Result<StreamChunk>> + Send>>;

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

fn message_to_anthropic(m: &Message) -> serde_json::Value {
    // Tool result message -> user role with tool_result content block.
    if m.role == "tool" {
        if let Some(id) = &m.tool_call_id {
            return serde_json::json!({
                "role": "user",
                "content": [{
                    "type": "tool_result",
                    "tool_use_id": id,
                    "content": m.content,
                    "is_error": m.is_error,
                }]
            });
        }
    }
    // Assistant message with tool_calls -> content array of tool_use blocks.
    if m.role == "assistant" {
        if let Some(tcs) = &m.tool_calls {
            let mut blocks: Vec<serde_json::Value> = Vec::new();
            if !m.content.is_empty() {
                blocks.push(serde_json::json!({"type": "text", "text": m.content}));
            }
            for t in tcs {
                blocks.push(serde_json::json!({
                    "type": "tool_use",
                    "id": t.id,
                    "name": t.name,
                    "input": t.arguments,
                }));
            }
            return serde_json::json!({
                "role": "assistant",
                "content": blocks,
            });
        }
    }
    serde_json::json!({"role": m.role, "content": m.content})
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    async fn chat_stream(&self, request: ChatRequest) -> Result<TokenStream> {
        let messages: Vec<serde_json::Value> =
            request.messages.iter().map(message_to_anthropic).collect();

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
                            // Anthropic SSE: lines start with "event: " or "data: "
                            if let Some(data) = line.strip_prefix("data: ") {
                                if let Ok(parsed) =
                                    serde_json::from_str::<serde_json::Value>(data)
                                {
                                    if parsed.get("type").and_then(|t| t.as_str())
                                        == Some("content_block_delta")
                                    {
                                        if let Some(text) = parsed
                                            .get("delta")
                                            .and_then(|d| d.get("text"))
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

    async fn chat_with_tools(
        &self,
        request: ChatRequest,
        tools: Vec<serde_json::Value>,
    ) -> Result<ChunkStream> {
        let messages: Vec<serde_json::Value> =
            request.messages.iter().map(message_to_anthropic).collect();

        let mut body = serde_json::json!({
            "model": request.model,
            "messages": messages,
            "max_tokens": request.max_tokens.unwrap_or(4096),
            "stream": true,
        });
        if !tools.is_empty() {
            body["tools"] = serde_json::json!(tools);
        }

        let res = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        let (tx, rx) = mpsc::unbounded_channel::<Result<StreamChunk>>();
        let mut stream = res.bytes_stream();
        let mut buffer = String::new();

        tokio::spawn(async move {
            // Per-index content block state.
            #[derive(Default)]
            struct BlockState {
                kind: String,
                id: String,
                name: String,
                args_buf: String,
            }
            let mut blocks: HashMap<usize, BlockState> = HashMap::new();
            let mut stop_reason: Option<String> = None;

            while let Some(item) = stream.next().await {
                match item {
                    Ok(bytes) => {
                        buffer.push_str(&String::from_utf8_lossy(&bytes));
                        while let Some(idx) = buffer.find('\n') {
                            let line = buffer[..idx].to_string();
                            buffer = buffer[idx + 1..].to_string();
                            let line = line.trim_end_matches('\r');
                            if let Some(data) = line.strip_prefix("data: ") {
                                let parsed: serde_json::Value = match serde_json::from_str(data) {
                                    Ok(v) => v,
                                    Err(_) => continue,
                                };
                                let typ = parsed.get("type").and_then(|t| t.as_str()).unwrap_or("");
                                match typ {
                                    "content_block_start" => {
                                        if let Some(idx) = parsed.get("index").and_then(|i| i.as_u64()).map(|n| n as usize) {
                                            let cb = parsed.get("content_block").cloned().unwrap_or(serde_json::json!({}));
                                            let mut bs = BlockState::default();
                                            bs.kind = cb.get("type").and_then(|t| t.as_str()).unwrap_or("text").to_string();
                                            if bs.kind == "tool_use" {
                                                bs.id = cb.get("id").and_then(|s| s.as_str()).unwrap_or("").to_string();
                                                bs.name = cb.get("name").and_then(|s| s.as_str()).unwrap_or("").to_string();
                                            }
                                            blocks.insert(idx, bs);
                                        }
                                    }
                                    "content_block_delta" => {
                                        if let Some(idx) = parsed.get("index").and_then(|i| i.as_u64()).map(|n| n as usize) {
                                            let delta = parsed.get("delta").cloned().unwrap_or(serde_json::json!({}));
                                            let dtyp = delta.get("type").and_then(|t| t.as_str()).unwrap_or("");
                                            let entry = blocks.entry(idx).or_default();
                                            match dtyp {
                                                "text_delta" => {
                                                    if let Some(text) = delta.get("text").and_then(|t| t.as_str()) {
                                                        if !text.is_empty() {
                                                            let _ = tx.send(Ok(StreamChunk::Token { text: text.to_string() }));
                                                        }
                                                    }
                                                }
                                                "input_json_delta" => {
                                                    if let Some(pj) = delta.get("partial_json").and_then(|s| s.as_str()) {
                                                        entry.args_buf.push_str(pj);
                                                    }
                                                }
                                                _ => {}
                                            }
                                        }
                                    }
                                    "content_block_stop" => {
                                        if let Some(idx) = parsed.get("index").and_then(|i| i.as_u64()).map(|n| n as usize) {
                                            if let Some(bs) = blocks.remove(&idx) {
                                                if bs.kind == "tool_use" {
                                                    let arguments = serde_json::from_str(&bs.args_buf).unwrap_or(serde_json::Value::String(bs.args_buf));
                                                    let call = crate::tools::ToolCall {
                                                        id: bs.id,
                                                        name: bs.name,
                                                        arguments,
                                                    };
                                                    let _ = tx.send(Ok(StreamChunk::ToolCall { call }));
                                                }
                                            }
                                        }
                                    }
                                    "message_delta" => {
                                        if let Some(sr) = parsed
                                            .get("delta")
                                            .and_then(|d| d.get("stop_reason"))
                                            .and_then(|s| s.as_str())
                                        {
                                            stop_reason = Some(sr.to_string());
                                        }
                                    }
                                    "message_stop" => {
                                        let _ = tx.send(Ok(StreamChunk::Done {
                                            stop_reason: stop_reason.clone(),
                                        }));
                                        return;
                                    }
                                    _ => {}
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
