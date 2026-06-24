use futures::StreamExt;
use orion_core::middleware::TokenOptimizer;
use orion_core::middleware::token_optimizer::OptimizerConfig;

#[tokio::test]
async fn token_optimizer_default_config() {
    let t = TokenOptimizer::new().expect("new");
    let cfg = t.config();
    assert!(cfg.enabled);
    assert_eq!(cfg.max_context_tokens, 100_000);
    assert!((cfg.auto_compress_threshold - 0.75).abs() < 0.001);
}

#[tokio::test]
async fn token_optimizer_record_and_stats() {
    let t = TokenOptimizer::new().expect("new");
    t.record("s1", 100);
    t.record("s1", 50);
    t.record("s2", 30);
    let s = t.stats();
    assert_eq!(s.used, 180);
    assert_eq!(*s.by_session.get("s1").unwrap(), 150);
    assert_eq!(*s.by_session.get("s2").unwrap(), 30);
}

#[tokio::test]
async fn token_optimizer_maybe_compress_threshold() {
    let t = TokenOptimizer::new().expect("new");
    t.set_config(OptimizerConfig {
        enabled: true,
        max_context_tokens: 1000,
        auto_compress_threshold: 0.5,
        budget_per_session: 5000,
    });
    assert!(!t.maybe_compress("s", 400));
    assert!(t.maybe_compress("s", 600));
}

#[tokio::test]
async fn token_optimizer_disabled_never_compresses() {
    let t = TokenOptimizer::new().expect("new");
    t.set_config(OptimizerConfig {
        enabled: false,
        max_context_tokens: 100,
        auto_compress_threshold: 0.1,
        budget_per_session: 1000,
    });
    assert!(!t.maybe_compress("s", 100_000));
}

#[test]
fn stream_pipe_drains() {
    use futures::stream;
    let s = stream::iter(vec![
        Ok::<String, anyhow::Error>("a".to_string()),
        Ok("b".into()),
        Ok("c".into()),
    ]);
    futures::pin_mut!(s);
    let rt = tokio::runtime::Runtime::new().unwrap();
    let out: String = rt.block_on(async {
        let mut acc = String::new();
        while let Some(c) = s.next().await {
            acc.push_str(&c.unwrap());
        }
        acc
    });
    assert_eq!(out, "abc");
}
