use crate::memory::Settings;
use crate::server::AppState;
use axum::{
    extract::State,
    http::StatusCode,
    Json,
};

pub async fn get(State(state): State<AppState>) -> Json<Settings> {
    Json(state.memory.get_settings().await)
}

pub async fn put(
    State(state): State<AppState>,
    Json(s): Json<Settings>,
) -> Result<StatusCode, (StatusCode, String)> {
    state
        .memory
        .save_settings(&s)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")))?;
    Ok(StatusCode::OK)
}
