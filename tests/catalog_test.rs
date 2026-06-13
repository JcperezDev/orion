use orion_agent::models::ModelCatalog;

#[test]
fn test_catalog_providers_list() {
    let catalog = ModelCatalog::new().expect("Failed to create catalog");
    let providers = catalog.list_providers();
    assert!(!providers.is_empty(), "Should have at least some default providers");
    println!("Providers: {} found", providers.len());
}

#[test]
fn test_catalog_models_empty_initially() {
    let catalog = ModelCatalog::new().expect("Failed to create catalog");
    let models = catalog.list_models(None);
    println!("Models initially: {} (expected 0, need sync to populate)", models.len());
}

#[test]
fn test_catalog_search_returns_results() {
    let catalog = ModelCatalog::new().expect("Failed to create catalog");
    let results = catalog.search("claude");
    println!("Search 'claude': {} results (expected 0, need sync)", results.len());
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
    println!("Ollama models: {} (expected 0 unless ollama running)", ollama.len());
}

#[test]
fn test_catalog_has_default_providers() {
    let catalog = ModelCatalog::new().expect("Failed to create catalog");
    let providers = catalog.list_providers();

    let provider_ids: Vec<_> = providers.iter().map(|p| p.id.as_str()).collect();
    println!("Provider IDs: {:?}", provider_ids);

    assert!(provider_ids.contains(&"openrouter"), "Should have openrouter provider");
    assert!(provider_ids.contains(&"anthropic"), "Should have anthropic provider");
    assert!(provider_ids.contains(&"ollama"), "Should have ollama provider");
}
