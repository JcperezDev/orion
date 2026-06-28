use orion_core::memory::{MemoryStore, Settings as MemSettings};

fn isolated_db() {
    let tmp = std::env::temp_dir().join(format!(
        "orion-mem-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&tmp).unwrap();
    std::env::set_var("ORION_MEMORY_DB", tmp.join("memory.db"));
    std::env::set_var("ORION_CATALOG_DB", tmp.join("catalog.db"));
}

#[tokio::test]
async fn memory_create_and_list_session() {
    isolated_db();
    let store = MemoryStore::new().expect("MemoryStore::new");
    let sid = uuid::Uuid::new_v4().to_string();
    store
        .create_session(&sid, "Test", "ollama", "llama3.2")
        .await
        .expect("create_session");
    let sessions = store.list_sessions().await;
    assert!(sessions.iter().any(|s| s.id == sid));
}

#[tokio::test]
async fn memory_record_and_get_messages() {
    isolated_db();
    let store = MemoryStore::new().expect("MemoryStore::new");
    let sid = uuid::Uuid::new_v4().to_string();
    store
        .create_session(&sid, "Test", "ollama", "llama3.2")
        .await
        .unwrap();
    store.record_message(&sid, "user", "hello").await;
    store.record_message(&sid, "assistant", "hi").await;
    store.record_message(&sid, "user", "how are you?").await;
    store.record_message(&sid, "assistant", "ok").await;

    let s = store.get_session(&sid).await.expect("get_session");
    let msgs = s
        .get("messages")
        .and_then(|m| m.as_array())
        .expect("messages array");
    assert_eq!(msgs.len(), 4);
    assert_eq!(msgs[0]["role"], "user");
    assert_eq!(msgs[1]["role"], "assistant");
}

#[tokio::test]
async fn memory_delete_session_removes_messages() {
    isolated_db();
    let store = MemoryStore::new().expect("MemoryStore::new");
    let sid = uuid::Uuid::new_v4().to_string();
    store
        .create_session(&sid, "Doomed", "ollama", "llama3.2")
        .await
        .unwrap();
    store.record_message(&sid, "user", "x").await;
    store.delete_session(&sid).await.expect("delete");
    assert!(store.get_session(&sid).await.is_none());
}

#[tokio::test]
async fn memory_settings_roundtrip() {
    isolated_db();
    let store = MemoryStore::new().expect("MemoryStore::new");
    let s = MemSettings {
        default_provider: Some("anthropic".into()),
        default_model: Some("claude-sonnet-4-5".into()),
        theme: Some("dracula".into()),
        language: Some("es".into()),
        auto_accept_permissions: Some(true),
        show_reasoning: Some(false),
        sound_effects: Some(false),
        notifications: Some(true),
        token_budget_per_session: Some(150_000),
        auto_compress_threshold: Some(0.8),
        mcp_enabled: Some(true),
        keybindings: None,
        permissions: None,
        ..Default::default()
    };
    store.save_settings(&s).await.expect("save_settings");
    let back = store.get_settings().await;
    assert_eq!(back.default_provider.as_deref(), Some("anthropic"));
    assert_eq!(back.default_model.as_deref(), Some("claude-sonnet-4-5"));
    assert_eq!(back.theme.as_deref(), Some("dracula"));
    assert_eq!(back.language.as_deref(), Some("es"));
    assert_eq!(back.auto_accept_permissions, Some(true));
    assert_eq!(back.token_budget_per_session, Some(150_000));
}

#[tokio::test]
async fn memory_mcp_servers_default() {
    isolated_db();
    let store = MemoryStore::new().expect("MemoryStore::new");
    let servers = store.list_mcp_servers().await;
    assert!(!servers.is_empty(), "default MCP servers should be present");
    let ids: Vec<&str> = servers.iter().map(|s| s.id.as_str()).collect();
    assert!(ids.contains(&"token-god"));
    assert!(ids.contains(&"filesystem"));
}

#[tokio::test]
async fn memory_inject_context_no_summary_is_noop() {
    isolated_db();
    let store = MemoryStore::new().expect("MemoryStore::new");
    let sid = uuid::Uuid::new_v4().to_string();
    store
        .create_session(&sid, "x", "ollama", "llama3.2")
        .await
        .unwrap();
    let mut msgs = vec![orion_core::providers::Message {
        role: "user".into(),
        content: "hi".into(),
        ..Default::default()
    }];
    store.inject_context(&sid, &mut msgs).await;
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0].role, "user");
}

#[tokio::test]
async fn memory_build_summary_prompt() {
    isolated_db();
    let store = MemoryStore::new().expect("MemoryStore::new");
    let sid = uuid::Uuid::new_v4().to_string();
    store
        .create_session(&sid, "x", "ollama", "llama3.2")
        .await
        .unwrap();
    assert!(store.build_summary_prompt(&sid).await.is_none());

    store.record_message(&sid, "user", "build a rocket").await;
    store.record_message(&sid, "assistant", "step 1: design").await;

    let prompt = store
        .build_summary_prompt(&sid)
        .await
        .expect("prompt should exist");
    assert!(prompt.contains("Summarize"));
    assert!(prompt.contains("build a rocket"));
    assert!(prompt.contains("step 1: design"));
}
