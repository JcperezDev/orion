use anyhow::Result;
use orion_core::{
    build_router,
    memory::MemoryStore,
    middleware::TokenOptimizer,
    models::ModelCatalog,
    providers::ProviderRegistry,
    server::AppState,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let catalog = Arc::new(ModelCatalog::new()?);
    let registry = Arc::new(ProviderRegistry::new(catalog.clone()));
    registry.load_from_catalog();

    let memory = Arc::new(MemoryStore::new()?);
    let token_optimizer = Arc::new(TokenOptimizer::new()?);

    let state = AppState {
        registry,
        catalog,
        memory,
        token_optimizer,
    };

    let app = build_router(state);

    let port: u16 = std::env::var("ORION_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(7337);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    tracing::info!("orion-server listening on http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
