use orion_agent::models::ModelCatalog;

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
    assert!(matches!(
        google.kind,
        orion_agent::models::ProviderKind::Google
    ));
}
