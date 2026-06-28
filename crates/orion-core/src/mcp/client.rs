use crate::mcp::protocol::{
    CallToolParams, CallToolResult, ClientInfo, InitializeParams, InitializeResult,
    JsonRpcNotification, JsonRpcRequest, JsonRpcResponse, ListToolsResult, Tool,
    PROTOCOL_VERSION,
};
use anyhow::{Context, Result};
use parking_lot::Mutex;
use serde_json::Value;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;

pub struct McpClient {
    name: String,
    child: Child,
    stdin_tx: mpsc::UnboundedSender<String>,
    pending: mpsc::UnboundedReceiver<JsonRpcResponse>,
    next_id: AtomicU64,
    initialized: bool,
    pub tools: Vec<Tool>,
}

impl McpClient {
    pub async fn spawn(name: &str, command: &str, args: &[String], env: &[(&str, &str)]) -> Result<Self> {
        let mut cmd = Command::new(command);
        cmd.args(args);
        for (k, v) in env {
            cmd.env(k, v);
        }
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd.spawn().with_context(|| format!("spawn mcp server: {command}"))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow::anyhow!("no stdin"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("no stdout"))?;

        let (tx, mut rx_writer) = mpsc::unbounded_channel::<String>();
        tokio::spawn(async move {
            let mut w = stdin;
            while let Some(line) = rx_writer.recv().await {
                if w.write_all(line.as_bytes()).await.is_err() {
                    break;
                }
                if w.write_all(b"\n").await.is_err() {
                    break;
                }
                if w.flush().await.is_err() {
                    break;
                }
            }
        });

        let (resp_tx, resp_rx) = mpsc::unbounded_channel::<JsonRpcResponse>();
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) => break,
                    Ok(_) => {}
                    Err(_) => break,
                }
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if let Ok(resp) = serde_json::from_str::<JsonRpcResponse>(trimmed) {
                    let _ = resp_tx.send(resp);
                } else if let Ok(notif) = serde_json::from_str::<JsonRpcNotification>(trimmed) {
                    tracing::debug!(method = %notif.method, "mcp notification ignored");
                }
            }
        });

        let mut client = McpClient {
            name: name.to_string(),
            child,
            stdin_tx: tx,
            pending: resp_rx,
            next_id: AtomicU64::new(1),
            initialized: false,
            tools: vec![],
        };
        client.initialize().await?;
        Ok(client)
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub async fn list_tools(&mut self) -> Result<Vec<Tool>> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id,
            method: "tools/list".into(),
            params: None,
        };
        let resp: ListToolsResult = self.call(req).await?;
        self.tools = resp.tools.clone();
        Ok(resp.tools)
    }

    pub async fn call_tool(&mut self, name: &str, args: Option<Value>) -> Result<CallToolResult> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id,
            method: "tools/call".into(),
            params: Some(serde_json::to_value(CallToolParams {
                name: name.to_string(),
                arguments: args,
            })?),
        };
        self.call(req).await
    }

    async fn initialize(&mut self) -> Result<()> {
        let params = InitializeParams {
            protocol_version: PROTOCOL_VERSION.into(),
            capabilities: serde_json::json!({}),
            client_info: ClientInfo {
                name: "orion".into(),
                version: env!("CARGO_PKG_VERSION").into(),
            },
        };
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id,
            method: "initialize".into(),
            params: Some(serde_json::to_value(params)?),
        };
        let _: InitializeResult = self.call(req).await?;

        let note = JsonRpcNotification {
            jsonrpc: "2.0".into(),
            method: "notifications/initialized".into(),
            params: None,
        };
        let raw = serde_json::to_string(&note)?;
        let _ = self.stdin_tx.send(raw);

        self.initialized = true;
        Ok(())
    }

    async fn call<T: serde::de::DeserializeOwned>(&mut self, req: JsonRpcRequest) -> Result<T> {
        let id = req.id;
        let raw = serde_json::to_string(&req)?;
        self.stdin_tx
            .send(raw)
            .map_err(|_| anyhow::anyhow!("mcp stdin closed"))?;

        loop {
            match self.pending.recv().await {
                Some(resp) if resp.id == id => {
                    if let Some(err) = resp.error {
                        anyhow::bail!(
                            "mcp rpc error {}: {}",
                            err.code,
                            err.message
                        );
                    }
                    let val = resp.result.unwrap_or(Value::Null);
                    return Ok(serde_json::from_value(val)?);
                }
                Some(_) => continue,
                None => {
                    anyhow::bail!("mcp server {} closed", self.name)
                }
            }
        }
    }

    pub async fn shutdown(mut self) -> Result<()> {
        let _ = self.child.kill().await;
        let _ = self.child.wait().await;
        Ok(())
    }
}

pub struct McpRegistry {
    clients: Mutex<Vec<McpClient>>,
}

impl McpRegistry {
    pub fn new() -> Self {
        Self {
            clients: Mutex::new(Vec::new()),
        }
    }

    pub async fn spawn(
        &self,
        name: &str,
        command: &str,
        args: &[String],
        env: &[(&str, &str)],
    ) -> Result<Vec<Tool>> {
        let client = McpClient::spawn(name, command, args, env).await?;
        let tools = client.tools.clone();
        self.clients.lock().push(client);
        Ok(tools)
    }

    pub fn names(&self) -> Vec<String> {
        self.clients
            .lock()
            .iter()
            .map(|c| c.name().to_string())
            .collect()
    }

    pub async fn call(&self, server: &str, tool: &str, args: Option<Value>) -> Result<CallToolResult> {
        for c in self.clients.lock().iter_mut() {
            if c.name() == server {
                return c.call_tool(tool, args).await;
            }
        }
        anyhow::bail!("mcp server not found: {server}")
    }
}

impl Default for McpRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn missing_command_errors() {
        let r = McpClient::spawn(
            "missing",
            "/nonexistent/command/path/should/never/exist",
            &[],
            &[],
        )
        .await;
        assert!(r.is_err());
    }
}
