pub mod analytics;
pub mod config;
pub mod core;
pub mod images;
pub mod mcp;
pub mod models;
pub mod providers;
pub mod router;

pub use config::Config;
pub use models::catalog::ModelCatalog;
pub use providers::ProviderRegistry;
pub use router::selector::{ModelRecommendation, TaskKind};
