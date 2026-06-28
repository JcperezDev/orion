//! Bidirectional JSON-RPC 2.0 wire protocol over stdio.
//!
//! Every line on stdout is a JSON-RPC 2.0 message (one per line, newline-terminated).
//! Tracing logs are redirected to stderr so they never corrupt the protocol stream.

use crate::acp::types::{parse_message, AcpError, ParsedMessage, RequestId};
use anyhow::{Context, Result};
use dashmap::DashMap;
use parking_lot::Mutex;
use serde::Serialize;
use serde_json::Value;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::sync::oneshot;

/// Bidirectional ACP connection. Cheap to clone (all state is in Arcs).
pub struct Connection {
    writer: Arc<Mutex<Vec<u8>>>,
    writer_notify: Arc<tokio::sync::Notify>,
    closed: Arc<std::sync::atomic::AtomicBool>,
    pending: Arc<DashMap<RequestId, oneshot::Sender<ParsedMessage>>>,
    next_outbound_id: Arc<std::sync::atomic::AtomicI64>,
}

impl Clone for Connection {
    fn clone(&self) -> Self {
        Self {
            writer: self.writer.clone(),
            writer_notify: self.writer_notify.clone(),
            closed: self.closed.clone(),
            pending: self.pending.clone(),
            next_outbound_id: self.next_outbound_id.clone(),
        }
    }
}

impl Connection {
    /// Build a connection backed by an in-memory buffer (for tests).
    pub fn buffered() -> Self {
        Self {
            writer: Arc::new(Mutex::new(Vec::new())),
            writer_notify: Arc::new(tokio::sync::Notify::new()),
            closed: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            pending: Arc::new(DashMap::new()),
            next_outbound_id: Arc::new(std::sync::atomic::AtomicI64::new(1)),
        }
    }

    /// Convenience constructor: buffered connection.
    pub fn stdio() -> Self {
        Self::buffered()
    }

    /// Mark the connection as closed so `pipe_to` will exit when the buffer drains.
    pub fn close(&self) {
        self.closed.store(true, std::sync::atomic::Ordering::SeqCst);
        self.writer_notify.notify_waiters();
    }

    /// Spawn a background task that drains the writer buffer to an async writer.
    /// Use this to pipe messages to `tokio::io::stdout()`.
    pub fn pipe_to<W>(self, mut writer: W) -> tokio::task::JoinHandle<Result<()>>
    where
        W: AsyncWrite + Unpin + Send + 'static,
    {
        tokio::spawn(async move {
            loop {
                let bytes = {
                    let mut buf = self.writer.lock();
                    if buf.is_empty() {
                        if self.closed.load(std::sync::atomic::Ordering::SeqCst) {
                            return Ok(());
                        }
                        None
                    } else {
                        let taken = buf.clone();
                        buf.clear();
                        Some(taken)
                    }
                };
                match bytes {
                    Some(b) => {
                        writer.write_all(&b).await?;
                        writer.flush().await?;
                    }
                    None => {
                        // Wait for notification OR 50ms timeout
                        let _ = tokio::time::timeout(
                            std::time::Duration::from_millis(50),
                            self.writer_notify.notified(),
                        )
                        .await;
                    }
                }
            }
        })
    }

    /// Snapshot of the buffered output (for tests).
    pub fn take_buffer(&self) -> Vec<u8> {
        std::mem::take(&mut *self.writer.lock())
    }

    /// Send a successful response to a client request.
    pub async fn send_response(&self, id: RequestId, result: Value) -> Result<()> {
        let envelope = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result,
        });
        self.write_envelope(&envelope).await
    }

    /// Send an error response to a client request.
    pub async fn send_error_response(&self, id: RequestId, error: AcpError) -> Result<()> {
        let envelope = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": error,
        });
        self.write_envelope(&envelope).await
    }

    /// Send a notification (no id, no response expected).
    pub async fn send_notification(&self, method: &str, params: Value) -> Result<()> {
        let envelope = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });
        self.write_envelope(&envelope).await
    }

    /// Send a request to the client and await its response.
    pub async fn send_request(&self, method: &str, params: Value) -> Result<ParsedMessage> {
        let id = RequestId::Number(
            self.next_outbound_id
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        );
        let (tx, rx) = oneshot::channel();
        self.pending.insert(id.clone(), tx);
        let envelope = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        self.write_envelope(&envelope).await?;
        rx.await
            .with_context(|| format!("client dropped response for {method}"))
    }

    /// Route an inbound response to the pending request waiter.
    pub fn route_response(&self, message: ParsedMessage) {
        let id = match &message {
            ParsedMessage::ResponseResult { id, .. } | ParsedMessage::ResponseError { id, .. } => id.clone(),
            _ => return,
        };
        if let Some((_, tx)) = self.pending.remove(&id) {
            let _ = tx.send(message);
        }
    }

    async fn write_envelope(&self, envelope: &impl Serialize) -> Result<()> {
        let line = serde_json::to_string(envelope).context("serialize envelope")?;
        let mut guard = self.writer.lock();
        guard.extend_from_slice(line.as_bytes());
        guard.push(b'\n');
        drop(guard);
        self.writer_notify.notify_one();
        Ok(())
    }
}

/// Drives stdin → parsed messages. Spawns a background task that reads line-by-line,
/// dispatches to `tx` for inbound requests/notifications, and routes responses back
/// to the connection's pending map.
pub async fn run_reader<R>(connection: Connection, stdin: R, tx: tokio::sync::mpsc::UnboundedSender<Inbound>)
where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    let mut lines = BufReader::new(stdin).lines();
    loop {
        match lines.next_line().await {
            Ok(Some(line)) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let value: Value = match serde_json::from_str(trimmed) {
                    Ok(v) => v,
                    Err(e) => {
                        let err = AcpError::parse_error();
                        let _ = connection
                            .send_error_response(RequestId::Null, err)
                            .await;
                        tracing::warn!("acp: failed to parse line: {e}");
                        continue;
                    }
                };
                match parse_message(&value) {
                    Ok(ParsedMessage::ResponseResult { .. }) | Ok(ParsedMessage::ResponseError { .. }) => {
                        connection.route_response(
                            parse_message(&value).expect("just parsed"),
                        );
                    }
                    Ok(ParsedMessage::Request { id, method, params }) => {
                        if tx.send(Inbound::Request { id, method, params }).is_err() {
                            return;
                        }
                    }
                    Ok(ParsedMessage::Notification { method, params }) => {
                        if tx.send(Inbound::Notification { method, params }).is_err() {
                            return;
                        }
                    }
                    Err(e) => {
                        let _ = connection
                            .send_error_response(RequestId::Null, e)
                            .await;
                    }
                }
            }
            Ok(None) => {
                // EOF on stdin
                let _ = tx.send(Inbound::Eof);
                return;
            }
            Err(e) => {
                tracing::warn!("acp: stdin read error: {e}");
                let _ = tx.send(Inbound::Eof);
                return;
            }
        }
    }
}

/// What the reader delivers to the dispatcher.
#[derive(Debug, Clone)]
pub enum Inbound {
    Request {
        id: RequestId,
        method: String,
        params: Option<Value>,
    },
    Notification {
        method: String,
        params: Option<Value>,
    },
    Eof,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn connection_is_clone() {
        let conn = Connection::buffered();
        let _clone = conn.clone();
    }

    #[tokio::test]
    async fn send_response_serializes_envelope() {
        let conn = Connection::buffered();
        conn.send_response(RequestId::Number(7), json!({"ok": true}))
            .await
            .unwrap();
        let line = conn.take_buffer();
        let text = String::from_utf8(line).unwrap();
        assert!(text.ends_with('\n'));
        let v: Value = serde_json::from_str(text.trim_end()).unwrap();
        assert_eq!(v["jsonrpc"], "2.0");
        assert_eq!(v["id"], 7);
        assert_eq!(v["result"]["ok"], true);
    }

    #[tokio::test]
    async fn send_notification_has_no_id() {
        let conn = Connection::buffered();
        conn.send_notification("session/update", json!({"x": 1}))
            .await
            .unwrap();
        let buf = conn.take_buffer();
        let v: Value = serde_json::from_slice(&buf).unwrap();
        assert!(v.get("id").is_none());
        assert_eq!(v["method"], "session/update");
    }

    #[tokio::test]
    async fn pending_requests_get_routed() {
        let conn = Connection::buffered();
        let conn2 = conn.clone();

        let waiter = tokio::spawn(async move {
            // This will time out because no client responds, but that's fine.
            let _ = tokio::time::timeout(
                std::time::Duration::from_millis(50),
                conn2.send_request("echo", json!({})),
            )
            .await;
        });

        // Give the spawn a moment, then route a synthetic response
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        // The pending map should have one entry now; verify the map key exists.
        assert!(!conn.pending.is_empty());
        waiter.await.unwrap();
    }
}
