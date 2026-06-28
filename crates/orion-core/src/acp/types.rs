//! ACP protocol types — modeled after the Agent Client Protocol v1.
//!
//! These types are intentionally minimal: just what ORION emits and consumes.
//! They are serde-compatible with the ACP JSON spec.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// JSON-RPC 2.0 error object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl AcpError {
    pub fn parse_error() -> Self {
        Self {
            code: -32700,
            message: "Parse error".into(),
            data: None,
        }
    }
    pub fn invalid_request() -> Self {
        Self {
            code: -32600,
            message: "Invalid request".into(),
            data: None,
        }
    }
    pub fn method_not_found() -> Self {
        Self {
            code: -32601,
            message: "Method not found".into(),
            data: None,
        }
    }
    pub fn invalid_params(msg: impl Into<String>) -> Self {
        Self {
            code: -32602,
            message: msg.into(),
            data: None,
        }
    }
    pub fn internal_error(msg: impl Into<String>) -> Self {
        Self {
            code: -32603,
            message: msg.into(),
            data: None,
        }
    }
}

/// JSON-RPC 2.0 request id (string, number, or null).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum RequestId {
    Null,
    Number(i64),
    Str(String),
}

impl From<i64> for RequestId {
    fn from(v: i64) -> Self {
        Self::Number(v)
    }
}
impl From<String> for RequestId {
    fn from(v: String) -> Self {
        Self::Str(v)
    }
}
impl From<&str> for RequestId {
    fn from(v: &str) -> Self {
        Self::Str(v.to_string())
    }
}

/// Agent implementation metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Implementation {
    pub name: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

impl Implementation {
    pub fn orion() -> Self {
        Self {
            name: "orion".into(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            title: Some("ORION".into()),
        }
    }
}

/// What the client (editor) can do.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClientCapabilities {
    #[serde(default)]
    pub fs: FsCapabilities,
    #[serde(default)]
    pub terminal: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FsCapabilities {
    #[serde(default)]
    pub read_text_file: bool,
    #[serde(default)]
    pub write_text_file: bool,
}

/// What the agent (ORION) advertises it supports.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCapabilities {
    #[serde(default = "default_true")]
    pub load_session: bool,
    #[serde(default)]
    pub prompt_capabilities: PromptCapabilities,
    #[serde(default)]
    pub mcp_capabilities: McpCapabilities,
}

fn default_true() -> bool {
    true
}

impl Default for AgentCapabilities {
    fn default() -> Self {
        Self {
            load_session: false,
            prompt_capabilities: PromptCapabilities::default(),
            mcp_capabilities: McpCapabilities::default(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PromptCapabilities {
    #[serde(default)]
    pub image: bool,
    #[serde(default)]
    pub audio: bool,
    #[serde(default)]
    pub embedded_context: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct McpCapabilities {
    #[serde(default)]
    pub http: bool,
    #[serde(default)]
    pub sse: bool,
}

/// Stable session identifier (UUID string).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct SessionId(pub String);

impl SessionId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}
impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}
impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Stable tool call identifier.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct ToolCallId(pub String);

impl ToolCallId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}
impl Default for ToolCallId {
    fn default() -> Self {
        Self::new()
    }
}

/// Classification of what a tool does (for editor UI rendering).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolKind {
    Read,
    Edit,
    Execute,
    Fetch,
    Search,
    Delete,
    Move,
    Think,
    Other,
}

/// Lifecycle status of a tool call.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolCallStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

/// A piece of content shown in the chat (text/image/etc).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text { text: String },
    Image { data: String, mime_type: String },
}

/// A streaming content chunk (used in `session/update` notifications).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentChunk {
    pub content: ContentBlock,
}

/// Tool call descriptor emitted when a tool starts running.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: ToolCallId,
    pub title: String,
    pub kind: ToolKind,
    pub status: ToolCallStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_input: Option<Value>,
}

/// Partial update to a tool call (status, output).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolCallUpdateFields {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<ToolCallStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Vec<ToolCallContent>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_output: Option<Value>,
}

/// Wrapper for a tool call update (sent as `session/update` notification).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallUpdate {
    pub id: ToolCallId,
    #[serde(flatten)]
    pub fields: ToolCallUpdateFields,
}

/// Content shown in a tool call (output preview).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolCallContent {
    Text { text: String },
    Diff { path: String, old_text: String, new_text: String },
}

/// `session/update` notification body.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SessionUpdate {
    /// Streamed message chunk from the assistant.
    AgentMessageChunk(ContentChunk),
    /// Streamed "thinking" chunk (chain-of-thought).
    AgentThoughtChunk(ContentChunk),
    /// Tool call started.
    ToolCall(ToolCall),
    /// Tool call status/output update.
    ToolCallUpdate(ToolCallUpdate),
}

/// Stop reason for the assistant's turn.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    EndTurn,
    MaxTokens,
    Cancelled,
    MaxTurnRequests,
    Refusal,
}

/// User choice for a permission request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct PermissionOptionId(pub String);

impl PermissionOptionId {
    pub fn allow_once() -> Self {
        Self("allow_once".into())
    }
    pub fn allow_always() -> Self {
        Self("allow_always".into())
    }
    pub fn reject_once() -> Self {
        Self("reject_once".into())
    }
}

/// Kind of permission option (affects editor rendering).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PermissionOptionKind {
    AllowOnce,
    AllowAlways,
    RejectOnce,
    RejectAlways,
}

/// One button on a permission request dialog.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionOption {
    pub id: PermissionOptionId,
    pub label: String,
    pub kind: PermissionOptionKind,
}

/// What the user picked (or cancelled).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum RequestPermissionOutcome {
    Selected { option_id: PermissionOptionId },
    Cancelled,
}

/// A pending permission request awaiting the user's choice.
pub struct PendingPermission {
    pub tool_call_id: ToolCallId,
    pub title: String,
    pub kind: ToolKind,
    pub options: Vec<PermissionOption>,
    /// Notifier wakes up the requester when the outcome is set.
    pub outcome: std::sync::Arc<parking_lot::Mutex<Option<RequestPermissionOutcome>>>,
    pub notify: Arc<tokio::sync::Notify>,
}

/// Helper to build the full session/update notification envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionNotification {
    pub session_id: SessionId,
    pub update: SessionUpdate,
}

/// Parse a JSON-RPC 2.0 message from a raw JSON value.
pub fn parse_message(value: &Value) -> Result<ParsedMessage, AcpError> {
    let obj = value.as_object().ok_or_else(AcpError::invalid_request)?;
    let jsonrpc = obj.get("jsonrpc").and_then(|v| v.as_str()).unwrap_or("");
    if jsonrpc != "2.0" {
        return Err(AcpError::invalid_request());
    }

    if let Some(method) = obj.get("method").and_then(|v| v.as_str()) {
        let id = obj.get("id").cloned().map(request_id_from_value);
        let params = obj.get("params").cloned();
        if let Some(id) = id {
            Ok(ParsedMessage::Request {
                id: id.unwrap_or(RequestId::Null),
                method: method.to_string(),
                params,
            })
        } else {
            Ok(ParsedMessage::Notification {
                method: method.to_string(),
                params,
            })
        }
    } else if let Some(id) = obj.get("id") {
        let id = request_id_from_value(id.clone()).unwrap_or(RequestId::Null);
        if let Some(error) = obj.get("error") {
            Ok(ParsedMessage::ResponseError {
                id,
                error: serde_json::from_value(error.clone()).map_err(|_| AcpError::internal_error("malformed error"))?,
            })
        } else if let Some(result) = obj.get("result") {
            Ok(ParsedMessage::ResponseResult {
                id,
                result: result.clone(),
            })
        } else {
            Err(AcpError::invalid_request())
        }
    } else {
        Err(AcpError::invalid_request())
    }
}

fn request_id_from_value(v: Value) -> Option<RequestId> {
    match v {
        Value::Null => Some(RequestId::Null),
        Value::Number(n) => n.as_i64().map(RequestId::Number),
        Value::String(s) => Some(RequestId::Str(s)),
        _ => None,
    }
}

/// Parsed JSON-RPC 2.0 message variants.
#[derive(Debug, Clone)]
pub enum ParsedMessage {
    Request {
        id: RequestId,
        method: String,
        params: Option<Value>,
    },
    Notification {
        method: String,
        params: Option<Value>,
    },
    ResponseResult {
        id: RequestId,
        result: Value,
    },
    ResponseError {
        id: RequestId,
        error: AcpError,
    },
}

/// Extra metadata exposed to JSON-RPC clients (e.g. method names ORION handles).
pub struct AcpMethodCatalog {
    pub methods: HashMap<&'static str, &'static str>,
}

impl AcpMethodCatalog {
    pub fn orion() -> Self {
        let mut methods = HashMap::new();
        methods.insert("initialize", "Handshake — exchange capabilities");
        methods.insert("authenticate", "Optional auth (no-op for ORION)");
        methods.insert("session/new", "Create a new session");
        methods.insert("session/load", "Load a session by id (unsupported in v1)");
        methods.insert("session/prompt", "Send a prompt to the agent");
        methods.insert("session/cancel", "Cancel the in-flight prompt");
        methods.insert("session/request_permission", "Permission request from agent (agent→client)");
        Self { methods }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_request() {
        let v = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {"protocolVersion": "v1"}
        });
        match parse_message(&v).unwrap() {
            ParsedMessage::Request { id, method, params } => {
                assert_eq!(id, RequestId::Number(1));
                assert_eq!(method, "initialize");
                assert!(params.is_some());
            }
            _ => panic!("expected request"),
        }
    }

    #[test]
    fn parse_notification() {
        let v = json!({
            "jsonrpc": "2.0",
            "method": "session/cancel",
            "params": {"sessionId": "abc"}
        });
        match parse_message(&v).unwrap() {
            ParsedMessage::Notification { method, params } => {
                assert_eq!(method, "session/cancel");
                assert!(params.is_some());
            }
            _ => panic!("expected notification"),
        }
    }

    #[test]
    fn parse_response_result() {
        let v = json!({"jsonrpc": "2.0", "id": "x", "result": {"ok": true}});
        match parse_message(&v).unwrap() {
            ParsedMessage::ResponseResult { id, result } => {
                assert_eq!(id, RequestId::Str("x".into()));
                assert_eq!(result["ok"], true);
            }
            _ => panic!("expected response"),
        }
    }

    #[test]
    fn parse_response_error() {
        let v = json!({
            "jsonrpc": "2.0",
            "id": null,
            "error": {"code": -32601, "message": "Method not found"}
        });
        match parse_message(&v).unwrap() {
            ParsedMessage::ResponseError { id, error } => {
                assert_eq!(id, RequestId::Null);
                assert_eq!(error.code, -32601);
            }
            _ => panic!("expected error response"),
        }
    }

    #[test]
    fn error_codes() {
        assert_eq!(AcpError::parse_error().code, -32700);
        assert_eq!(AcpError::method_not_found().code, -32601);
        assert_eq!(AcpError::invalid_params("x").code, -32602);
        assert_eq!(AcpError::internal_error("x").code, -32603);
    }

    #[test]
    fn content_chunk_serialization() {
        let chunk = ContentChunk {
            content: ContentBlock::Text { text: "hello".into() },
        };
        let v = serde_json::to_value(&chunk).unwrap();
        assert_eq!(v["content"]["type"], "text");
        assert_eq!(v["content"]["text"], "hello");
    }

    #[test]
    fn session_update_variants() {
        let u = SessionUpdate::AgentMessageChunk(ContentChunk {
            content: ContentBlock::Text { text: "x".into() },
        });
        let v = serde_json::to_value(&u).unwrap();
        assert_eq!(v["type"], "agent_message_chunk");
    }

    #[test]
    fn tool_call_status_roundtrip() {
        for s in [
            ToolCallStatus::Pending,
            ToolCallStatus::InProgress,
            ToolCallStatus::Completed,
            ToolCallStatus::Failed,
        ] {
            let v = serde_json::to_value(s).unwrap();
            let back: ToolCallStatus = serde_json::from_value(v).unwrap();
            assert_eq!(back, s);
        }
    }

    #[test]
    fn tool_kind_classification() {
        let k = ToolKind::Execute;
        let v = serde_json::to_value(k).unwrap();
        assert_eq!(v, "execute");
    }

    #[test]
    fn permission_options() {
        let opt = PermissionOption {
            id: PermissionOptionId::allow_once(),
            label: "Allow".into(),
            kind: PermissionOptionKind::AllowOnce,
        };
        let v = serde_json::to_value(&opt).unwrap();
        assert_eq!(v["id"], "allow_once");
        assert_eq!(v["kind"], "allow_once");
    }
}
