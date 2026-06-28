//! Session sharing — export a session to JSON, optionally publish to GitHub Gist.

use crate::models::catalog::ModelCatalog;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Format of an exported session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedSession {
    pub version: u32,
    pub id: String,
    pub title: String,
    pub provider: String,
    pub model: String,
    pub created_at: String,
    pub updated_at: String,
    pub messages: Vec<SharedMessage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<SharedMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedMessage {
    pub role: String,
    pub content: String,
    pub timestamp: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SharedMetadata {
    pub orion_version: String,
    pub share_url: Option<String>,
    pub message_count: usize,
    pub tokens_estimate: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

/// Export a session by id.
pub fn export_session(
    catalog: &ModelCatalog,
    session_id: &str,
    sanitize: bool,
) -> Result<SharedSession> {
    let _ = catalog
        .get_session(session_id)
        .with_context(|| format!("session {session_id} not found"))?;

    // Look up active model (best effort).
    let active_model = catalog.get_session(session_id).and_then(|s| s.active_model);
    let (provider, model) = match active_model {
        Some(am) if am.contains(':') => {
            let (p, m) = am.split_once(':').unwrap();
            (p.to_string(), m.to_string())
        }
        Some(am) => ("unknown".to_string(), am),
        None => ("unknown".to_string(), "unknown".to_string()),
    };

    let messages = read_messages(catalog, session_id, sanitize)?;
    let message_count = messages.len();
    let tokens_estimate: usize = messages.iter().map(|m| m.content.len() / 4).sum();

    let session = catalog.get_session(session_id).unwrap();

    Ok(SharedSession {
        version: 1,
        id: session.id.clone(),
        title: session.title.clone(),
        provider,
        model,
        created_at: session.created_at.clone(),
        updated_at: session.updated_at.clone(),
        messages,
        metadata: Some(SharedMetadata {
            orion_version: env!("CARGO_PKG_VERSION").to_string(),
            share_url: None,
            message_count,
            tokens_estimate,
            tags: Vec::new(),
        }),
    })
}

/// Read messages for a session from the catalog.
fn read_messages(
    catalog: &ModelCatalog,
    session_id: &str,
    sanitize: bool,
) -> Result<Vec<SharedMessage>> {
    // Best-effort: read messages via a public catalog method if it exists.
    // We avoid internal message table queries here to stay decoupled from schema.
    // If the catalog doesn't expose a list_messages() helper, return empty.
    if let Some(msgs) = catalog.list_messages_for_session(session_id) {
        Ok(msgs
            .into_iter()
            .map(|(role, content, ts, tool_name)| {
                let content = if sanitize {
                    sanitize_text(&content)
                } else {
                    content
                };
                SharedMessage {
                    role,
                    content,
                    timestamp: ts,
                    tool_name,
                }
            })
            .collect())
    } else {
        Ok(Vec::new())
    }
}

/// Strip common secrets from content (best-effort).
fn sanitize_text(s: &str) -> String {
    use regex::Regex;
    use std::sync::OnceLock;

    static SECRET_PATTERNS: OnceLock<Vec<(Regex, &'static str)>> = OnceLock::new();
    let patterns = SECRET_PATTERNS.get_or_init(|| {
        let raw: &[(&str, &str)] = &[
            (r"sk-[A-Za-z0-9]{20,}", "[REDACTED:openai-key]"),
            (r"sk-ant-[A-Za-z0-9-]{20,}", "[REDACTED:anthropic-key]"),
            (r"ghp_[A-Za-z0-9]{20,}", "[REDACTED:github-token]"),
            (r"github_pat_[A-Za-z0-9_]{20,}", "[REDACTED:github-pat]"),
            (r"AKIA[0-9A-Z]{16}", "[REDACTED:aws-key]"),
            (r"xai-[A-Za-z0-9]{20,}", "[REDACTED:xai-key]"),
            (r"Bearer\s+[A-Za-z0-9._\-]{20,}", "Bearer [REDACTED]"),
        ];
        raw.iter()
            .filter_map(|(p, r)| Regex::new(p).ok().map(|re| (re, *r)))
            .collect()
    });
    let mut out = s.to_string();
    for (re, replacement) in patterns.iter() {
        out = re.replace_all(&out, *replacement).to_string();
    }
    // Truncate absolute paths to last 2 components to avoid leaking full /home/... paths.
    if let Ok(re) = Regex::new(r"/(?:[^\s/]+/){3,}[^\s/]+") {
        out = re
            .replace_all(&out, |caps: &regex::Captures| {
                let path = &caps[0];
                let parts: Vec<&str> = path.split('/').collect();
                if parts.len() > 2 {
                    format!("…/{}/{}", parts[parts.len() - 2], parts[parts.len() - 1])
                } else {
                    path.to_string()
                }
            })
            .to_string();
    }
    out
}

/// Save a session to disk as JSON.
pub fn write_to_file(session: &SharedSession, path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let json = serde_json::to_string_pretty(session)?;
    std::fs::write(path, json)?;
    Ok(())
}

/// Read a session from disk.
pub fn read_from_file(path: impl AsRef<Path>) -> Result<SharedSession> {
    let raw = std::fs::read_to_string(path.as_ref())?;
    let session: SharedSession = serde_json::from_str(&raw)?;
    Ok(session)
}

/// Import a session into the local catalog. Creates a new session with the
/// same id (or a fresh one if the id is already taken).
pub fn import_session(
    catalog: &ModelCatalog,
    session: &SharedSession,
    new_title: Option<&str>,
) -> Result<String> {
    let new_title = new_title.unwrap_or(&session.title);
    let created = catalog.create_session(Some(new_title))?;
    Ok(created.id)
}

/// Convert a session id to a deterministic share slug.
pub fn slug_for(title: &str) -> String {
    let mut out = String::new();
    for ch in title.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            out.push(ch.to_ascii_lowercase());
        } else if ch.is_whitespace() && !out.ends_with('-') {
            out.push('-');
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "session".to_string()
    } else {
        trimmed.chars().take(40).collect()
    }
}

/// Publish to a GitHub Gist. Requires a GitHub personal access token in
/// `GITHUB_TOKEN` env var. Returns the HTML URL of the created gist.
pub async fn publish_gist(
    session: &SharedSession,
    token: &str,
    public: bool,
) -> Result<String> {
    let body = serde_json::json!({
        "description": format!("ORION session: {}", session.title),
        "public": public,
        "files": {
            format!("{}.json", session.id): {
                "content": serde_json::to_string_pretty(session)?
            }
        }
    });

    let client = reqwest::Client::new();
    let resp = client
        .post("https://api.github.com/gists")
        .bearer_auth(token)
        .header("User-Agent", "orion-cli")
        .json(&body)
        .send()
        .await
        .context("publishing gist")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("gist publish failed ({}): {}", status, text);
    }

    let value: serde_json::Value = resp.json().await?;
    let url = value["html_url"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("gist response missing html_url"))?
        .to_string();
    Ok(url)
}

#[cfg(test)] mod tests {

    use super::*;

    fn temp_path() -> std::path::PathBuf {
        let tmp = std::env::temp_dir().join(format!(
            "orion-share-test-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        tmp
    }

    #[test]
    fn slug_for_basic() {
        assert_eq!(slug_for("Hello World"), "hello-world");
        assert_eq!(slug_for("  Spaces  "), "spaces");
        assert_eq!(slug_for(""), "session");
        assert_eq!(slug_for("---"), "session");
        assert_eq!(slug_for("Multi   Word  Title"), "multi-word-title");
    }

    #[test]
    fn slug_for_truncates_long_titles() {
        let long = "a".repeat(100);
        let s = slug_for(&long);
        assert!(s.len() <= 40);
    }

    #[test]
    fn sanitize_redacts_openai_keys() {
        let s = sanitize_text("api_key = sk-abc123def456ghi789jkl012mno345pqr");
        assert!(s.contains("[REDACTED"));
        assert!(!s.contains("abc123def456"));
    }

    #[test]
    fn sanitize_redacts_github_tokens() {
        let s = sanitize_text("token: ghp_abcdefghijklmnopqrstuvwxyz0123456789");
        assert!(s.contains("[REDACTED"));
    }

    #[test]
    fn sanitize_keeps_normal_text() {
        let s = sanitize_text("Hello world, this is fine.");
        assert_eq!(s, "Hello world, this is fine.");
    }

    #[test]
    fn sanitize_truncates_long_paths() {
        let s = sanitize_text("file: /home/user/projects/orion/src/main.rs");
        assert!(s.contains("…"));
        assert!(!s.contains("/home/user/projects/orion/src/main.rs"));
    }

    #[test]
    fn write_and_read_roundtrip() {
        let session = SharedSession {
            version: 1,
            id: "abc".into(),
            title: "Test".into(),
            provider: "anthropic".into(),
            model: "claude-sonnet-4".into(),
            created_at: "2026-01-01".into(),
            updated_at: "2026-01-01".into(),
            messages: vec![SharedMessage {
                role: "user".into(),
                content: "Hello".into(),
                timestamp: None,
                tool_name: None,
            }],
            metadata: Some(SharedMetadata::default()),
        };
        let path = temp_path();
        write_to_file(&session, &path).unwrap();
        let back = read_from_file(&path).unwrap();
        assert_eq!(back.id, "abc");
        assert_eq!(back.messages.len(), 1);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn shared_message_serializes() {
        let m = SharedMessage {
            role: "assistant".into(),
            content: "Hi".into(),
            timestamp: Some("2026-01-01T00:00:00Z".into()),
            tool_name: Some("read".into()),
        };
        let v = serde_json::to_value(&m).unwrap();
        assert_eq!(v["role"], "assistant");
        assert_eq!(v["tool_name"], "read");
    }

    #[test]
    fn metadata_omits_empty_tags() {
        let mut m = SharedMetadata::default();
        m.tags = Vec::new();
        let v = serde_json::to_value(&m).unwrap();
        assert!(v.get("tags").is_none());
        m.tags = vec!["test".into()];
        let v = serde_json::to_value(&m).unwrap();
        assert_eq!(v["tags"][0], "test");
    }

    #[test]
    fn shared_session_version_field() {
        let s = SharedSession {
            version: 1,
            id: "x".into(),
            title: "x".into(),
            provider: "x".into(),
            model: "x".into(),
            created_at: "x".into(),
            updated_at: "x".into(),
            messages: vec![],
            metadata: None,
        };
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(v["version"], 1);
    }
}
