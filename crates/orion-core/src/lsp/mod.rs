//! LSP (Language Server Protocol) integration.
//!
//! Provides a thin abstraction over LSP so the `lsp` tool can spawn language
//! servers, send requests, and surface diagnostics to the agent. The client
//! speaks JSON-RPC 2.0 over stdio (the LSP wire format).
//!
//! This module is intentionally minimal — it implements only the actions that
//! ORION needs to be useful (hover, definition, references, symbols,
//! diagnostics) and supports the `lsp-types` crate for message types.

pub mod client;
pub mod manager;

pub use client::{LspClient, LspRequest, LspResponse};
pub use manager::LspServerConfig;
