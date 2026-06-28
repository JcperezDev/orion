pub mod chat;
pub mod context;
pub mod mcp;
pub mod models;
pub mod providers;
pub mod sessions;
pub mod settings;
pub mod web;

use crate::memory::MemoryStore;
use crate::middleware::TokenOptimizer;
use crate::models::ModelCatalog;
use crate::providers::ProviderRegistry;
use axum::{routing::get, routing::post, Router};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

#[derive(Clone)]
pub struct AppState {
    pub registry: Arc<ProviderRegistry>,
    pub catalog: Arc<ModelCatalog>,
    pub memory: Arc<MemoryStore>,
    pub token_optimizer: Arc<TokenOptimizer>,
}

pub fn build_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/", get(web::index))
        .route("/health", get(health))
        .route("/api/health", get(web::health_json))
        .route("/api/info", get(web::info))
        .route("/api/chat", post(chat::post_chat))
        .route("/api/stream/:session_id", get(chat::get_stream))
        .route("/api/sessions", get(sessions::list).post(sessions::create))
        .route(
            "/api/sessions/:id",
            get(sessions::get).delete(sessions::delete),
        )
        .route(
            "/api/sessions/:id/fork",
            axum::routing::post(sessions::fork),
        )
        .route(
            "/api/sessions/:id/summarize",
            axum::routing::post(sessions::summarize),
        )
        .route("/api/providers", get(providers::list))
        .route("/api/models", get(models::list))
        .route("/api/settings", get(settings::get).put(settings::put))
        .route(
            "/api/mcp/servers",
            get(mcp::list_servers).post(mcp::add_server),
        )
        .route("/api/context", get(context::get_context))
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(cors)
}

async fn health() -> &'static str {
    "orion-server ok"
}
