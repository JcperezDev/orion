use crate::server::AppState;
use axum::{extract::State, Json};

pub async fn get_context(State(state): State<AppState>) -> Json<crate::memory::ContextSnapshot> {
    Json(state.memory.context_snapshot().await)
}
