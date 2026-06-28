pub mod acp;
pub mod agents;
pub mod config;
pub mod core;
pub mod lsp;
pub mod mcp;
pub mod memory;
pub mod middleware;
pub mod models;
pub mod oauth;
pub mod permissions;
pub mod plugins;
pub mod providers;
pub mod router;
pub mod server;
pub mod share;
pub mod skills;
pub mod stats;
pub mod tools;
pub use tools::bash_parser as shell_parser;
pub use core::compactor::ContextCompactor;
pub use core::spill::SpillManager;
pub use plugins::loader::PluginLoader;
pub use plugins::{ExternalTool, PluginDescriptor, PluginDef};
pub use tools::task::TaskTool;
pub use tools::register_task_tool;

pub use config::Config;
pub use memory::{MemoryStore, Settings as MemorySettings};
pub use middleware::token_optimizer::OptimizerConfig;
pub use middleware::{TokenOptimizer, TokenStats};
pub use models::catalog::ModelCatalog;
pub use providers::ProviderRegistry;
pub use router::selector::{ModelRecommendation, TaskKind};
pub use server::{build_router, AppState};
pub use tools::{
    builtin_registry, ApprovalChannel, ApprovalRequest, ApprovalResponse,
    PermissionKind, StreamChunk, Tool, ToolCall, ToolContext, ToolDefinition, ToolRegistry,
    ToolResult,
};
pub use permissions::{
    Action as PermissionAction, PermissionConfig, PermissionEngine, Rule as PermissionRule,
};
pub use permissions::bash_risk::RiskClass;
pub use permissions::store::LearnedStore;
pub use permissions::trust::{decide as decide_permission, Decision, PathVerdict};
pub use core::dispatch::{DispatchConfig, DispatchEvent, Dispatcher};
pub use core::snapshot::{PatchEntry, SnapshotManager, StepSnapshot};
