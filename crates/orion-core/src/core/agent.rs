use crate::config::Config;
use crate::models::{ModelCatalog, ModelInfo};
use crate::providers::{
    traits::{ChatRequest, Message},
    ProviderRegistry,
};
use crate::router::{ModelConstraints, ModelSelector, TaskKind};
use anyhow::Result;
use std::sync::Arc;

pub struct Agent {
    pub registry: Arc<ProviderRegistry>,
    pub selector: ModelSelector,
    pub catalog: Arc<ModelCatalog>,
    pub current_model: parking_lot::Mutex<ModelInfo>,
    pub config: Config,
}

impl Agent {
    pub async fn new(config: Config) -> Result<Self> {
        let catalog = Arc::new(ModelCatalog::new()?);
        let registry = Arc::new(ProviderRegistry::new(catalog.clone()));
        registry.load_from_catalog();

        let default_model = catalog
            .get_default_model()
            .or_else(|| catalog.pick_connected_model())
            .unwrap_or_else(|| ModelInfo {
            id: "openrouter:anthropic/claude-3.5-sonnet".to_string(),
            provider_id: "openrouter".to_string(),
            model_id: "anthropic/claude-3.5-sonnet".to_string(),
            display_name: "Claude 3.5 Sonnet".to_string(),
            source: Some("default".to_string()),
            context_window: Some(200000),
            max_output: Some(8192),
            input_price: Some(3.0),
            output_price: Some(15.0),
            supports_vision: true,
            supports_tools: true,
            supports_reasoning: true,
            supports_structured_output: true,
            enabled: true,
            is_free: false,
            is_local: false,
            is_available: true,
            rank_overall: None,
            rank_coding: None,
            rank_vision: None,
            updated_at: None,
        });

        let selector = ModelSelector::new(catalog.clone(), registry.clone());

        Ok(Self {
            registry,
            selector,
            catalog,
            current_model: parking_lot::Mutex::new(default_model),
            config,
        })
    }

    pub async fn process_input(&mut self, input: &str) {
        if input.trim().is_empty() {
            return;
        }

        if input.starts_with('/') {
            self.handle_command(input).await;
        } else {
            self.send_message(input).await;
        }
    }

    pub async fn handle_command(&mut self, cmd: &str) {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        match parts[0] {
            "/providers" => match parts.get(1).map(|s| *s) {
                Some("list") => self.list_providers(),
                Some("status") => self.list_providers_detailed(),
                Some("sync") => self.sync_providers(),
                _ => eprintln!("Usage: /providers list|status|sync"),
            },
            "/models" => match parts.get(1).map(|s| *s) {
                Some("list") => self.list_models(parts.get(2).map(|s| *s)),
                Some("search") => self.search_models(parts.get(2).map(|s| *s).unwrap_or("")),
                Some("vision") => self.list_vision_models(),
                Some("tools") => self.list_tool_models(),
                Some("free") => self.list_free_models(),
                Some("local") => self.list_local_models(),
                Some("reasoning") => self.list_reasoning_models(),
                Some("sync") => self.sync_models(),
                Some("sources") => self.list_sources(),
                Some("inspect") => {
                    let model_spec = parts.get(2).map(|s| *s).unwrap_or("");
                    self.inspect_model(model_spec);
                }
                _ => eprintln!("Usage: /models list|search|vision|tools|free|local|reasoning|sync|sources|inspect"),
            },
            "/model" => {
                if let Some(model_spec) = parts.get(1) {
                    self.set_model(model_spec).await;
                } else {
                    let current = self.current_model.lock();
                    eprintln!("Current model: {}", current.full_id());
                }
            }
            "/best" => {
                if let Some(task) = parts.get(1) {
                    self.best_model(task);
                }
            }
            "/help" => self.print_help(),
            _ => {}
        }
    }

    fn list_providers(&self) {
        let providers = self.catalog.list_providers();

        eprintln!(
            "{:<14} {:<18} {:<22} {}",
            "provider", "kind", "api_key", "status"
        );
        eprintln!("{:-<14} {:-<18} {:-<22} {}", "", "", "", "");

        for provider in providers {
            let status = if provider.enabled {
                "enabled"
            } else {
                "disabled"
            };

            let api_key_str = provider.api_key_env.as_deref().unwrap_or("none");

            let key_status = if let Some(env_var) = &provider.api_key_env {
                if std::env::var(env_var).is_ok() {
                    "ready"
                } else {
                    "missing_key"
                }
            } else {
                "ready"
            };

            let final_status = if !provider.enabled {
                "disabled".to_string()
            } else {
                format!("{}/{}", key_status, status)
            };

            eprintln!(
                "{:<14} {:<18} {:<22} {}",
                provider.id,
                provider.kind.as_str(),
                api_key_str,
                final_status
            );
        }
    }

    fn list_providers_detailed(&self) {
        let providers = self.catalog.list_providers();

        eprintln!("Provider Status:");
        eprintln!(
            "{:<12} {:<10} {:<20} {:<10} {:>6} {}",
            "provider", "enabled", "api_key", "loaded", "models", "capabilities"
        );
        eprintln!(
            "{:-<12} {:-<10} {:-<20} {:-<10} {:->6} {}",
            "", "", "", "", "", ""
        );

        for provider in providers {
            let enabled_str = if provider.enabled { "yes" } else { "no" };
            let loaded = self.registry.is_available(&provider.id);
            let loaded_str = if loaded { "yes" } else { "no" };

            let api_key_status = if let Some(env_var) = &provider.api_key_env {
                if std::env::var(env_var).is_ok() {
                    format!("{}/set", env_var)
                } else {
                    format!("{}/MISSING", env_var)
                }
            } else {
                "none".to_string()
            };

            let model_count = self.catalog.list_models(Some(&provider.id)).len();

            let caps = format!(
                "s:{}/t:{}/v:{}",
                if provider.supports_streaming {
                    "Y"
                } else {
                    "N"
                },
                if provider.supports_tools { "Y" } else { "N" },
                if provider.supports_vision { "Y" } else { "N" }
            );

            eprintln!(
                "{:<12} {:<10} {:<20} {:<10} {:>6} {}",
                provider.id, enabled_str, api_key_status, loaded_str, model_count, caps
            );

            if let Some(url) = &provider.base_url {
                eprintln!("           base_url: {}", url);
            }
        }
    }

    fn sync_providers(&self) {
        match crate::models::sync::sync_providers(&self.catalog) {
            Ok(_) => {}
            Err(e) => {
                eprintln!("{}", e);
            }
        }
    }

    fn list_models(&self, provider_id: Option<&str>) {
        let models = self.catalog.list_models(provider_id);

        if models.is_empty() {
            if let Some(pid) = provider_id {
                eprintln!(
                    "Provider '{}' registered, but no models available yet.",
                    pid
                );
                eprintln!("Run `/providers sync` to fetch models from OpenRouter.");
            } else {
                eprintln!("No models loaded yet. Run `/providers sync` or configure static catalog entries.");
            }
            return;
        }

        for model in models.iter().take(20) {
            eprintln!("  {} ({})", model.full_id(), model.display_name);
        }
        if models.len() > 20 {
            eprintln!("  ... and {} more", models.len() - 20);
        }
    }

    fn search_models(&self, query: &str) {
        let models = self.catalog.search(query);

        if models.is_empty() {
            eprintln!("No models found matching '{}'.", query);
            eprintln!("Try `/providers sync` to fetch latest models from OpenRouter.");
            return;
        }

        for model in models {
            eprintln!("  {} ({})", model.full_id(), model.display_name);
        }
    }

    fn list_vision_models(&self) {
        let models = self.catalog.list_models(None);
        let vision: Vec<_> = models.into_iter().filter(|m| m.supports_vision).collect();

        if vision.is_empty() {
            eprintln!("No vision models available yet.");
            eprintln!("Run `/providers sync` to fetch models from OpenRouter.");
            return;
        }

        for model in vision.iter().take(20) {
            eprintln!("  {} - vision", model.full_id());
        }
        if vision.len() > 20 {
            eprintln!("  ... and {} more", vision.len() - 20);
        }
    }

    fn list_tool_models(&self) {
        let models = self.catalog.list_models(None);
        let tools: Vec<_> = models.into_iter().filter(|m| m.supports_tools).collect();

        if tools.is_empty() {
            eprintln!("No models with tool calling available yet.");
            eprintln!("Run `/providers sync` to fetch models from OpenRouter.");
            return;
        }

        for model in tools.iter().take(20) {
            eprintln!("  {} - tools", model.full_id());
        }
        if tools.len() > 20 {
            eprintln!("  ... and {} more", tools.len() - 20);
        }
    }

    fn list_free_models(&self) {
        let models = self.catalog.list_models(Some("ollama"));
        for model in models.into_iter().take(10) {
            eprintln!("  {} - free", model.full_id());
        }
    }

    async fn set_model(&mut self, model_spec: &str) {
        if let Some(model) = self.catalog.get_model(model_spec) {
            let mut current = self.current_model.lock();
            *current = model.clone();
            eprintln!("Model set to: {}", model.full_id());
        } else {
            eprintln!("Model not found: {}", model_spec);

            let parts: Vec<&str> = model_spec.splitn(2, ':').collect();
            if parts.len() == 2 {
                let provider_id = parts[0];
                let search_term = parts[1];

                let provider_models: Vec<_> = self
                    .catalog
                    .list_models(Some(provider_id))
                    .into_iter()
                    .filter(|m| m.model_id.contains(search_term))
                    .take(5)
                    .collect();

                if !provider_models.is_empty() {
                    eprintln!("\nSuggestions from provider '{}':", provider_id);
                    for m in provider_models {
                        eprintln!("  /model {}", m.full_id());
                    }
                } else {
                    eprintln!("\nNo models found for provider '{}'. Try `/models list {}` to see available models.", provider_id, provider_id);
                }
            } else {
                let all_models: Vec<_> =
                    self.catalog.list_models(None).into_iter().take(5).collect();

                if all_models.is_empty() {
                    eprintln!("\nNo models in catalog. Run `/providers sync` first.");
                } else {
                    eprintln!("\nAvailable models:");
                    for m in all_models {
                        eprintln!("  /model {}", m.full_id());
                    }
                    eprintln!("\nOr try `/models search {}` to find models.", model_spec);
                }
            }
        }
    }

    fn print_help(&self) {
        eprintln!(
            r#"
ORION Commands:
  /providers list              List available providers
  /providers status            Detailed provider status (includes API key info)
  /providers sync             Sync models from OpenRouter
  /models list [provider]     List models (optionally filtered by provider)
  /models search <query>       Search models by name
  /models vision               Models with vision support
  /models tools                Models with tool calling
  /models free                 Free local models (Ollama)
  /model <provider:model>      Set model (e.g., /model openrouter:anthropic/claude-3.5-sonnet)
  /model                       Show current model
  /best <task>                Best model for task (coding, vision, cheap, etc.)
  /help                     Show this help

Providers: openrouter, openai, anthropic, ollama, deepseek, groq, mistral, together, perplexity, minimax
"#
        );
    }

    pub async fn send_message(&mut self, content: &str) {
        let model = self.current_model.lock().clone();

        if let Some(provider) = self.registry.get(&model.provider_id) {
            let messages = vec![Message {
                role: "user".to_string(),
                content: content.to_string(),
                ..Default::default()
            }];

            let request = ChatRequest {
                model: model.model_id.clone(),
                messages,
                temperature: None,
                max_tokens: Some(4096),
                tools: None,
            };

            match provider.chat_stream(request).await {
                Ok(mut stream) => {
                    use futures::StreamExt;
                    while let Some(chunk) = stream.next().await {
                        match chunk {
                            Ok(s) if s.is_empty() => break,
                            Ok(s) => print!("{s}"),
                            Err(e) => {
                                eprintln!("\n{}", format_provider_error(&e.to_string()));
                                break;
                            }
                        }
                    }
                    println!();
                }
                Err(e) => {
                    eprintln!("{}", format_provider_error(&e.to_string()));
                }
            }
        } else {
            eprintln!("Provider {} not available", model.provider_id);
        }
    }

    fn list_sources(&self) {
        let sources = self.catalog.list_sources();
        if sources.is_empty() {
            eprintln!("No model sources configured.");
            return;
        }
        eprintln!(
            "{:<12} {:<16} {:.<42} {:<9} {}",
            "id", "name", "url", "enabled", "last_sync"
        );
        eprintln!("{:-<12} {:-<16} {:-<42} {:<9} {}", "", "", "", "", "");
        for source in sources {
            let last_sync = source.last_sync_at.as_deref().unwrap_or("never");
            let last_error = source.last_error.as_deref().unwrap_or("");
            eprintln!(
                "{:<12} {:.<16} {:.<42} {:.<9} {}",
                source.id.chars().take(12).collect::<String>(),
                source.name,
                source.url.chars().take(40).collect::<String>(),
                source.enabled,
                last_sync
            );
            if !last_error.is_empty() {
                eprintln!(
                    "{:<12} {:.<16} {:.<80}",
                    "",
                    "",
                    format!("ERROR: {}", last_error)
                );
            }
        }
    }

    fn sync_models(&self) {
        eprintln!("Syncing models from all sources...");
        if let Err(e) = crate::models::sync::sync_providers(&self.catalog) {
            eprintln!("Sync failed: {}", e);
        } else {
            eprintln!("Sync complete.");
        }
    }

    fn inspect_model(&self, model_spec: &str) {
        if model_spec.is_empty() {
            eprintln!("Usage: /models inspect <provider:model>");
            return;
        }
        if let Some(model) = self.catalog.get_model(model_spec) {
            eprintln!("{:<20} {}", "provider:", model.provider_id);
            eprintln!("{:<20} {}", "model:", model.model_id);
            eprintln!("{:<20} {}", "display_name:", model.display_name);
            eprintln!(
                "{:<20} {}",
                "source:",
                model.source.as_deref().unwrap_or("unknown")
            );
            eprintln!(
                "{:<20} {}",
                "context_window:",
                model
                    .context_window
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "unknown".to_string())
            );
            eprintln!(
                "{:<20} {}",
                "input_price:",
                model
                    .input_price
                    .map(|p| format!("${:.6}/tok", p))
                    .unwrap_or_else(|| "unknown".to_string())
            );
            eprintln!(
                "{:<20} {}",
                "output_price:",
                model
                    .output_price
                    .map(|p| format!("${:.6}/tok", p))
                    .unwrap_or_else(|| "unknown".to_string())
            );
            eprintln!("{:<20} {}", "supports_vision:", model.supports_vision);
            eprintln!("{:<20} {}", "supports_tools:", model.supports_tools);
            eprintln!("{:<20} {}", "supports_reasoning:", model.supports_reasoning);
            eprintln!("{:<20} {}", "is_free:", model.is_free);
            eprintln!("{:<20} {}", "is_local:", model.is_local);
            eprintln!("{:<20} {}", "is_available:", model.is_available);
            eprintln!("{:<20} {:?}", "rank_overall:", model.rank_overall);
        } else {
            eprintln!("Model {} not found", model_spec);
        }
    }

    fn list_local_models(&self) {
        let models = self.catalog.list_models(None);
        let local: Vec<_> = models.into_iter().filter(|m| m.is_local).collect();
        if local.is_empty() {
            eprintln!("No local models found");
            return;
        }
        eprintln!("{:<20} {:<40} {}", "provider", "model", "context");
        eprintln!("{:-<20} {:-<40} {}", "", "", "");
        for model in local {
            eprintln!(
                "{:<20} {:.<40} {}",
                model.provider_id,
                model.id.chars().take(38).collect::<String>(),
                model
                    .context_window
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "?".to_string())
            );
        }
    }

    fn list_reasoning_models(&self) {
        let models = self.catalog.list_models(None);
        let reasoning: Vec<_> = models
            .into_iter()
            .filter(|m| m.id.contains("reasoning") || m.id.contains("o1") || m.id.contains("o3"))
            .collect();
        if reasoning.is_empty() {
            eprintln!("No reasoning models found");
            return;
        }
        eprintln!("{:<20} {:<40} {}", "provider", "model", "context");
        eprintln!("{:-<20} {:-<40} {}", "", "", "");
        for model in reasoning {
            eprintln!(
                "{:<20} {:.<40} {}",
                model.provider_id,
                model.id.chars().take(38).collect::<String>(),
                model
                    .context_window
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "?".to_string())
            );
        }
    }

    fn best_model(&self, task: &str) {
        let kind = match task {
            "coding" => TaskKind::Coding,
            "vision" => TaskKind::Vision,
            "reasoning" => TaskKind::Reasoning,
            "cheap" => TaskKind::Cheap,
            "fast" => TaskKind::Fast,
            "local" => TaskKind::Local,
            "tools" => TaskKind::Tools,
            _ => TaskKind::General,
        };
        let result = self
            .selector
            .select_best_model(kind, ModelConstraints::default());
        eprintln!("Task: {}", task);
        eprintln!(
            "Model: {}",
            result
                .recommendation
                .as_ref()
                .map(|r| r.model.full_id())
                .unwrap_or_else(|| "none".to_string())
        );
        if let Some(ref rec) = result.recommendation {
            eprintln!("Score: {:.2}", rec.score);
            eprintln!("Heuristic: {}", rec.heuristic_score);
            eprintln!("Fallbacks: {}", rec.fallback_count);
        }
        if !result.missing_requirements.is_empty() {
            eprintln!("Missing: {:?}", result.missing_requirements);
        }
    }
}

/// Format a provider error for the terminal, surfacing rate/usage limits with
/// actionable guidance (mirrors the desktop limit banner).
fn format_provider_error(msg: &str) -> String {
    use crate::core::ratelimit::{classify_error, ErrorClass};
    match classify_error(msg) {
        ErrorClass::RateLimited { retry_after } => {
            let when = retry_after
                .map(|s| format!(" Resets in ~{s}s."))
                .unwrap_or_default();
            format!(
                "Usage limit reached.{when} Switch model with `/model <provider:model>`, \
                 or add credits to this provider.\n  ({msg})"
            )
        }
        ErrorClass::Overloaded => {
            format!("Provider is overloaded — try again shortly.\n  ({msg})")
        }
        _ => format!("Error: {msg}"),
    }
}
