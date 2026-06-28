#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use orion_core::core::dispatch::{DispatchConfig, DispatchEvent, Dispatcher};
use orion_core::memory::project::{merged_system_prompt, ProjectMemoryLoader};
use orion_core::SpillManager;
use orion_core::models::catalog::{ModelInfo, ProviderInfo, Session};
use orion_core::permissions::{Action as PermissionAction, PermissionConfig, PermissionEngine};
use orion_core::providers::registry::ProviderRegistry;
use orion_core::providers::traits::Message;
use orion_core::tools::{
    builtin_registry, ApprovalChannel, ApprovalRequest, ApprovalResponse, ToolRegistry,
};
use orion_core::ModelCatalog;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Instant;
use tauri::{AppHandle, Emitter, State};
use futures::StreamExt;
use tokio::sync::oneshot;

pub struct AppState {
    pub catalog: Arc<ModelCatalog>,
    pub registry: Arc<ProviderRegistry>,
    pub default_model: Mutex<Option<String>>,
    pub tools: Arc<ToolRegistry>,
    pub permissions: Arc<PermissionEngine>,
    pub dispatcher: Arc<Dispatcher>,
    pub project_memory: StdMutex<Vec<orion_core::memory::project::ProjectMemory>>,
    pub approvals: Arc<ApprovalBridge>,
    pub cancel_flag: Arc<AtomicBool>,
    /// Pre-edit file contents for reversible edits, keyed by tool_call_id.
    /// `None` content = the file was newly created (undo deletes it).
    pub undo_stack: StdMutex<HashMap<String, Vec<(PathBuf, Option<String>)>>>,
}

pub struct ApprovalBridge {
    next_id: AtomicU64,
    pending: Mutex<std::collections::HashMap<u64, oneshot::Sender<ApprovalResponse>>>,
}

impl ApprovalBridge {
    pub fn new() -> Self {
        Self {
            next_id: AtomicU64::new(1),
            pending: Mutex::new(std::collections::HashMap::new()),
        }
    }

    pub fn create(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::SeqCst)
    }

    pub fn register(&self, id: u64, tx: oneshot::Sender<ApprovalResponse>) {
        self.pending.lock().insert(id, tx);
    }

    pub fn resolve(&self, id: u64, decision: ApprovalResponse) -> bool {
        if let Some(tx) = self.pending.lock().remove(&id) {
            let _ = tx.send(decision);
            true
        } else {
            false
        }
    }
}

pub struct TauriApprovalChannel {
    pub app: AppHandle,
    pub bridge: Arc<ApprovalBridge>,
}

#[async_trait::async_trait]
impl ApprovalChannel for TauriApprovalChannel {
    async fn request_approval(&self, request: ApprovalRequest) -> ApprovalResponse {
        let id = self.bridge.create();
        let (tx, rx) = oneshot::channel();
        self.bridge.register(id, tx);

        let payload = serde_json::json!({
            "id": id,
            "tool": request.tool_name,
            "action": request.action,
            "pattern": request.matched_pattern,
            "arguments": request.arguments,
        });
        let _ = self.app.emit("orion://approval_request", payload);

        match rx.await {
            Ok(decision) => decision,
            Err(_) => ApprovalResponse::Deny,
        }
    }
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
        _ => (
            "https://api.openai.com/v1/models",
            vec![("Authorization", format!("Bearer {}", api_key))],
        )
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
            ..Default::default()
        })
        .collect();

    state
        .registry
        .chat(&provider_id, &model_id, msgs)
        .await
        .map_err(|e| e.to_string())
}

/// Emit a provider error to the frontend, routing rate/usage limits to the
/// friendly "limit reached" banner (with resume) instead of a raw error.
fn emit_provider_error(app_handle: &AppHandle, msg: &str) {
    use orion_core::core::ratelimit::{classify_error, ErrorClass};
    match classify_error(msg) {
        ErrorClass::RateLimited { retry_after } => {
            let _ = app_handle.emit(
                "orion://limit_reached",
                serde_json::json!({ "retry_after_secs": retry_after, "message": clean_provider_message(msg) }),
            );
            let _ = app_handle.emit("orion://done", ());
        }
        _ => {
            let _ = app_handle.emit("orion://error", clean_provider_message(msg));
        }
    }
}

/// Pull the human-readable `"message":"..."` out of a provider JSON error body,
/// falling back to a truncated raw string.
fn clean_provider_message(msg: &str) -> String {
    const NEEDLE: &str = "\"message\":\"";
    if let Some(idx) = msg.find(NEEDLE) {
        let rest = &msg[idx + NEEDLE.len()..];
        if let Some(end) = rest.find('"') {
            let extracted = rest[..end].trim();
            if !extracted.is_empty() {
                return extracted.to_string();
            }
        }
    }
    msg.chars().take(200).collect()
}

#[tauri::command]
async fn send_message(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    session_id: String,
    content: String,
    mode: Option<String>,
    history: Option<Vec<HistoryMsg>>,
) -> Result<String, String> {
    state.cancel_flag.store(false, Ordering::SeqCst);
    // Prior conversation turns (user/assistant) for multi-turn context.
    let history: Vec<Message> = history
        .unwrap_or_default()
        .into_iter()
        .filter(|m| (m.role == "user" || m.role == "assistant") && !m.content.trim().is_empty())
        .map(|m| Message { role: m.role, content: m.content, ..Default::default() })
        .collect();

    let resolved_mode = mode.as_deref().unwrap_or("build");
    let agent_mode = resolved_mode == "agent";
    let plan_mode = resolved_mode == "plan";

    // Resolve active model (provider:model format). If none is selected yet,
    // auto-pick a model from a connected provider so the user can just chat.
    let active = match state.catalog.get_default_model() {
        Some(m) => m,
        None => {
            let connected: std::collections::HashSet<String> = state
                .catalog
                .list_providers()
                .into_iter()
                .filter(|p| {
                    p.id == "ollama"
                        || state.catalog.get_api_key(&p.id).is_some()
                        || p.api_key_env
                            .as_ref()
                            .and_then(|k| std::env::var(k).ok())
                            .is_some()
                })
                .map(|p| p.id)
                .collect();
            let mut candidates: Vec<_> = state
                .catalog
                .list_models(None)
                .into_iter()
                .filter(|m| connected.contains(&m.provider_id) && m.is_available)
                .collect();
            // Prefer tool-capable models (needed for agent mode).
            candidates.sort_by_key(|m| !m.supports_tools);
            match candidates.into_iter().next() {
                Some(m) => {
                    let _ = state.catalog.set_default_model(&m.full_id());
                    let _ = app_handle.emit("orion://model_changed", m.full_id());
                    m
                }
                None => {
                    return Err("No connected provider has a usable model. Open Settings → Providers to connect one.".to_string());
                }
            }
        }
    };
    let provider_id = active.provider_id.clone();
    let model_id = active.model_id.clone();

    let provider = state
        .registry
        .get_or_create(&provider_id)
        .ok_or_else(|| format!("provider not available: {provider_id}"))?;

    // Persist the user's message so the conversation survives restarts.
    let _ = state.catalog.add_message(&session_id, "user", &content);

    if !agent_mode {
        // Chat-only path (build or plan): stream text and emit orion://token events.
        let mut msgs = history.clone();
        msgs.push(Message {
            role: "user".to_string(),
            content: if plan_mode {
                format!("[PLAN MODE]\n{}", content)
            } else {
                content.clone()
            },
            ..Default::default()
        });

        let mut stream = match state
            .registry
            .stream_chat_async(&provider_id, &model_id, msgs)
            .await
        {
            Ok(s) => s,
            Err(e) => {
                // Surfaced via an event (error/limit banner); return Ok so the
                // frontend doesn't also report the invoke rejection.
                emit_provider_error(&app_handle, &e.to_string());
                return Ok(String::new());
            }
        };

        let mut full_response = String::new();
        while let Some(chunk) = stream.next().await {
            if state.cancel_flag.load(Ordering::SeqCst) {
                state.cancel_flag.store(false, Ordering::SeqCst);
                let _ = app_handle.emit("orion://done", ());
                return Ok(full_response);
            }
            match chunk {
                Ok(token) => {
                    full_response.push_str(&token);
                    let _ = app_handle.emit("orion://token", &token);
                }
                Err(e) => {
                    emit_provider_error(&app_handle, &e.to_string());
                    return Ok(full_response);
                }
            }
        }

        if !full_response.trim().is_empty() {
            let _ = state.catalog.add_message(&session_id, "assistant", &full_response);
        }
        // Guard against a silent empty response (provider returned 200 but no
        // content) so the user always gets feedback.
        if full_response.trim().is_empty() {
            let _ = app_handle.emit(
                "orion://error",
                &format!(
                    "The model ({}:{}) returned an empty response. Try another model from the menu in the top bar.",
                    provider_id, model_id
                ),
            );
        }
        let _ = app_handle.emit("orion://done", ());
        let _ = state.catalog.touch_session(&session_id);

        Ok(full_response)
    } else {
        // Agent mode: dispatcher handles tool calls.
        let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        let approval_channel = Arc::new(TauriApprovalChannel {
            app: app_handle.clone(),
            bridge: state.approvals.clone(),
        });
        let full_access = state.catalog.get_bool_config("full_access");
        let mut dispatch_cfg = DispatchConfig::new(cwd.clone())
            .with_approval(approval_channel)
            .with_spill(SpillManager::new_temp())
            .with_plan_mode(plan_mode)
            .with_full_access(full_access);
        // Load this project's learned "always allow" rules and keep the store
        // so new approvals persist.
        if let Ok(store) = orion_core::LearnedStore::open() {
            let store = Arc::new(store);
            let _ = store.hydrate(&state.permissions, &cwd);
            dispatch_cfg = dispatch_cfg.with_learned(store);
        }
        let dispatcher = Dispatcher::new(
            state.tools.clone(),
            state.permissions.clone(),
            dispatch_cfg,
        );

        let mut messages = Vec::new();
        // Prepend project memory (AGENTS.md / ORION.md) if any.
        let memory_snapshot = state.project_memory.lock().unwrap().clone();
        let prompt = merged_system_prompt(&memory_snapshot);
        if !prompt.is_empty() {
            messages.push(Message {
                role: "system".into(),
                content: prompt,
                ..Default::default()
            });
        }
        // Prior conversation turns for multi-turn context.
        messages.extend(history.iter().cloned());
        messages.push(Message {
            role: "user".into(),
            content: content.clone(),
            ..Default::default()
        });

        let events = dispatcher
            .run(provider, &provider_id, &model_id, messages)
            .await
            .map_err(|e| e.to_string())?;

        let mut full_response = String::new();
        for ev in events {
            if state.cancel_flag.load(Ordering::SeqCst) {
                state.cancel_flag.store(false, Ordering::SeqCst);
                break;
            }
            match ev {
                DispatchEvent::Token(text) => {
                    full_response.push_str(&text);
                    let _ = app_handle.emit("orion://token", &text);
                }
                DispatchEvent::ToolCall(call) => {
                    let _ = app_handle.emit("orion://tool_call", &call);
                }
                DispatchEvent::ToolResult { tool_call_id, content, is_error } => {
                    let _ = app_handle.emit(
                        "orion://tool_result",
                        serde_json::json!({
                            "tool_call_id": tool_call_id,
                            "content": content,
                            "is_error": is_error,
                        }),
                    );
                }
                DispatchEvent::StepSnapshot(_snap) => {
                    // UI can optionally store snapshots for undo
                }
                DispatchEvent::Undoable { tool_call_id, paths, summary, before } => {
                    state
                        .undo_stack
                        .lock()
                        .unwrap()
                        .insert(tool_call_id.clone(), before);
                    let _ = app_handle.emit(
                        "orion://undoable",
                        serde_json::json!({
                            "tool_call_id": tool_call_id,
                            "paths": paths.iter().map(|p| p.to_string_lossy().to_string()).collect::<Vec<_>>(),
                            "summary": summary,
                        }),
                    );
                }
                DispatchEvent::Retrying { attempt, delay_secs, reason } => {
                    let _ = app_handle.emit(
                        "orion://retrying",
                        serde_json::json!({ "attempt": attempt, "delay_secs": delay_secs, "reason": reason }),
                    );
                }
                DispatchEvent::LimitReached { retry_after_secs, message } => {
                    let _ = app_handle.emit(
                        "orion://limit_reached",
                        serde_json::json!({ "retry_after_secs": retry_after_secs, "message": message }),
                    );
                    // Stop streaming UI; the session keeps its messages so the
                    // user can resume by sending again once the limit clears.
                    let _ = app_handle.emit("orion://done", ());
                    return Ok(full_response);
                }
                DispatchEvent::Done { .. } => {
                    let _ = app_handle.emit("orion://done", ());
                }
                DispatchEvent::Error(msg) => {
                    let _ = app_handle.emit("orion://error", &msg);
                    return Err(msg);
                }
            }
        }

        if !full_response.trim().is_empty() {
            let _ = state.catalog.add_message(&session_id, "assistant", &full_response);
        }
        let _ = app_handle.emit("orion://done", ());
        let _ = state.catalog.touch_session(&session_id);
        Ok(full_response)
    }
}

#[tauri::command]
fn get_messages(state: State<'_, AppState>, session_id: String) -> Vec<orion_core::models::catalog::StoredMessage> {
    state.catalog.get_messages(&session_id)
}

#[tauri::command]
fn cancel_generation(state: State<'_, AppState>) -> Result<(), String> {
    state.cancel_flag.store(true, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
fn delete_provider_api_key(
    state: State<'_, AppState>,
    provider_id: String,
) -> Result<(), String> {
    state
        .catalog
        .delete_api_key(&provider_id)
        .map_err(|e| e.to_string())?;
    state
        .catalog
        .set_provider_enabled(&provider_id, false)
        .map_err(|e| e.to_string())?;
    state
        .registry
        .remove_provider(&provider_id);
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

#[tauri::command]
fn list_tools(state: State<'_, AppState>) -> Vec<orion_core::tools::ToolDefinition> {
    state.tools.list()
}

#[tauri::command]
fn add_permission_rule(
    state: State<'_, AppState>,
    tool: String,
    pattern: String,
    action: String,
) -> Result<(), String> {
    let act = match action.as_str() {
        "allow" => PermissionAction::Allow,
        "ask" => PermissionAction::Ask,
        "deny" => PermissionAction::Deny,
        _ => return Err(format!("unknown action: {action}")),
    };
    state.permissions.add_rule(&tool, &pattern, act)
}

/// Undo a reversible edit: restore each target to its pre-edit content (or
/// delete it if it was newly created). Returns the restored file paths.
#[tauri::command]
fn undo_changes(state: State<'_, AppState>, tool_call_id: String) -> Result<Vec<String>, String> {
    let entry = state.undo_stack.lock().unwrap().remove(&tool_call_id);
    let files = entry.ok_or_else(|| "nothing to undo for this action".to_string())?;
    let mut restored = Vec::new();
    for (path, before) in files {
        match before {
            Some(content) => {
                if let Some(parent) = path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                std::fs::write(&path, content).map_err(|e| e.to_string())?;
            }
            None => {
                // File was newly created by the edit — remove it.
                let _ = std::fs::remove_file(&path);
            }
        }
        restored.push(path.to_string_lossy().to_string());
    }
    Ok(restored)
}

/// Read the master "full access" switch (Trust Engine off → allow everything).
#[tauri::command]
fn get_full_access(state: State<'_, AppState>) -> bool {
    state.catalog.get_bool_config("full_access")
}

/// Toggle the master "full access" switch. Persisted in the shared catalog DB.
#[tauri::command]
fn set_full_access(state: State<'_, AppState>, enabled: bool) -> Result<(), String> {
    state
        .catalog
        .set_config("full_access", if enabled { "true" } else { "false" })
        .map_err(|e| e.to_string())
}

/// A prior conversation turn sent from the frontend for multi-turn context.
#[derive(Debug, Serialize, Deserialize)]
pub struct HistoryMsg {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectMemoryDto {
    pub path: String,
    pub body: String,
}

#[tauri::command]
fn get_project_memory(state: State<'_, AppState>) -> Vec<ProjectMemoryDto> {
    state
        .project_memory
        .lock()
        .unwrap()
        .iter()
        .map(|m| ProjectMemoryDto {
            path: m.path.to_string_lossy().to_string(),
            body: m.body.clone(),
        })
        .collect()
}

#[tauri::command]
fn reload_project_memory(
    state: State<'_, AppState>,
    cwd: Option<String>,
) -> Vec<ProjectMemoryDto> {
    let path = cwd
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")));
    let loader = ProjectMemoryLoader::new(path);
    let loaded = loader.load().unwrap_or_default();
    let dtos: Vec<ProjectMemoryDto> = loaded
        .iter()
        .map(|m| ProjectMemoryDto {
            path: m.path.to_string_lossy().to_string(),
            body: m.body.clone(),
        })
        .collect();
    *state.project_memory.lock().unwrap() = loaded;
    dtos
}

#[tauri::command]
fn submit_approval(
    state: State<'_, AppState>,
    id: u64,
    decision: String,
) -> bool {
    let resp = match decision.as_str() {
        "allow" => ApprovalResponse::Allow,
        "allow_always" => ApprovalResponse::AllowAlways,
        "deny" => ApprovalResponse::Deny,
        _ => ApprovalResponse::Deny,
    };
    state.approvals.resolve(id, resp)
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

    let tools = Arc::new(builtin_registry());
    let permissions = Arc::new(PermissionEngine::new(PermissionConfig::safe_defaults()));
    let cfg = DispatchConfig::new(std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")))
        .with_spill(SpillManager::new_temp());
    let dispatcher = Arc::new(Dispatcher::new(
        tools.clone(),
        permissions.clone(),
        cfg,
    ));

    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let project_memory = ProjectMemoryLoader::new(cwd).load().unwrap_or_default();

    let approvals = Arc::new(ApprovalBridge::new());

    let state = AppState {
        catalog,
        registry,
        default_model: Mutex::new(default_model),
        tools,
        permissions,
        dispatcher,
        project_memory: StdMutex::new(project_memory),
        approvals,
        cancel_flag: Arc::new(AtomicBool::new(false)),
        undo_stack: StdMutex::new(HashMap::new()),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            // Ensure the window/taskbar icon is the ORION logo at runtime (in
            // dev the WM icon isn't always applied from the bundle config).
            use tauri::Manager;
            if let (Some(win), Some(icon)) =
                (app.get_webview_window("main"), app.default_window_icon().cloned())
            {
                let _ = win.set_icon(icon);
            }
            Ok(())
        })
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
            get_messages,
            list_tools,
            add_permission_rule,
            get_full_access,
            set_full_access,
            undo_changes,
            get_project_memory,
            reload_project_memory,
            submit_approval,
            cancel_generation,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
