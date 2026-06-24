use crate::server::AppState;
use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct McpServerView {
    pub id: String,
    pub name: String,
    pub transport: String,
    pub status: String,
    pub tools: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct AddServerBody {
    pub id: String,
    pub name: String,
    pub transport: String,
    pub command: Option<String>,
    pub url: Option<String>,
}

pub async fn list_servers(State(state): State<AppState>) -> Json<Vec<McpServerView>> {
    Json(state.memory.list_mcp_servers().await)
}

pub async fn add_server(
    State(state): State<AppState>,
    Json(body): Json<AddServerBody>,
) -> Result<Json<McpServerView>, (StatusCode, String)> {
    if body.id.trim().is_empty() || body.name.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "id and name required".into()));
    }
    let srv = McpServerView {
        id: body.id,
        name: body.name,
        transport: body.transport,
        status: "configured".into(),
        tools: vec![],
    };
    state.memory.upsert_mcp_server(srv.clone()).await;
    Ok(Json(srv))
}
