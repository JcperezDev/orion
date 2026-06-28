//! Agent Client Protocol (ACP) — JSON-RPC 2.0 over stdio for editor integration.
//!
//! ACP lets editor integrations (Zed, Neovim via ACP plugins, JetBrains, etc.) drive
//! ORION as a subprocess. The protocol is line-delimited JSON-RPC 2.0 on stdin/stdout;
//! tracing goes to stderr so it never corrupts the protocol stream.
//!
//! See `https://agentclientprotocol.com` for the canonical spec.

pub mod connection;
pub mod server;
pub mod sessions;
pub mod types;

pub use connection::Connection;
pub use server::{run_acp_server, AcpAgentBuilder, AcpServer, PromptHandler, StopReasonValue};
pub use sessions::{SessionRegistry, SessionState};
pub use types::{
    AcpError, AgentCapabilities, ClientCapabilities, ContentChunk, ContentBlock, Implementation,
    McpCapabilities, PendingPermission, PermissionOption, PermissionOptionId,
    PermissionOptionKind, PromptCapabilities, RequestId, RequestPermissionOutcome, SessionId,
    SessionUpdate, StopReason, ToolCall, ToolCallContent, ToolCallId, ToolCallStatus,
    ToolCallUpdate, ToolCallUpdateFields, ToolKind,
};
