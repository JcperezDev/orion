#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use orion_core::models::catalog::{ModelInfo, ProviderInfo, Session};
use orion_core::providers::registry::ProviderRegistry;
use orion_core::providers::traits::Message;
use orion_core::ModelCatalog;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use tauri::{Emitter, State};
use futures::StreamExt;

pub struct AppState {
    pub catalog: Arc<ModelCatalog>,
    pub registry: Arc<ProviderRegistry>,
    pub default_model: Mutex<Option<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProviderView {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub enabled: bool,
    pub available: bool,
    pub has_api_key: bool,
    pub base_url: Option<String>,
    pub models_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProviderTestResult {
    pub success: bool,
    pub models: Vec<String>,
    pub error: Option<String>,
    pub latency_ms: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelView {
    pub id: String,
    pub provider: String,
    pub name: String,
    pub context_window: Option<u32>,
    pub supports_vision: bool,
    pub supports_tools: bool,
    pub is_free: bool,
    pub is_local: bool,
    pub is_available: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatMessageDto {
    pub role: String,
    pub content: String,
}

fn info_to_view(info: &ProviderInfo, available: bool, has_key: bool) -> ProviderView {
    ProviderView {
        id: info.id.clone(),
        name: info.name.clone(),
        kind: info.kind.as_str().to_string(),
        enabled: info.enabled,
        available,
        has_api_key: has_key,
        base_url: info.base_url.clone(),
        models_count: 0,
    }
}

#[tauri::command]
fn list_providers(state: State<'_, AppState>) -> Vec<ProviderView> {
    let available = state.registry.list_providers();
    state
        .catalog
        .list_providers()
        .into_iter()
        .map(|info| {
            let avail = available.contains(&info.id) || state.registry.get_or_create(&info.id).is_some();
            let has_key = state.catalog.get_api_key(&info.id).is_some()
                || info
                    .api_key_env
                    .as_ref()
                    .and_then(|k| std::env::var(k).ok())
                    .is_some();
            let mut view = info_to_view(&info, avail, has_key);
            view.models_count = state.catalog.list_models(Some(&info.id)).len();
            view
        })
        .collect()
}

#[tauri::command]
fn get_connected_providers(state: State<'_, AppState>) -> Vec<ProviderView> {
    list_providers(state)
        .into_iter()
        .filter(|p| p.has_api_key || p.id == "ollama")
        .collect()
}

#[tauri::command]
fn list_models(
    state: State<'_, AppState>,
    provider: Option<String>,
) -> Vec<ModelView> {
    state
        .catalog
        .list_models(provider.as_deref())
        .into_iter()
        .map(model_to_view)
        .collect()
}

fn model_to_view(m: ModelInfo) -> ModelView {
    ModelView {
        id: m.full_id(),
        provider: m.provider_id,
        name: m.display_name,
        context_window: m.context_window,
        supports_vision: m.supports_vision,
        supports_tools: m.supports_tools,
        is_free: m.is_free,
        is_local: m.is_local,
        is_available: m.is_available,
    }
}

#[tauri::command]
fn search_models(state: State<'_, AppState>, query: String) -> Vec<ModelView> {
    state
        .catalog
        .search(&query)
        .into_iter()
        .map(model_to_view)
        .collect()
}

#[tauri::command]
fn inspect_model(state: State<'_, AppState>, id: String) -> Option<ModelView> {
    state.catalog.get_model(&id).map(model_to_view)
}

#[tauri::command]
fn best_model(
    state: State<'_, AppState>,
    task: String,
) -> Option<ModelView> {
    let models = match task.as_str() {
        "coding" => state.catalog.get_best_coding(),
        "vision" => state.catalog.get_best_vision(),
        _ => state.catalog.get_best_overall(),
    };
    models.into_iter().next().map(model_to_view)
}

#[tauri::command]
fn get_default_model(state: State<'_, AppState>) -> Option<String> {
    state
        .catalog
        .get_default_model()
        .map(|m| m.full_id())
        .or_else(|| state.default_model.lock().clone())
}

#[tauri::command]
fn set_default_model(state: State<'_, AppState>, model_id: String) -> Result<(), String> {
    state
        .catalog
        .set_default_model(&model_id)
        .map_err(|e| e.to_string())?;
    *state.default_model.lock() = Some(model_id);
    Ok(())
}

#[tauri::command]
fn save_provider_api_key(
    state: State<'_, AppState>,
    provider_id: String,
    api_key: String,
) -> Result<(), String> {
    state
        .catalog
        .save_api_key(&provider_id, &api_key)
        .map_err(|e| e.to_string())?;
    state
        .catalog
        .set_provider_enabled(&provider_id, true)
        .map_err(|e| e.to_string())?;
    let _ = state.registry.get_or_create(&provider_id);
    Ok(())
}

#[tauri::command]
fn reload_registry(state: State<'_, AppState>) -> Result<(), String> {
    state.registry.load_from_catalog();
    Ok(())
}

#[tauri::command]
async fn test_provider_connection(
    provider_id: String,
    api_key: String,
) -> Result<ProviderTestResult, String> {
    let start = Instant::now();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;

    let (url, headers) = match provider_id.as_str() {
        "anthropic" => (
            "https://api.anthropic.com/v1/models",
            vec![
                ("x-api-key", api_key.clone()),
                ("anthropic-version", "2023-06-01".to_string()),
            ],
        ),
        "openai" => (
            "https://api.openai.com/v1/models",
            vec![("Authorization", format!("Bearer {}", api_key))],
        ),
        "openrouter" => (
            "https://openrouter.ai/api/v1/models",
            vec![("Authorization", format!("Bearer {}", api_key))],
        ),
        "google" => (
            &format!(
                "https://generativelanguage.googleapis.com/v1/models?key={}",
                api_key
            )[..]
            ,
            vec![],
        ),
        "groq" => (
            "https://api.groq.com/openai/v1/models",
            vec![("Authorization", format!("Bearer {}", api_key))],
        ),
        "mistral" => (
            "https://api.mistral.ai/v1/models",
            vec![("Authorization", format!("Bearer {}", api_key))],
        ),
        "deepseek" => (
            "https://api.deepseek.com/v1/models",
            vec![("Authorization", format!("Bearer {}", api_key))],
        ),
        "together" => (
            "https://api.together.xyz/v1/models",
            vec![("Authorization", format!("Bearer {}", api_key))],
        ),
        "perplexity" => (
            "https://api.perplexity.ai/models",
            vec![("Authorization", format!("Bearer {}", api_key))],
        ),
        "minimax" => (
            "https://api.minimaxi.chat/v1/models",
            vec![("Authorization", format!("Bearer {}", api_key))],
        ),
        "ollama" => (
            "http://localhost:11434/api/tags",
            vec![],
        ),
        _ => {
            // Generic OpenAI-compatible
            (
                "https://api.openai.com/v1/models",
                vec![("Authorization", format!("Bearer {}", api_key))],
            )
        }
    };

    let mut req = client.get(url);
    for (k, v) in &headers {
        req = req.header(*k, v.as_str());
    }

    match req.send().await {
        Ok(resp) => {
            let status = resp.status();
            if !status.is_success() {
                let body = resp.text().await.unwrap_or_default();
                return Ok(ProviderTestResult {
                    success: false,
                    models: vec![],
                    error: Some(format!("HTTP {} — {}", status, body.chars().take(200).collect::<String>())),
                    latency_ms: start.elapsed().as_millis() as u64,
                });
            }
            let body: serde_json::Value = resp
                .json()
                .await
                .map_err(|e| format!("Invalid JSON: {}", e))?;

            let models: Vec<String> = if let Some(arr) = body.get("data").and_then(|d| d.as_array()) {
                arr.iter()
                    .filter_map(|m| {
                        m.get("id")
                            .and_then(|i| i.as_str())
                            .map(|s| s.to_string())
                    })
                    .take(50)
                    .collect()
            } else if let Some(arr) = body.get("models").and_then(|d| d.as_array()) {
                arr.iter()
                    .filter_map(|m| {
                        m.get("name")
                            .and_then(|i| i.as_str())
                            .map(|s| s.to_string())
                    })
                    .take(50)
                    .collect()
            } else {
                vec![]
            };

            Ok(ProviderTestResult {
                success: true,
                models,
                error: None,
                latency_ms: start.elapsed().as_millis() as u64,
            })
        }
        Err(e) => Ok(ProviderTestResult {
            success: false,
            models: vec![],
            error: Some(format!("Connection failed: {}", e)),
            latency_ms: start.elapsed().as_millis() as u64,
        }),
    }
}

#[tauri::command]
fn sync_provider_models(
    state: State<'_, AppState>,
    provider_id: String,
) -> Result<usize, String> {
    use orion_core::models::sync;
    let res = match provider_id.as_str() {
        "openrouter" => sync::sync_openrouter(&state.catalog).map_err(|e| e.to_string()),
        _ => sync::sync_models_dev(&state.catalog).map_err(|e| e.to_string()),
    };
    state
        .catalog
        .update_provider_sync_time(&provider_id)
        .ok();
    res?;
    Ok(0)
}

#[tauri::command]
async fn chat(
    state: State<'_, AppState>,
    provider_id: String,
    model_id: String,
    messages: Vec<ChatMessageDto>,
) -> Result<String, String> {
    let msgs: Vec<Message> = messages
        .into_iter()
        .map(|m| Message {
            role: m.role,
            content: m.content,
        })
        .collect();

    state
        .registry
        .chat(&provider_id, &model_id, msgs)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn send_message(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    session_id: String,
    content: String,
    mode: Option<String>,
) -> Result<String, String> {
    let _ = mode;

    // Resolve active model (provider:model format).
    let active = state
        .catalog
        .get_default_model()
        .ok_or_else(|| "No active model. Connect a provider first.".to_string())?;
    let provider_id = active.provider_id.clone();
    let model_id = active.model_id.clone();

    let msgs = vec![Message {
        role: "user".to_string(),
        content: content.clone(),
    }];

    let mut stream = state
        .registry
        .stream_chat_async(&provider_id, &model_id, msgs)
        .await
        .map_err(|e| e.to_string())?;

    let mut full_response = String::new();
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(token) => {
                full_response.push_str(&token);
                let _ = app_handle.emit("orion://token", &token);
            }
            Err(e) => {
                let msg = e.to_string();
                let _ = app_handle.emit("orion://error", &msg);
                return Err(msg);
            }
        }
    }

    let _ = app_handle.emit("orion://done", ());
    let _ = state.catalog.touch_session(&session_id);

    Ok(full_response)
}

#[tauri::command]
fn delete_provider_api_key(
    state: State<'_, AppState>,
    provider_id: String,
) -> Result<(), String> {
    state
        .catalog
        .save_api_key(&provider_id, "")
        .map_err(|e| e.to_string())?;
    state
        .catalog
        .set_provider_enabled(&provider_id, false)
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn get_sessions(state: State<'_, AppState>) -> Vec<Session> {
    state.catalog.list_sessions()
}

#[tauri::command]
fn create_session(
    state: State<'_, AppState>,
    title: Option<String>,
) -> Result<Session, String> {
    let session = state
        .catalog
        .create_session(title.as_deref())
        .map_err(|e| e.to_string())?;
    state
        .catalog
        .set_active_session(&session.id)
        .map_err(|e| e.to_string())?;
    Ok(session)
}

#[tauri::command]
fn get_active_session(state: State<'_, AppState>) -> Option<Session> {
    if let Some(s) = state.catalog.get_active_session() {
        return Some(s);
    }
    // No active session yet — create a default one and return it.
    let session = state.catalog.create_session(Some("New session")).ok()?;
    let _ = state.catalog.set_active_session(&session.id);
    Some(session)
}

#[tauri::command]
fn set_active_session(state: State<'_, AppState>, id: String) -> Result<(), String> {
    state
        .catalog
        .set_active_session(&id)
        .map_err(|e| e.to_string())?;
    state.catalog.touch_session(&id).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn delete_session(state: State<'_, AppState>, id: String) -> Result<(), String> {
    state
        .catalog
        .delete_session(&id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn rename_session(
    state: State<'_, AppState>,
    id: String,
    title: String,
) -> Result<(), String> {
    state
        .catalog
        .rename_session(&id, &title)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn health() -> &'static str {
    "orion-desktop ok"
}

// Renamed command aliases (spec rename: set_default_model -> set_active_model,
// save_provider_api_key -> save_provider). Old names kept for one version.
#[tauri::command]
fn set_active_model(state: State<'_, AppState>, model_id: String) -> Result<(), String> {
    set_default_model(state, model_id)
}

#[tauri::command]
fn save_provider(
    state: State<'_, AppState>,
    provider_id: String,
    api_key: String,
) -> Result<(), String> {
    save_provider_api_key(state, provider_id, api_key)
}

fn main() {
    tracing_subscriber::fmt::init();

    let catalog = Arc::new(ModelCatalog::new().expect("failed to init model catalog"));
    let registry = Arc::new(ProviderRegistry::new(catalog.clone()));
    registry.load_from_catalog();

    let default_model = catalog.get_default_model().map(|m| m.full_id());

    let state = AppState {
        catalog,
        registry,
        default_model: Mutex::new(default_model),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            health,
            list_providers,
            get_connected_providers,
            list_models,
            search_models,
            inspect_model,
            best_model,
            get_default_model,
            set_default_model,
            set_active_model,
            save_provider_api_key,
            save_provider,
            delete_provider_api_key,
            reload_registry,
            test_provider_connection,
            sync_provider_models,
            chat,
            send_message,
            get_sessions,
            create_session,
            get_active_session,
            set_active_session,
            delete_session,
            rename_session,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
