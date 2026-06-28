use crate::providers::traits::{ChatRequest, LlmProvider, Message, RequestedToolCall, TokenStream};
use crate::tools::StreamChunk;
use anyhow::Result;
use async_trait::async_trait;
use futures::{stream, StreamExt};
use reqwest::Client;
use std::collections::HashMap;
use std::pin::Pin;
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;

pub type ChunkStream = Pin<Box<dyn futures::Stream<Item = Result<StreamChunk>> + Send>>;

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

fn message_to_json(m: &Message) -> serde_json::Value {
    let mut obj = serde_json::json!({
        "role": m.role,
        "content": m.content,
    });
    if let Some(id) = &m.tool_call_id {
        obj["tool_call_id"] = serde_json::json!(id);
    }
    if let Some(tcs) = &m.tool_calls {
        obj["tool_calls"] = serde_json::json!(tcs.iter().map(|t| serde_json::json!({
            "id": t.id,
            "type": "function",
            "function": {
                "name": t.name,
                "arguments": serde_json::to_string(&t.arguments).unwrap_or_else(|_| "{}".into()),
            }
        })).collect::<Vec<_>>());
    }
    obj
}

#[async_trait]
impl LlmProvider for OpenAICompatibleProvider {
    async fn chat_stream(&self, request: ChatRequest) -> Result<TokenStream> {
        let messages: Vec<serde_json::Value> =
            request.messages.iter().map(message_to_json).collect();

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

        // Surface HTTP errors instead of silently streaming an error body that
        // yields no tokens (e.g. a wrong model id or endpoint).
        let status = res.status();
        if !status.is_success() {
            let body = res.text().await.unwrap_or_default();
            let snippet: String = body.chars().take(600).collect();
            return Err(anyhow::anyhow!(
                "provider returned HTTP {}: {}",
                status.as_u16(),
                snippet.trim()
            ));
        }

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

    async fn chat_with_tools(
        &self,
        request: ChatRequest,
        tools: Vec<serde_json::Value>,
    ) -> Result<ChunkStream> {
        let messages: Vec<serde_json::Value> =
            request.messages.iter().map(message_to_json).collect();

        let mut body = serde_json::json!({
            "model": request.model,
            "messages": messages,
            "stream": true,
        });
        if !tools.is_empty() {
            body["tools"] = serde_json::json!(tools);
        }
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

        let status = res.status();
        if !status.is_success() {
            let body = res.text().await.unwrap_or_default();
            let snippet: String = body.chars().take(600).collect();
            return Err(anyhow::anyhow!(
                "provider returned HTTP {}: {}",
                status.as_u16(),
                snippet.trim()
            ));
        }

        let (tx, rx) = mpsc::unbounded_channel::<Result<StreamChunk>>();
        let mut stream = res.bytes_stream();
        let mut buffer = String::new();

        tokio::spawn(async move {
            // Per-index tool call accumulation: index -> (id, name, args_buf)
            let mut tool_calls: HashMap<usize, (String, String, String)> = HashMap::new();
            let mut stop_reason: Option<String> = None;

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
                                // Emit any accumulated tool calls in index order.
                                let mut indices: Vec<usize> = tool_calls.keys().copied().collect();
                                indices.sort();
                                for i in indices {
                                    if let Some((id, name, args_buf)) = tool_calls.remove(&i) {
                                        let arguments = serde_json::from_str(&args_buf)
                                            .unwrap_or(serde_json::Value::String(args_buf));
                                        let call = crate::tools::ToolCall {
                                            id,
                                            name,
                                            arguments,
                                        };
                                        if tx.send(Ok(StreamChunk::ToolCall { call })).is_err() {
                                            return;
                                        }
                                    }
                                }
                                let _ = tx.send(Ok(StreamChunk::Done {
                                    stop_reason: stop_reason.clone(),
                                }));
                                return;
                            }
                            if let Ok(parsed) =
                                serde_json::from_str::<serde_json::Value>(data)
                            {
                                if let Some(sr) = parsed
                                    .get("choices")
                                    .and_then(|c| c.as_array())
                                    .and_then(|arr| arr.first())
                                    .and_then(|c| c.get("finish_reason"))
                                    .and_then(|f| f.as_str())
                                {
                                    stop_reason = Some(sr.to_string());
                                }
                                if let Some(choices) =
                                    parsed.get("choices").and_then(|c| c.as_array())
                                {
                                    for choice in choices {
                                        let delta = match choice.get("delta") {
                                            Some(d) => d,
                                            None => continue,
                                        };
                                        if let Some(text) =
                                            delta.get("content").and_then(|t| t.as_str())
                                        {
                                            if !text.is_empty() {
                                                let _ = tx.send(Ok(StreamChunk::Token {
                                                    text: text.to_string(),
                                                }));
                                            }
                                        }
                                        if let Some(tcs) = delta
                                            .get("tool_calls")
                                            .and_then(|t| t.as_array())
                                        {
                                            for tc in tcs {
                                                let index = tc
                                                    .get("index")
                                                    .and_then(|i| i.as_u64())
                                                    .map(|n| n as usize)
                                                    .unwrap_or(0);
                                                let entry = tool_calls
                                                    .entry(index)
                                                    .or_insert_with(|| {
                                                        (
                                                            String::new(),
                                                            String::new(),
                                                            String::new(),
                                                        )
                                                    });
                                                if let Some(id) =
                                                    tc.get("id").and_then(|s| s.as_str())
                                                {
                                                    if !id.is_empty() {
                                                        entry.0 = id.to_string();
                                                    }
                                                }
                                                if let Some(func) = tc.get("function") {
                                                    if let Some(name) =
                                                        func.get("name").and_then(|s| s.as_str())
                                                    {
                                                        entry.1.push_str(name);
                                                    }
                                                    if let Some(args) =
                                                        func.get("arguments").and_then(|s| s.as_str())
                                                    {
                                                        entry.2.push_str(args);
                                                    }
                                                }
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

    fn supports_tools(&self, _model: &str) -> bool {
        true
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    fn api_key_env(&self) -> &str {
        ""
    }
}

// Suppress unused warning for stream helper when no tool-call path is exercised.
#[allow(dead_code)]
fn _stream_compat(s: ChunkStream) -> impl futures::Stream<Item = Result<StreamChunk>> {
    stream::unfold(s, |mut s| async move { s.next().await.map(|c| (c, s)) })
}

// Convenience: build a RequestedToolCall for tests.
#[allow(dead_code)]
pub fn example_tool_call() -> RequestedToolCall {
    RequestedToolCall {
        id: "call_test".into(),
        name: "read".into(),
        arguments: serde_json::json!({"path": "/tmp/x"}),
    }
}

// Convenience constructor used by Message helpers in tests.
#[allow(dead_code)]
pub fn tool_message(id: &str, content: &str, is_error: bool) -> Message {
    Message {
        role: "tool".into(),
        content: content.into(),
        tool_call_id: Some(id.into()),
        tool_calls: None,
        is_error,
    }
}
