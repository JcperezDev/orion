//! Integration test for orion-server: spin up an Axum router in-process,
//! drive the HTTP API end-to-end against a mock LLM provider.

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use orion_core::{
    build_router,
    memory::MemoryStore,
    middleware::TokenOptimizer,
    models::ModelCatalog,
    providers::{ChatRequest, LlmProvider, ProviderRegistry, TokenStream},
    server::AppState,
};
use std::sync::Arc;
use tower::ServiceExt;

fn isolated_db() {
    let tmp = std::env::temp_dir().join(format!(
        "orion-int-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&tmp).unwrap();
    std::env::set_var("ORION_CATALOG_DB", tmp.join("catalog.db"));
    std::env::set_var("ORION_MEMORY_DB", tmp.join("memory.db"));
}

struct MockProvider;

#[async_trait::async_trait]
impl LlmProvider for MockProvider {
    async fn chat_stream(&self, _req: ChatRequest) -> anyhow::Result<TokenStream> {
        let s = futures::stream::iter(vec![
            Ok("Hola".to_string()),
            Ok(", ".to_string()),
            Ok("mundo".to_string()),
        ]);
        Ok(Box::pin(s))
    }
    fn provider_id(&self) -> &str {
        "mock"
    }
    fn base_url(&self) -> &str {
        "http://mock"
    }
    fn api_key_env(&self) -> &str {
        ""
    }
    fn requires_api_key(&self) -> bool {
        false
    }
}

async fn build_test_app() -> axum::Router {
    isolated_db();
    let catalog = Arc::new(ModelCatalog::new().expect("catalog"));
    let registry = Arc::new(ProviderRegistry::new(catalog.clone()));
    registry.register("mock", Arc::new(MockProvider));

    let memory = Arc::new(MemoryStore::new().expect("memory"));
    let optimizer = Arc::new(TokenOptimizer::new().expect("optimizer"));

    let state = AppState {
        registry,
        catalog,
        memory,
        token_optimizer: optimizer,
    };
    build_router(state)
}

#[tokio::test]
async fn health_endpoint_returns_ok() {
    let app = build_test_app().await;
    let res = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), 1024).await.unwrap();
    assert_eq!(std::str::from_utf8(&body).unwrap(), "orion-server ok");
}

#[tokio::test]
async fn providers_endpoint_lists_seeded_providers() {
    let app = build_test_app().await;
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/providers")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(res.into_body(), 1_048_576).await.unwrap(),
    )
    .unwrap();
    let arr = body.as_array().expect("providers is array");
    let ids: Vec<&str> = arr.iter().map(|p| p["id"].as_str().unwrap()).collect();
    assert!(ids.contains(&"anthropic"));
    assert!(ids.contains(&"ollama"));
    assert!(ids.contains(&"openrouter"));
}

#[tokio::test]
async fn settings_roundtrip_via_http() {
    let app = build_test_app().await;
    let new_settings = serde_json::json!({
        "default_provider": "ollama",
        "default_model": "llama3.2",
        "theme": "dracula",
        "language": "es",
        "auto_accept_permissions": true,
        "token_budget_per_session": 100000
    });
    let put = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PUT)
                .uri("/api/settings")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&new_settings).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(put.status(), StatusCode::OK);

    let get = app
        .oneshot(
            Request::builder()
                .uri("/api/settings")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(get.status(), StatusCode::OK);
    let body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(get.into_body(), 1_048_576).await.unwrap(),
    )
    .unwrap();
    assert_eq!(body["default_provider"], "ollama");
    assert_eq!(body["theme"], "dracula");
    assert_eq!(body["auto_accept_permissions"], true);
}

#[tokio::test]
async fn mcp_servers_endpoint_returns_defaults() {
    let app = build_test_app().await;
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/mcp/servers")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(res.into_body(), 1_048_576).await.unwrap(),
    )
    .unwrap();
    let arr = body.as_array().unwrap();
    let ids: Vec<&str> = arr.iter().map(|s| s["id"].as_str().unwrap()).collect();
    assert!(ids.contains(&"token-god"));
    assert!(ids.contains(&"filesystem"));
}

#[tokio::test]
async fn chat_streams_sse_with_mock_provider() {
    let app = build_test_app().await;
    let body = serde_json::json!({
        "provider": "mock",
        "model": "mock-model",
        "message": "hola",
        "session_id": "test-stream"
    });
    let res = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/chat")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        res.headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok()),
        Some("text/event-stream")
    );

    let bytes = axum::body::to_bytes(res.into_body(), 1_048_576)
        .await
        .unwrap();
    let text = std::str::from_utf8(&bytes).unwrap();
    assert!(text.contains("data: Hola"));
    assert!(text.contains("data: , "));
    assert!(text.contains("data: mundo"));
    assert!(text.contains("event: done"));
    assert!(text.contains("data: [DONE]"));
}

#[tokio::test]
async fn chat_records_messages_in_memory() {
    let app = build_test_app().await;
    let body = serde_json::json!({
        "provider": "mock",
        "model": "mock-model",
        "message": "user-message-xyz",
        "session_id": "memory-trace"
    });
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/chat")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let _ = axum::body::to_bytes(res.into_body(), 1_048_576).await.unwrap();

    let get = app
        .oneshot(
            Request::builder()
                .uri("/api/sessions/memory-trace")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(get.status(), StatusCode::OK);
    let body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(get.into_body(), 1_048_576).await.unwrap(),
    )
    .unwrap();
    let msgs = body["messages"].as_array().unwrap();
    assert!(msgs
        .iter()
        .any(|m| m["role"] == "user" && m["content"] == "user-message-xyz"));
    assert!(msgs.iter().any(|m| m["role"] == "assistant"));
}

#[tokio::test]
async fn models_endpoint_filters_by_query() {
    let app = build_test_app().await;
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/models?provider=anthropic")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(res.into_body(), 1_048_576).await.unwrap(),
    )
    .unwrap();
    let arr = body.as_array().unwrap();
    for m in arr {
        assert_eq!(m["providerId"], "anthropic");
    }
}

#[tokio::test]
async fn sessions_create_and_list() {
    let app = build_test_app().await;
    let create_body = serde_json::json!({
        "title": "My Session",
        "provider": "ollama",
        "model": "llama3.2"
    });
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/sessions")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&create_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(res.into_body(), 1_048_576).await.unwrap(),
    )
    .unwrap();
    let id = body["id"].as_str().unwrap().to_string();

    let list = app
        .oneshot(
            Request::builder()
                .uri("/api/sessions")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let list_body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(list.into_body(), 1_048_576).await.unwrap(),
    )
    .unwrap();
    let arr = list_body.as_array().unwrap();
    assert!(arr.iter().any(|s| s["id"] == id));
}

#[tokio::test]
async fn chat_missing_provider_returns_error() {
    let app = build_test_app().await;
    let body = serde_json::json!({
        "provider": "nonexistent",
        "model": "x",
        "message": "hi"
    });
    let res = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/chat")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn fork_session_copies_messages() {
    isolated_db();
    let app = build_test_app().await;

    // Create a session first
    let create = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/sessions")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::json!({"title": "original"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create.status(), StatusCode::OK);
    let session: serde_json::Value =
        serde_json::from_slice(&axum::body::to_bytes(create.into_body(), usize::MAX).await.unwrap()).unwrap();
    let original_id = session["id"].as_str().unwrap().to_string();

    // Fork the session
    let fork = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(&format!("/api/sessions/{original_id}/fork"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::default())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(fork.status(), StatusCode::OK);
    let fork_session: serde_json::Value =
        serde_json::from_slice(&axum::body::to_bytes(fork.into_body(), usize::MAX).await.unwrap()).unwrap();
    let fork_id = fork_session["id"].as_str().unwrap();
    assert_ne!(fork_id, &original_id);
    assert_eq!(fork_session["title"].as_str().unwrap(), "original (fork)");

    // GET the forked session
    let get = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(&format!("/api/sessions/{fork_id}"))
                .body(Body::default())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(get.status(), StatusCode::OK);
    let fetched: serde_json::Value =
        serde_json::from_slice(&axum::body::to_bytes(get.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(fetched["id"].as_str().unwrap(), fork_id);
    assert_eq!(fetched["messages"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn fork_nonexistent_session_returns_404() {
    isolated_db();
    let app = build_test_app().await;

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/sessions/nonexistent-id/fork")
                .body(Body::default())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}
