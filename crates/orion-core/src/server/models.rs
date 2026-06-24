use crate::server::AppState;
use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct ModelQuery {
    pub provider: Option<String>,
    pub free: Option<bool>,
    pub vision: Option<bool>,
    pub tools: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct ModelView {
    pub full_id: String,
    pub provider_id: String,
    pub model_id: String,
    pub display_name: String,
    pub context_window: Option<u32>,
    pub input_price: Option<f64>,
    pub output_price: Option<f64>,
    pub is_free: bool,
    pub is_local: bool,
    pub supports_vision: bool,
    pub supports_tools: bool,
    pub supports_reasoning: bool,
}

pub async fn list(
    State(state): State<AppState>,
    Query(q): Query<ModelQuery>,
) -> Json<Vec<ModelView>> {
    let models = state.catalog.list_models(q.provider.as_deref());

    let filtered: Vec<_> = models
        .into_iter()
        .filter(|m| q.free.map(|f| m.is_free == f).unwrap_or(true))
        .filter(|m| q.vision.map(|v| m.supports_vision == v).unwrap_or(true))
        .filter(|m| q.tools.map(|t| m.supports_tools == t).unwrap_or(true))
        .map(|m| ModelView {
            full_id: m.full_id(),
            provider_id: m.provider_id,
            model_id: m.model_id,
            display_name: m.display_name,
            context_window: m.context_window,
            input_price: m.input_price,
            output_price: m.output_price,
            is_free: m.is_free,
            is_local: m.is_local,
            supports_vision: m.supports_vision,
            supports_tools: m.supports_tools,
            supports_reasoning: m.supports_reasoning,
        })
        .collect();

    Json(filtered)
}
