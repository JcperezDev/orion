//! ACP request/notification dispatcher. Routes JSON-RPC 2.0 messages to the right handler.

use crate::acp::connection::{Connection, Inbound};
use crate::acp::sessions::{SessionRegistry, SessionState};
use crate::acp::types::{
    AcpError, AgentCapabilities, ClientCapabilities, ContentBlock, ContentChunk, Implementation,
    RequestId, SessionId, SessionUpdate,
};
use crate::config::Config;
use crate::core::agent::Agent;
use anyhow::Result;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;

/// Long-lived ACP server. Built once at startup, dispatches messages until EOF.
pub struct AcpServer {
    pub connection: Connection,
    pub sessions: SessionRegistry,
    pub config: Arc<Config>,
    /// Cached initialize response (sent back to client on `initialize`).
    pub agent_capabilities: AgentCapabilities,
    /// Pending session creation params waiting for the agent to be built.
    pub pending_initializes: parking_lot::Mutex<Vec<InitializeParams>>,
    /// Optional agent builder hook. Set after `initialize`, called on `session/new`.
    pub agent_builder: Option<Arc<dyn AcpAgentBuilder>>,
    /// Server-side handlers for `session/prompt`.
    pub prompt_handlers: Option<Arc<dyn PromptHandler>>,
    /// Capabilities advertised by the client.
    pub client_capabilities: parking_lot::Mutex<Option<ClientCapabilities>>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct InitializeParams {
    #[serde(default = "default_protocol_version")]
    pub protocol_version: String,
    #[serde(default)]
    pub client_capabilities: ClientCapabilities,
}

fn default_protocol_version() -> String {
    "v1".into()
}

/// Trait for spawning ORION agents from the ACP server.
#[async_trait::async_trait]
pub trait AcpAgentBuilder: Send + Sync {
    async fn build(&self, cwd: &std::path::PathBuf) -> Result<Arc<Agent>>;
}

/// Trait for handling `session/prompt` requests.
#[async_trait::async_trait]
pub trait PromptHandler: Send + Sync {
    async fn handle(
        &self,
        session: Arc<SessionState>,
        prompt: String,
        connection: Connection,
    ) -> Result<StopReasonValue>;
}

/// A simple stop reason result.
#[derive(Debug, Clone, Copy)]
pub struct StopReasonValue(pub crate::acp::types::StopReason);

impl From<crate::acp::types::StopReason> for StopReasonValue {
    fn from(s: crate::acp::types::StopReason) -> Self {
        Self(s)
    }
}

impl AcpServer {
    pub fn new(connection: Connection, config: Arc<Config>) -> Arc<Self> {
        Arc::new(Self {
            connection,
            sessions: SessionRegistry::new(),
            config,
            agent_capabilities: AgentCapabilities::default(),
            pending_initializes: parking_lot::Mutex::new(Vec::new()),
            agent_builder: None,
            prompt_handlers: None,
            client_capabilities: parking_lot::Mutex::new(None),
        })
    }

    pub fn with_agent_builder(self: Arc<Self>, builder: Arc<dyn AcpAgentBuilder>) -> Arc<Self> {
        match Arc::try_unwrap(self) {
            Ok(mut me) => {
                me.agent_builder = Some(builder);
                Arc::new(me)
            }
            Err(me) => Arc::new(AcpServer {
                connection: me.connection.clone(),
                sessions: me.sessions.clone(),
                config: me.config.clone(),
                agent_capabilities: me.agent_capabilities.clone(),
                pending_initializes: parking_lot::Mutex::new(
                    me.pending_initializes.lock().clone(),
                ),
                agent_builder: Some(builder),
                prompt_handlers: me.prompt_handlers.clone(),
                client_capabilities: parking_lot::Mutex::new(
                    me.client_capabilities.lock().clone(),
                ),
            }),
        }
    }

    pub fn with_prompt_handler(self: Arc<Self>, handler: Arc<dyn PromptHandler>) -> Arc<Self> {
        match Arc::try_unwrap(self) {
            Ok(mut me) => {
                me.prompt_handlers = Some(handler);
                Arc::new(me)
            }
            Err(me) => {
                let mut new = AcpServer {
                    connection: me.connection.clone(),
                    sessions: me.sessions.clone(),
                    config: me.config.clone(),
                    agent_capabilities: me.agent_capabilities.clone(),
                    pending_initializes: parking_lot::Mutex::new(
                        me.pending_initializes.lock().clone(),
                    ),
                    agent_builder: me.agent_builder.clone(),
                    prompt_handlers: Some(handler.clone()),
                    client_capabilities: parking_lot::Mutex::new(
                        me.client_capabilities.lock().clone(),
                    ),
                };
                new.prompt_handlers = Some(handler);
                Arc::new(new)
            }
        }
    }

    /// Drive the dispatcher loop. Reads from `rx` (fed by `run_reader`), handles
    /// each message, awaits outstanding handlers before returning on EOF.
    pub async fn run(
        self: Arc<Self>,
        mut rx: tokio::sync::mpsc::UnboundedReceiver<Inbound>,
    ) -> Result<()> {
        let mut joins: Vec<tokio::task::JoinHandle<()>> = Vec::new();
        while let Some(msg) = rx.recv().await {
            match msg {
                Inbound::Request { id, method, params } => {
                    let me = self.clone();
                    joins.push(tokio::spawn(async move {
                        let params = params.unwrap_or(Value::Null);
                        me.handle_request(id, &method, params).await;
                    }));
                }
                Inbound::Notification { method, params } => {
                    let me = self.clone();
                    joins.push(tokio::spawn(async move {
                        let params = params.unwrap_or(Value::Null);
                        me.handle_notification(&method, params).await;
                    }));
                }
                Inbound::Eof => break,
            }
        }
        for j in joins {
            let _ = j.await;
        }
        Ok(())
    }

    async fn handle_request(
        self: Arc<Self>,
        id: RequestId,
        method: &str,
        params: Value,
    ) {
        let connection = self.connection.clone();
        let result = match method {
            "initialize" => self.on_initialize(id.clone(), params).await,
            "authenticate" => self.on_authenticate(),
            "session/new" => self.on_new_session(id.clone(), params).await,
            "session/load" => self.on_session_load(id.clone()),
            "session/prompt" => self.on_prompt(id.clone(), params).await,
            _ => Err(AcpError::method_not_found()),
        };
        match result {
            Ok(value) => {
                if let Err(e) = connection.send_response(id, value).await {
                    tracing::warn!("acp: failed to send response: {e}");
                }
            }
            Err(e) => {
                if let Err(er) = connection.send_error_response(id, e).await {
                    tracing::warn!("acp: failed to send error: {er}");
                }
            }
        }
    }

    async fn handle_notification(self: Arc<Self>, method: &str, _params: Value) {
        match method {
            "session/cancel" => {
                // We don't track which session cancelled in v1; cancel all.
                for sid in self.sessions.list() {
                    if let Some(state) = self.sessions.get(&sid) {
                        state.cancel();
                    }
                }
            }
            _ => {
                tracing::debug!("acp: ignoring unknown notification {method}");
            }
        }
    }

    async fn on_initialize(self: Arc<Self>, _id: RequestId, params: Value) -> Result<Value, AcpError> {
        let parsed: InitializeParams = serde_json::from_value(params)
            .map_err(|e| AcpError::invalid_params(format!("initialize: {e}")))?;
        *self.client_capabilities.lock() = Some(parsed.client_capabilities.clone());
        let impl_ = Implementation::orion();
        Ok(json!({
            "protocolVersion": parsed.protocol_version,
            "agentCapabilities": self.agent_capabilities,
            "agentInfo": impl_,
            "authMethods": [],
        }))
    }

    fn on_authenticate(&self) -> Result<Value, AcpError> {
        Ok(json!({}))
    }

    async fn on_new_session(self: Arc<Self>, _id: RequestId, params: Value) -> Result<Value, AcpError> {
        let cwd_value = params
            .get("cwd")
            .and_then(|v| v.as_str())
            .unwrap_or(".");
        let cwd = PathBuf::from(cwd_value);
        if !cwd.exists() {
            return Err(AcpError::invalid_params(format!("cwd {cwd:?} does not exist")));
        }
        let session_id = SessionId::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let state = SessionState::new(session_id.clone(), cwd.clone(), tx);
        self.sessions.insert(state.clone());
        tracing::info!("acp: created session {} in {}", session_id, cwd.display());
        Ok(json!({
            "sessionId": session_id,
            "mcpServers": [],
        }))
    }

    fn on_session_load(&self, _id: RequestId) -> Result<Value, AcpError> {
        Err(AcpError::method_not_found())
    }

    async fn on_prompt(self: Arc<Self>, _id: RequestId, params: Value) -> Result<Value, AcpError> {
        let session_id_value = params
            .get("sessionId")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AcpError::invalid_params("sessionId missing"))?;
        let session_id = SessionId(session_id_value.to_string());
        let session = self
            .sessions
            .get(&session_id)
            .ok_or_else(|| AcpError::invalid_params(format!("unknown session {session_id}")))?;
        let prompt = params
            .get("prompt")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|block| block.get("text").and_then(|t| t.as_str()))
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default();

        if let Some(handler) = &self.prompt_handlers {
            match handler.handle(session.clone(), prompt, self.connection.clone()).await {
                Ok(stop) => Ok(json!({"stopReason": stop_reason_str(stop.0)})),
                Err(e) => Err(AcpError::internal_error(format!("{:#}", e))),
            }
        } else {
            // No prompt handler installed — fall back to acknowledging.
            let update = SessionUpdate::AgentMessageChunk(ContentChunk {
                content: ContentBlock::Text {
                    text: format!("[acp stub] received prompt: {}", truncate(&prompt, 80)),
                },
            });
            session.inbound_tx.send(update).ok();
            Ok(json!({"stopReason": "end_turn"}))
        }
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

fn stop_reason_str(s: crate::acp::types::StopReason) -> &'static str {
    match s {
        crate::acp::types::StopReason::EndTurn => "end_turn",
        crate::acp::types::StopReason::MaxTokens => "max_tokens",
        crate::acp::types::StopReason::Cancelled => "cancelled",
        crate::acp::types::StopReason::MaxTurnRequests => "max_turn_requests",
        crate::acp::types::StopReason::Refusal => "refusal",
    }
}

/// Entry point: install stderr tracing, build the connection, run the dispatcher
/// until stdin closes.
pub async fn run_acp_server(config: Arc<Config>) -> Result<()> {
    install_stderr_tracing();
    let connection = Connection::stdio();
    let server = AcpServer::new(connection.clone(), config);
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

    // Pipe the buffered connection to stdout in a background task.
    let writer_handle = connection.clone().pipe_to(tokio::io::stdout());

    let reader_conn = connection.clone();
    let reader_handle = tokio::spawn(async move {
        let stdin = tokio::io::stdin();
        crate::acp::connection::run_reader(reader_conn, stdin, tx).await;
    });
    server.run(rx).await?;
    reader_handle.abort();
    connection.close();
    let _ = writer_handle.await;
    Ok(())
}

fn install_stderr_tracing() {
    use tracing_subscriber::EnvFilter;
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_env("CLAURST_ACP_LOG")
                .or_else(|_| EnvFilter::try_from_default_env())
                .unwrap_or_else(|_| EnvFilter::new("warn")),
        )
        .with_writer(std::io::stderr)
        .try_init();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn server_handles_initialize() {
        let conn = Connection::buffered();
        let server = AcpServer::new(conn.clone(), Arc::new(Config::default()));
        let (id, _method, params) = (
            RequestId::Number(1),
            "initialize".to_string(),
            json!({"protocolVersion": "v1"}),
        );
        let value = server
            .clone()
            .on_initialize(id.clone(), params)
            .await
            .unwrap();
        assert_eq!(value["protocolVersion"], "v1");
        assert_eq!(value["agentInfo"]["name"], "orion");
    }

    #[tokio::test]
    async fn server_rejects_unknown_method() {
        let conn = Connection::buffered();
        let server = AcpServer::new(conn, Arc::new(Config::default()));
        let params = json!({"sessionId": "abc", "prompt": [{"type": "text", "text": "hi"}]});
        let res = server
            .clone()
            .on_prompt(RequestId::Number(2), params)
            .await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn server_creates_session_and_handles_prompt() {
        let conn = Connection::buffered();
        let server = AcpServer::new(conn, Arc::new(Config::default()));
        let cwd = std::env::current_dir().unwrap();
        let new = server
            .clone()
            .on_new_session(RequestId::Number(1), json!({"cwd": cwd.to_string_lossy()}))
            .await
            .unwrap();
        let session_id = new["sessionId"].as_str().unwrap().to_string();
        // No prompt handler installed → fallback path runs.
        let resp = server
            .clone()
            .on_prompt(
                RequestId::Number(2),
                json!({"sessionId": session_id, "prompt": [{"type": "text", "text": "hello"}]}),
            )
            .await
            .unwrap();
        assert_eq!(resp["stopReason"], "end_turn");
    }

    #[tokio::test]
    async fn server_rejects_missing_cwd() {
        let conn = Connection::buffered();
        let server = AcpServer::new(conn, Arc::new(Config::default()));
        let res = server
            .on_new_session(
                RequestId::Number(1),
                json!({"cwd": "/this/path/should/never/exist/orion-test-12345"}),
            )
            .await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn parse_round_trip() {
        let v = json!({"jsonrpc": "2.0", "id": 1, "method": "x"});
        let msg = crate::acp::types::parse_message(&v).unwrap();
        match msg {
            crate::acp::types::ParsedMessage::Request { id, method, .. } => {
                assert_eq!(id, RequestId::Number(1));
                assert_eq!(method, "x");
            }
            _ => panic!(),
        }
    }
}
