use crate::server::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct Session {
    pub id: String,
    pub title: String,
    pub provider: String,
    pub model: String,
    pub created_at: String,
    pub message_count: usize,
}

pub async fn list(State(state): State<AppState>) -> Json<Vec<Session>> {
    Json(state.memory.list_sessions().await)
}

pub async fn create(
    State(state): State<AppState>,
    Json(input): Json<serde_json::Value>,
) -> Result<Json<Session>, (StatusCode, String)> {
    let id = Uuid::new_v4().to_string();
    let title = input
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("New session")
        .to_string();
    let provider = input
        .get("provider")
        .and_then(|v| v.as_str())
        .unwrap_or("anthropic")
        .to_string();
    let model = input
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("claude-sonnet-4-5")
        .to_string();

    state
        .memory
        .create_session(&id, &title, &provider, &model)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")))?;

    Ok(Json(Session {
        id,
        title,
        provider,
        model,
        created_at: chrono::Utc::now().to_rfc3339(),
        message_count: 0,
    }))
}

pub async fn get(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    state
        .memory
        .get_session(&id)
        .await
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("session {id} not found")))
        .map(Json)
}

pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    state
        .memory
        .delete_session(&id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")))?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct ForkQuery {
    pub fork_seq: Option<i64>,
    pub title: Option<String>,
}

pub async fn fork(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(params): Query<ForkQuery>,
) -> Result<Json<Session>, (StatusCode, String)> {
    let new_id = Uuid::new_v4().to_string();
    state
        .memory
        .fork_session(&new_id, &id, params.fork_seq, params.title.as_deref())
        .await
        .map_err(|e| {
            if e.to_string().contains("not found") {
                (StatusCode::NOT_FOUND, e.to_string())
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            }
        })?;

    // Load the newly created session to return its metadata
    let session = state.memory.get_session(&new_id).await.ok_or_else(|| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "fork created but failed to retrieve".into(),
        )
    })?;
    let s = serde_json::from_value::<crate::memory::store::SessionRecord>(session)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")))?;
    let count = s.messages.len();

    Ok(Json(Session {
        id: new_id,
        title: s.title,
        provider: s.provider,
        model: s.model,
        created_at: s.created_at,
        message_count: count,
    }))
}

#[derive(Debug, Deserialize)]
pub struct SummarizeBody {
    pub provider: String,
    pub model: String,
}

#[derive(Debug, Serialize)]
pub struct SummarizeResponse {
    pub summary: String,
}

pub async fn summarize(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<SummarizeBody>,
) -> Result<Json<SummarizeResponse>, (StatusCode, String)> {
    let prompt = state
        .memory
        .build_summary_prompt(&id)
        .await
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("no messages for session {id}")))?;

    let provider = state
        .registry
        .get_or_create(&body.provider)
        .ok_or_else(|| {
            (
                StatusCode::SERVICE_UNAVAILABLE,
                format!("provider not available: {}", body.provider),
            )
        })?;

    let req = crate::providers::ChatRequest {
        model: body.model.clone(),
        messages: vec![crate::providers::Message {
            role: "user".into(),
            content: prompt,
            ..Default::default()
        }],
        temperature: Some(0.2),
        max_tokens: Some(512),
        tools: None,
    };

    let mut stream = provider
        .chat_stream(req)
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("{e}")))?;
    let mut summary = String::new();
    while let Some(c) = stream.next().await {
        match c {
            Ok(s) if s.is_empty() => break,
            Ok(s) => summary.push_str(&s),
            Err(e) => return Err((StatusCode::BAD_GATEWAY, format!("{e}"))),
        }
    }

    let working_dir = std::env::current_dir()
        .ok()
        .and_then(|p| p.to_str().map(String::from))
        .unwrap_or_default();
    state
        .memory
        .save_project_summary(&working_dir, &summary)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")))?;

    Ok(Json(SummarizeResponse { summary }))
}
