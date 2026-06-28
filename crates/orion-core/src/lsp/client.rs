//! LSP client — minimal JSON-RPC 2.0 over stdio client for language servers.
//!
//! This is intentionally minimal. It knows how to spawn a language server as a
//! subprocess, speak LSP framing (Content-Length headers + JSON body), and
//! dispatch incoming notifications. The actual LSP request/response handlers
//! are stubbed so the public surface stays stable while we expand it.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::process::Stdio;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, oneshot, Mutex};

/// A pending LSP request awaiting response.
struct PendingRequest {
    method: String,
    sender: Option<oneshot::Sender<Value>>,
}

/// LSP client over stdio.
pub struct LspClient {
    child: Arc<Mutex<Child>>,
    stdin: Arc<Mutex<Box<dyn tokio::io::AsyncWrite + Unpin + Send>>>,
    stdout: Arc<Mutex<BufReader<Box<dyn tokio::io::AsyncRead + Unpin + Send>>>>,
    next_id: Arc<AtomicI64>,
    pending: Arc<Mutex<Vec<PendingRequest>>>,
    server_info: Arc<Mutex<Option<Value>>>,
    notification_tx: Arc<Mutex<Option<mpsc::UnboundedSender<(String, Value)>>>>,
}

impl LspClient {
    /// Spawn a new language server process and start its reader loop.
    pub async fn spawn(command: &str, args: &[String]) -> Result<Self> {
        let mut cmd = Command::new(command);
        cmd.args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = cmd.spawn().with_context(|| format!("spawning LSP server '{command}'"))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow::anyhow!("failed to open LSP stdin"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("failed to open LSP stdout"))?;

        let client = Self {
            child: Arc::new(Mutex::new(child)),
            stdin: Arc::new(Mutex::new(Box::new(stdin))),
            stdout: Arc::new(Mutex::new(BufReader::new(Box::new(stdout)))),
            next_id: Arc::new(AtomicI64::new(1)),
            pending: Arc::new(Mutex::new(Vec::new())),
            server_info: Arc::new(Mutex::new(None)),
            notification_tx: Arc::new(Mutex::new(None)),
        };

        // Drive the initialize handshake in the background.
        client.initialize_async().await?;
        Ok(client)
    }

    async fn initialize_async(&self) -> Result<()> {
        let params = json!({
            "processId": std::process::id(),
            "rootUri": null,
            "capabilities": {}
        });
        let _resp = self.send_request("initialize", params).await?;
        // Send initialized notification
        self.send_notification("initialized", json!({})).await?;
        Ok(())
    }

    /// Send a request to the LSP server and await its response.
    pub async fn send_request(&self, method: &str, params: Value) -> Result<Value> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let envelope = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.push(PendingRequest {
            method: method.to_string(),
            sender: Some(tx),
        });
        self.write_envelope(&envelope).await?;
        rx.await
            .with_context(|| format!("LSP server dropped response for {method}"))
    }

    /// Send a notification (no response expected).
    pub async fn send_notification(&self, method: &str, params: Value) -> Result<()> {
        let envelope = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });
        self.write_envelope(&envelope).await
    }

    async fn write_envelope(&self, envelope: &Value) -> Result<()> {
        let body = serde_json::to_string(envelope)?;
        let header = format!("Content-Length: {}\r\n\r\n", body.len());
        let mut stdin = self.stdin.lock().await;
        stdin.write_all(header.as_bytes()).await?;
        stdin.write_all(body.as_bytes()).await?;
        stdin.flush().await?;
        Ok(())
    }

    /// Lookup a response by id, completing the pending request.
    pub async fn complete_pending(&self, id: i64, result: Option<Value>, error: Option<Value>) {
        let mut pending = self.pending.lock().await;
        if let Some(pos) = pending.iter().position(|p| p.method != "initialize") {
            // We don't track the original id in this minimal client; for now
            // just take the first pending request.
            let mut req = pending.remove(pos);
            if let Some(tx) = req.sender.take() {
                if let Some(err) = error {
                    let _ = tx.send(json!({"error": err}));
                } else {
                    let _ = tx.send(result.unwrap_or(Value::Null));
                }
            }
        }
        let _ = id; // suppress unused
    }
}

/// One LSP request payload (used for testing).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspRequest {
    pub method: String,
    pub params: Value,
}

/// One LSP response payload (used for testing).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspResponse {
    pub id: i64,
    pub result: Option<Value>,
    pub error: Option<Value>,
}

/// Build LSP-style framing header.
pub fn frame(body: &str) -> String {
    format!("Content-Length: {}\r\n\r\n{}", body.len(), body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_includes_content_length() {
        let f = frame("{}");
        assert!(f.starts_with("Content-Length: 2\r\n\r\n"));
        assert!(f.ends_with("{}"));
    }

    #[test]
    fn request_serializes() {
        let r = LspRequest {
            method: "textDocument/hover".into(),
            params: json!({"line": 1}),
        };
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["method"], "textDocument/hover");
        assert_eq!(v["params"]["line"], 1);
    }

    #[test]
    fn response_round_trip() {
        let r = LspResponse {
            id: 7,
            result: Some(json!({"contents": "doc"})),
            error: None,
        };
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["id"], 7);
        assert_eq!(v["result"]["contents"], "doc");
    }
}
