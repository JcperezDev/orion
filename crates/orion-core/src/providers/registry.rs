use crate::models::{ModelCatalog, ProviderKind};
use crate::providers::traits::LlmProvider;
use crate::providers::{AnthropicProvider, OllamaProvider, OpenAICompatibleProvider};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;

pub struct ProviderRegistry {
    providers: Mutex<HashMap<String, Arc<dyn LlmProvider>>>,
    catalog: Arc<ModelCatalog>,
}

impl ProviderRegistry {
    pub fn new(catalog: Arc<ModelCatalog>) -> Self {
        Self {
            providers: Mutex::new(HashMap::new()),
            catalog,
        }
    }

    pub fn load_from_catalog(&self) {
        let providers = self.catalog.list_providers();

        for info in providers {
            if !info.enabled {
                continue;
            }

            if let Some(key) = &info.api_key_env {
                if std::env::var(key).is_err() {
                    continue;
                }
            }

            let provider: Option<Arc<dyn LlmProvider>> = match info.kind {
                ProviderKind::OpenAICompatible | ProviderKind::Google | ProviderKind::Custom => {
                    let base_url = info.base_url.as_ref().map(|s| s.as_str()).unwrap_or("");
                    let api_key = info
                        .api_key_env
                        .as_ref()
                        .and_then(|k| std::env::var(k).ok())
                        .unwrap_or_default();
                    if !base_url.is_empty() {
                        Some(Arc::new(OpenAICompatibleProvider::new(
                            info.id.as_str(),
                            base_url,
                            &api_key,
                        )) as Arc<dyn LlmProvider>)
                    } else {
                        None
                    }
                }
                ProviderKind::Anthropic => {
                    let api_key = info
                        .api_key_env
                        .as_ref()
                        .and_then(|k| std::env::var(k).ok())
                        .unwrap_or_default();
                    Some(Arc::new(AnthropicProvider::new(&api_key)) as Arc<dyn LlmProvider>)
                }
                ProviderKind::Ollama => {
                    let base_url = info
                        .base_url
                        .as_ref()
                        .map(|s| s.as_str())
                        .unwrap_or("http://localhost:11434");
                    Some(Arc::new(OllamaProvider::new(base_url)) as Arc<dyn LlmProvider>)
                }
            };

            if let Some(p) = provider {
                self.providers.lock().insert(info.id.clone(), p);
            }
        }
    }

    pub fn register(&self, id: &str, provider: Arc<dyn LlmProvider>) {
        self.providers.lock().insert(id.to_string(), provider);
    }

    pub fn get(&self, id: &str) -> Option<Arc<dyn LlmProvider>> {
        self.providers.lock().get(id).cloned()
    }

    pub fn list_providers(&self) -> Vec<String> {
        self.providers.lock().keys().cloned().collect()
    }

    pub fn is_available(&self, id: &str) -> bool {
        self.providers.lock().contains_key(id)
    }

    pub fn catalog(&self) -> &ModelCatalog {
        &self.catalog
    }
}
