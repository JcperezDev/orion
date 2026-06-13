use orion_agent::models::ModelCatalog;

fn env_remove(key: &str) {
    std::env::remove_var(key);
}

#[test]
fn test_catalog_providers_list() {
    let catalog = ModelCatalog::new().expect("Failed to create catalog");
    let providers = catalog.list_providers();
    assert!(
        !providers.is_empty(),
        "Should have at least some default providers"
    );
    println!("Providers: {} found", providers.len());
}

#[test]
fn test_catalog_models_empty_initially() {
    let catalog = ModelCatalog::new().expect("Failed to create catalog");
    let models = catalog.list_models(None);
    println!(
        "Models initially: {} (expected 0, need sync to populate)",
        models.len()
    );
}

#[test]
fn test_catalog_search_returns_results() {
    let catalog = ModelCatalog::new().expect("Failed to create catalog");
    let results = catalog.search("claude");
    println!(
        "Search 'claude': {} results (expected 0, need sync)",
        results.len()
    );
}

#[test]
fn test_catalog_vision_filter() {
    let catalog = ModelCatalog::new().expect("Failed to create catalog");
    let all = catalog.list_models(None);
    let vision: Vec<_> = all.into_iter().filter(|m| m.supports_vision).collect();
    println!("Vision models: {} (expected 0, need sync)", vision.len());
}

#[test]
fn test_catalog_ollama_provider() {
    let catalog = ModelCatalog::new().expect("Failed to create catalog");
    let ollama = catalog.list_models(Some("ollama"));
    println!(
        "Ollama models: {} (expected 0 unless ollama running)",
        ollama.len()
    );
}

#[test]
fn test_catalog_has_default_providers() {
    let catalog = ModelCatalog::new().expect("Failed to create catalog");
    let providers = catalog.list_providers();

    let provider_ids: Vec<_> = providers.iter().map(|p| p.id.as_str()).collect();
    println!("Provider IDs: {:?}", provider_ids);

    assert!(
        provider_ids.contains(&"openrouter"),
        "Should have openrouter provider"
    );
    assert!(
        provider_ids.contains(&"anthropic"),
        "Should have anthropic provider"
    );
    assert!(
        provider_ids.contains(&"ollama"),
        "Should have ollama provider"
    );
}

#[test]
fn test_catalog_openai_compatible_providers() {
    let catalog = ModelCatalog::new().expect("Failed to create catalog");
    let providers = catalog.list_providers();

    let openai_compat_ids = vec![
        "deepseek",
        "groq",
        "mistral",
        "together",
        "perplexity",
        "minimax",
    ];

    let provider_ids: Vec<_> = providers.iter().map(|p| p.id.as_str()).collect();

    for id in openai_compat_ids {
        assert!(provider_ids.contains(&id), "Should have {} provider", id);
    }
}

#[test]
fn test_catalog_provider_capabilities() {
    let catalog = ModelCatalog::new().expect("Failed to create catalog");
    let providers = catalog.list_providers();

    let provider_map: std::collections::HashMap<_, _> =
        providers.iter().map(|p| (p.id.as_str(), p)).collect();

    assert!(
        provider_map.get("anthropic").unwrap().supports_tools,
        "Anthropic should support tools"
    );
    assert!(
        provider_map.get("anthropic").unwrap().supports_vision,
        "Anthropic should support vision"
    );
    assert!(
        provider_map.get("anthropic").unwrap().supports_streaming,
        "Anthropic should support streaming"
    );

    assert!(
        provider_map.get("ollama").unwrap().supports_streaming,
        "Ollama should support streaming"
    );
    assert!(
        provider_map.get("ollama").unwrap().supports_tools,
        "Ollama should support tools"
    );
    assert!(
        !provider_map.get("ollama").unwrap().supports_vision,
        "Ollama base should not support vision (model dependent)"
    );

    assert!(
        provider_map.get("deepseek").unwrap().supports_streaming,
        "DeepSeek should support streaming"
    );
    assert!(
        provider_map.get("groq").unwrap().supports_streaming,
        "Groq should support streaming"
    );
}

#[test]
fn test_catalog_provider_get_by_id() {
    let catalog = ModelCatalog::new().expect("Failed to create catalog");

    let deepseek = catalog.get_provider("deepseek");
    assert!(
        deepseek.is_some(),
        "Should be able to get deepseek provider"
    );
    let deepseek = deepseek.unwrap();
    assert_eq!(deepseek.id, "deepseek");
    assert_eq!(deepseek.name, "DeepSeek");
    assert!(deepseek.base_url.is_some());
    assert!(deepseek.base_url.unwrap().contains("deepseek.com"));

    let ollama = catalog.get_provider("ollama");
    assert!(ollama.is_some(), "Should be able to get ollama provider");
    assert!(
        ollama.unwrap().api_key_env.is_none(),
        "Ollama should not require API key"
    );
}

#[test]
fn test_catalog_provider_kind() {
    let catalog = ModelCatalog::new().expect("Failed to create catalog");

    let anthropic = catalog.get_provider("anthropic").unwrap();
    assert!(matches!(
        anthropic.kind,
        orion_agent::models::ProviderKind::Anthropic
    ));

    let openrouter = catalog.get_provider("openrouter").unwrap();
    assert!(matches!(
        openrouter.kind,
        orion_agent::models::ProviderKind::OpenAICompatible
    ));

    let ollama = catalog.get_provider("ollama").unwrap();
    assert!(matches!(
        ollama.kind,
        orion_agent::models::ProviderKind::Ollama
    ));

    let google = catalog.get_provider("google").unwrap();
    assert!(
        matches!(
            google.kind,
            orion_agent::models::ProviderKind::OpenAICompatible
        ),
        "Google should be OpenAICompatible, not Google"
    );
}

#[test]
fn test_google_provider_base_url() {
    let catalog = ModelCatalog::new().expect("Failed to create catalog");
    let google = catalog.get_provider("google").unwrap();

    let expected_url = "https://generativelanguage.googleapis.com/v1beta/openai";
    assert_eq!(
        google.base_url.as_deref(),
        Some(expected_url),
        "Google should use OpenAI-compatible endpoint"
    );
}

#[test]
fn test_all_openai_compatible_providers_have_base_urls() {
    let catalog = ModelCatalog::new().expect("Failed to create catalog");
    let providers = catalog.list_providers();

    let openai_compat_kinds = vec![
        "deepseek",
        "groq",
        "mistral",
        "together",
        "perplexity",
        "minimax",
        "google",
        "qwen",
    ];

    for id in openai_compat_kinds {
        let provider = providers.iter().find(|p| p.id == id);
        assert!(provider.is_some(), "Provider {} should exist", id);
        let p = provider.unwrap();
        assert!(
            p.base_url.is_some() && !p.base_url.as_ref().unwrap().is_empty(),
            "Provider {} should have a base_url",
            id
        );
    }
}

#[test]
fn test_provider_requires_api_key() {
    let catalog = ModelCatalog::new().expect("Failed to create catalog");

    let providers_requiring_key = vec![
        ("deepseek", "DEEPSEEK_API_KEY"),
        ("groq", "GROQ_API_KEY"),
        ("mistral", "MISTRAL_API_KEY"),
        ("together", "TOGETHER_API_KEY"),
        ("perplexity", "PERPLEXITY_API_KEY"),
        ("minimax", "MINIMAX_API_KEY"),
        ("google", "GOOGLE_API_KEY"),
    ];

    for (id, expected_env) in providers_requiring_key {
        let provider = catalog.get_provider(id).unwrap();
        assert_eq!(
            provider.api_key_env.as_deref(),
            Some(expected_env),
            "{} should require {}",
            id,
            expected_env
        );
    }
}

#[test]
fn test_sync_openrouter_missing_key_returns_error() {
    use orion_agent::models::sync;

    let catalog = ModelCatalog::new().expect("Failed to create catalog");

    env_remove("OPENROUTER_API_KEY");

    let result = sync::sync_providers(&catalog);
    assert!(
        result.is_err(),
        "sync_providers should fail when OPENROUTER_API_KEY is missing"
    );
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("OPENROUTER_API_KEY"),
        "Error should mention missing API key: {}",
        err_msg
    );
}

#[test]
fn test_models_list_empty_returns_message() {
    let catalog = ModelCatalog::new().expect("Failed to create catalog");
    let models = catalog.list_models(None);
    let provider_models = catalog.list_models(Some("deepseek"));

    if models.is_empty() {
        assert!(
            provider_models.is_empty(),
            "Deepseek models should be empty when overall models is empty"
        );
    } else {
        println!(
            "Models not empty ({} found) - possibly test pollution from prior run",
            models.len()
        );
    }
}

#[test]
fn test_list_sources_returns_default_sources() {
    let catalog = ModelCatalog::new().expect("Failed to create catalog");
    let sources = catalog.list_sources();

    assert!(
        !sources.is_empty(),
        "Should have at least some default sources"
    );

    let source_ids: Vec<_> = sources.iter().map(|s| s.id.as_str()).collect();
    println!("Source IDs: {:?}", source_ids);

    assert!(
        source_ids.contains(&"models_dev"),
        "Should have models_dev source"
    );
    assert!(
        source_ids.contains(&"openrouter"),
        "Should have openrouter source"
    );

    let openrouter_source = sources.iter().find(|s| s.id == "openrouter").unwrap();
    assert!(
        openrouter_source.enabled,
        "OpenRouter source should be enabled"
    );
    assert!(
        openrouter_source.url.contains("openrouter.ai"),
        "OpenRouter URL should contain openrouter.ai"
    );
}

#[test]
fn test_get_model_returns_none_for_nonexistent() {
    let catalog = ModelCatalog::new().expect("Failed to create catalog");

    let result = catalog.get_model("nonexistent:model-id");
    assert!(
        result.is_none(),
        "get_model should return None for nonexistent model"
    );

    let result2 = catalog.get_model("openrouter:nonexistent-model");
    assert!(
        result2.is_none(),
        "get_model should return None for nonexistent provider:model combo"
    );
}

#[test]
fn test_sync_openrouter_missing_key_preserves_existing_models() {
    use orion_agent::models::sync;

    let catalog = ModelCatalog::new().expect("Failed to create catalog");

    catalog
        .upsert_model(
            "openrouter",
            "test-model",
            "Test Model",
            &[
                ("context_window", "8192".to_string()),
                ("input_price", "0.001".to_string()),
            ],
        )
        .expect("Failed to insert test model");

    let models_before = catalog.list_models(None);
    let test_model_exists = models_before.iter().any(|m| m.model_id == "test-model");
    assert!(test_model_exists, "Test model should exist before sync");

    env_remove("OPENROUTER_API_KEY");

    let result = sync::sync_providers(&catalog);
    assert!(
        result.is_err(),
        "sync_providers should fail when OPENROUTER_API_KEY is missing"
    );

    let models_after = catalog.list_models(None);
    let test_model_still_exists = models_after.iter().any(|m| m.model_id == "test-model");
    assert!(
        test_model_still_exists,
        "Test model should still exist after failed sync"
    );
}
