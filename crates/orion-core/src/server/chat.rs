use crate::providers::traits::{ChatRequest, Message};
use crate::server::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse,
    },
    Json,
};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::time::Duration;

#[derive(Debug, Deserialize)]
pub struct ChatRequestBody {
    pub session_id: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub message: String,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub history: Vec<HistoryItem>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct HistoryItem {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct ChatErrorBody {
    pub error: String,
}

pub async fn post_chat(
    State(state): State<AppState>,
    Json(body): Json<ChatRequestBody>,
) -> Result<Sse<impl futures::Stream<Item = Result<Event, Infallible>>>, (StatusCode, Json<ChatErrorBody>)>
{
    let session_id = body
        .session_id
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    if state.memory.get_session(&session_id).await.is_none() {
        let provider_label = body
            .provider
            .clone()
            .unwrap_or_else(|| "default".into());
        let model_label = body.model.clone().unwrap_or_else(|| "default".into());
        let title = format!(
            "Session {}",
            &session_id[..session_id.len().min(8)]
        );
        if let Err(e) = state
            .memory
            .create_session(&session_id, &title, &provider_label, &model_label)
            .await
        {
            tracing::warn!("failed to auto-create session {session_id}: {e}");
        }
    }

    let model = match body.model.clone() {
        Some(m) => m,
        None => state
            .catalog
            .get_default_model()
            .map(|m| m.model_id)
            .ok_or_else(|| {
                (
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(ChatErrorBody {
                        error: "no default model configured".into(),
                    }),
                )
            })?,
    };

    let provider_id = match body.provider.clone() {
        Some(p) => p,
        None => state
            .catalog
            .get_default_model()
            .map(|m| m.provider_id)
            .ok_or_else(|| {
                (
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(ChatErrorBody {
                        error: "no provider configured".into(),
                    }),
                )
            })?,
    };

    let mut messages: Vec<Message> = body
        .history
        .into_iter()
        .map(|h| Message {
            role: h.role,
            content: h.content,
        })
        .collect();

    state
        .memory
        .inject_context(&session_id, &mut messages)
        .await;

    messages.push(Message {
        role: "user".into(),
        content: body.message.clone(),
    });

    let request = ChatRequest {
        model: model.clone(),
        messages,
        temperature: body.temperature,
        max_tokens: body.max_tokens,
        tools: None,
    };

    let provider = state
        .registry
        .get_or_create(&provider_id)
        .ok_or_else(|| {
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ChatErrorBody {
                    error: format!("provider not available: {provider_id}"),
                }),
            )
        })?;

    let token_stream = match provider.chat_stream(request).await {
        Ok(s) => s,
        Err(e) => {
            return Err((
                StatusCode::BAD_GATEWAY,
                Json(ChatErrorBody {
                    error: format!("provider error: {e}"),
                }),
            ))
        }
    };

    state.memory.record_message(&session_id, "user", &body.message).await;

    let session_id_for_stream = session_id.clone();
    let memory = state.memory.clone();

    let sse_stream = async_stream::stream! {
        let mut full = String::new();
        let mut token_stream = Box::pin(token_stream);
        while let Some(chunk) = token_stream.next().await {
            match chunk {
                Ok(s) if s.is_empty() => break,
                Ok(s) => {
                    full.push_str(&s);
                    yield Ok(Event::default().data(s));
                }
                Err(e) => {
                    yield Ok(Event::default()
                        .event("error")
                        .data(format!("{e}")));
                    break;
                }
            }
        }
        memory.record_message(&session_id_for_stream, "assistant", &full).await;
        yield Ok(Event::default().event("done").data("[DONE]"));
    };

    Ok(Sse::new(sse_stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15))))
}

pub async fn get_stream(
    State(_state): State<AppState>,
    Path(_session_id): Path<String>,
) -> impl IntoResponse {
    // For reconnect/replay — placeholder. In a real impl, replay buffered events.
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(serde_json::json!({"error": "stream resume not yet wired"})),
    )
}
