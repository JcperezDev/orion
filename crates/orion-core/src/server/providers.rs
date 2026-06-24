use crate::server::AppState;
use axum::{extract::State, Json};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ProviderView {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub enabled: bool,
    pub available: bool,
    pub base_url: Option<String>,
    pub models: Vec<String>,
}

pub async fn list(State(state): State<AppState>) -> Json<Vec<ProviderView>> {
    let catalog_providers = state.catalog.list_providers();
    let available = state.registry.list_providers();

    let views = catalog_providers
        .into_iter()
        .map(|info| {
            let models: Vec<String> = state
                .catalog
                .list_models(Some(&info.id))
                .into_iter()
                .map(|m| m.model_id)
                .collect();
            ProviderView {
                id: info.id.clone(),
                name: info.name,
                kind: info.kind.as_str().to_string(),
                enabled: info.enabled,
                available: available.contains(&info.id),
                base_url: info.base_url,
                models,
            }
        })
        .collect();

    Json(views)
}
