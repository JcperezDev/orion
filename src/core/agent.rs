use crate::config::Config;
use crate::models::{ModelCatalog, ModelInfo};
use crate::providers::{
    traits::{ChatRequest, Message},
    ProviderRegistry,
};
use crate::router::ModelSelector;
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

        let default_model = catalog.get_default_model().unwrap_or_else(|| ModelInfo {
            id: "openrouter:anthropic/claude-3.5-sonnet".to_string(),
            provider_id: "openrouter".to_string(),
            model_id: "anthropic/claude-3.5-sonnet".to_string(),
            display_name: "Claude 3.5 Sonnet".to_string(),
            context_window: Some(200000),
            max_output: Some(8192),
            input_price: Some(3.0),
            output_price: Some(15.0),
            supports_vision: true,
            supports_tools: true,
            supports_reasoning: true,
            supports_structured_output: true,
            enabled: true,
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
                _ => eprintln!("Usage: /models list|search|vision|tools|free"),
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
                    if let Some(model) = self.selector.best_for_task(task) {
                        eprintln!("Best model for {}: {}", task, model.full_id());
                    }
                }
            }
            "/help" => self.print_help(),
            _ => {}
        }
    }

    fn list_providers(&self) {
        let providers = self.catalog.list_providers();
        for provider in providers {
            let status = if provider.enabled {
                "enabled"
            } else {
                "disabled"
            };

            let api_key_status = if let Some(env_var) = &provider.api_key_env {
                if std::env::var(env_var).is_ok() {
                    "API key set"
                } else {
                    "MISSING API key"
                }
            } else {
                "no API key required"
            };

            eprintln!(
                "  {} ({}) - {} [{}]",
                provider.name, provider.id, status, api_key_status
            );
            if let Some(url) = &provider.base_url {
                eprintln!("    base_url: {}", url);
            }
        }
    }

    fn list_providers_detailed(&self) {
        let providers = self.catalog.list_providers();
        eprintln!("Provider Status:");
        for provider in providers {
            let enabled_str = if provider.enabled {
                "enabled"
            } else {
                "disabled"
            };
            let loaded = self.registry.is_available(&provider.id);

            let api_key_status = if let Some(env_var) = &provider.api_key_env {
                if std::env::var(env_var).is_ok() {
                    "✓ API key set"
                } else {
                    "✗ MISSING API key"
                }
            } else {
                "— no API key required"
            };

            let loaded_str = if loaded {
                "✓ loaded"
            } else {
                "✗ not loaded"
            };

            eprintln!(
                "  {}: {} | {} | {}",
                provider.id, enabled_str, api_key_status, loaded_str
            );
            eprintln!(
                "    {} | streaming:{} | tools:{} | vision:{}",
                provider.kind.as_str(),
                if provider.supports_streaming {
                    "✓"
                } else {
                    "✗"
                },
                if provider.supports_tools {
                    "✓"
                } else {
                    "✗"
                },
                if provider.supports_vision {
                    "✓"
                } else {
                    "✗"
                }
            );
        }
    }

    fn sync_providers(&self) {
        if let Err(e) = crate::models::sync::sync_providers(&self.catalog) {
            eprintln!("Sync failed: {}", e);
        } else {
            eprintln!("Sync complete");
        }
    }

    fn list_models(&self, provider_id: Option<&str>) {
        let models = self.catalog.list_models(provider_id);
        for model in models.iter().take(20) {
            eprintln!("  {} ({})", model.full_id(), model.display_name);
        }
        if models.len() > 20 {
            eprintln!("  ... and {} more", models.len() - 20);
        }
    }

    fn search_models(&self, query: &str) {
        let models = self.catalog.search(query);
        for model in models {
            eprintln!("  {} ({})", model.full_id(), model.display_name);
        }
    }

    fn list_vision_models(&self) {
        let models = self.catalog.list_models(None);
        for model in models.into_iter().filter(|m| m.supports_vision).take(20) {
            eprintln!("  {} - vision", model.full_id());
        }
    }

    fn list_tool_models(&self) {
        let models = self.catalog.list_models(None);
        for model in models.into_iter().filter(|m| m.supports_tools).take(20) {
            eprintln!("  {} - tools", model.full_id());
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
            }];

            let request = ChatRequest {
                model: model.model_id.clone(),
                messages,
                temperature: None,
                max_tokens: Some(4096),
                tools: None,
            };

            match provider.chat_stream(request).await {
                Ok(response) => {
                    eprintln!("Assistant: {}", response.content);
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                }
            }
        } else {
            eprintln!("Provider {} not available", model.provider_id);
        }
    }
}
