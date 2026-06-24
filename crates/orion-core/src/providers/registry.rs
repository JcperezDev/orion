use crate::models::{ModelCatalog, ProviderKind};
use crate::providers::traits::{ChatRequest, LlmProvider, Message, TokenStream};
use crate::providers::{AnthropicProvider, OllamaProvider, OpenAICompatibleProvider};
use anyhow::Result;
use futures::StreamExt;
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

    fn build_provider(
        &self,
        info: &crate::models::catalog::ProviderInfo,
    ) -> Option<Arc<dyn LlmProvider>> {
        let api_key = info
            .api_key_env
            .as_ref()
            .and_then(|k| std::env::var(k).ok())
            .or_else(|| self.catalog.get_api_key(&info.id));

        let needs_key = info.kind != ProviderKind::Ollama;
        if needs_key && api_key.is_none() {
            return None;
        }
        let api_key = api_key.unwrap_or_default();

        match info.kind {
            ProviderKind::OpenAICompatible | ProviderKind::Google | ProviderKind::Custom => {
                let base_url = info.base_url.as_deref().unwrap_or("");
                if !base_url.is_empty() {
                    Some(Arc::new(OpenAICompatibleProvider::new(
                        &info.id, base_url, &api_key,
                    )) as Arc<dyn LlmProvider>)
                } else {
                    None
                }
            }
            ProviderKind::Anthropic => {
                Some(Arc::new(AnthropicProvider::new(&api_key)) as Arc<dyn LlmProvider>)
            }
            ProviderKind::Ollama => {
                let base_url = info
                    .base_url
                    .as_deref()
                    .unwrap_or("http://localhost:11434");
                Some(Arc::new(OllamaProvider::new(base_url)) as Arc<dyn LlmProvider>)
            }
        }
    }

    pub fn load_from_catalog(&self) {
        for info in self.catalog.list_providers() {
            if !info.enabled {
                continue;
            }
            if let Some(p) = self.build_provider(&info) {
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

    pub fn get_or_create(&self, provider_id: &str) -> Option<Arc<dyn LlmProvider>> {
        if let Some(p) = self.providers.lock().get(provider_id) {
            return Some(p.clone());
        }

        let info = self.catalog.get_provider(provider_id)?;
        if !info.enabled {
            return None;
        }

        let provider = self.build_provider(&info)?;
        self.providers
            .lock()
            .insert(provider_id.to_string(), provider.clone());
        Some(provider)
    }

    pub fn stream_chat(
        &self,
        provider_id: &str,
        model: &str,
        messages: Vec<Message>,
    ) -> Result<TokenStream> {
        let provider = self.get_or_create(provider_id).ok_or_else(|| {
            anyhow::anyhow!("provider not available: {provider_id}")
        })?;

        let request = ChatRequest {
            model: model.to_string(),
            messages,
            temperature: None,
            max_tokens: None,
            tools: None,
        };

        // We can't await in a sync function; we need to block_on here.
        // For server use, callers should be inside an async context.
        // Provide both sync and async variants.
        let rt = tokio::runtime::Handle::try_current();
        match rt {
            Ok(handle) => tokio::task::block_in_place(|| {
                handle.block_on(provider.chat_stream(request))
            }),
            Err(_) => {
                let rt = tokio::runtime::Runtime::new()?;
                rt.block_on(provider.chat_stream(request))
            }
        }
    }

    pub async fn stream_chat_async(
        &self,
        provider_id: &str,
        model: &str,
        messages: Vec<Message>,
    ) -> Result<TokenStream> {
        let provider = self.get_or_create(provider_id).ok_or_else(|| {
            anyhow::anyhow!("provider not available: {provider_id}")
        })?;
        let request = ChatRequest {
            model: model.to_string(),
            messages,
            temperature: None,
            max_tokens: None,
            tools: None,
        };
        provider.chat_stream(request).await
    }

    pub async fn chat(
        &self,
        provider_id: &str,
        model: &str,
        messages: Vec<Message>,
    ) -> Result<String> {
        let mut stream = self.stream_chat_async(provider_id, model, messages).await?;
        let mut out = String::new();
        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(s) => out.push_str(&s),
                Err(e) => return Err(e),
            }
        }
        Ok(out)
    }
}
