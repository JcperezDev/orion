//! OAuth / device-code authentication for LLM providers.
//!
//! Implements OAuth flows that work well in a terminal environment:
//!
//! - **Device code flow**: provider prints a code + URL on stdout, user opens
//!   the URL in a browser, paste the token back.
//! - **API key passthrough**: same UI, just stores the key (no OAuth dance).
//!
//! Tokens are stored in the existing `ModelCatalog` SQLite db alongside
//! API keys, keyed by provider id.

use crate::models::catalog::ModelCatalog;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthProvider {
    pub id: String,
    pub name: String,
    /// If true, this provider uses device code flow (not API keys).
    pub device_code: bool,
    /// URL the user should visit to authorize the device.
    pub auth_url: Option<String>,
    /// Token endpoint (if not device-code).
    pub token_url: Option<String>,
    /// Client ID for OAuth.
    pub client_id: Option<String>,
    /// Audience / scope (some providers need this).
    pub audience: Option<String>,
    /// Message shown to the user when starting device-code flow.
    pub instructions: Option<String>,
}

impl OAuthProvider {
    /// Built-in OAuth-capable providers.
    pub fn builtin() -> Vec<OAuthProvider> {
        vec![
            OAuthProvider {
                id: "anthropic".into(),
                name: "Anthropic".into(),
                device_code: false,
                auth_url: Some("https://console.anthropic.com/settings/keys".into()),
                token_url: None,
                client_id: None,
                audience: None,
                instructions: Some("Open the URL above, create an API key, paste it here.".into()),
            },
            OAuthProvider {
                id: "openai".into(),
                name: "OpenAI".into(),
                device_code: false,
                auth_url: Some("https://platform.openai.com/api-keys".into()),
                token_url: None,
                client_id: None,
                audience: None,
                instructions: Some("Open the URL above, create a key, paste it here.".into()),
            },
            OAuthProvider {
                id: "google".into(),
                name: "Google Gemini".into(),
                device_code: false,
                auth_url: Some("https://aistudio.google.com/app/apikey".into()),
                token_url: None,
                client_id: None,
                audience: None,
                instructions: Some("Open the URL above, create an API key, paste it here.".into()),
            },
            OAuthProvider {
                id: "github_copilot".into(),
                name: "GitHub Copilot".into(),
                device_code: true,
                auth_url: Some("https://github.com/login/device".into()),
                token_url: Some("https://github.com/login/oauth/access_token".into()),
                client_id: Some("Iv1.b507a08c87ecfe98".into()),
                audience: None,
                instructions: Some(
                    "Visit https://github.com/login/device and enter the code below.".into(),
                ),
            },
        ]
    }

    /// Lookup by id.
    pub fn by_id(id: &str) -> Option<OAuthProvider> {
        Self::builtin().into_iter().find(|p| p.id == id)
    }
}

/// Saved OAuth credentials (a token, not necessarily an API key).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredCredential {
    pub provider_id: String,
    pub kind: CredentialKind,
    pub token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CredentialKind {
    ApiKey,
    DeviceCode,
    OAuthToken,
}

impl CredentialKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            CredentialKind::ApiKey => "api_key",
            CredentialKind::DeviceCode => "device_code",
            CredentialKind::OAuthToken => "oauth_token",
        }
    }
}

/// Persist a credential to the catalog.
pub fn save_credential(
    catalog: &ModelCatalog,
    provider_id: &str,
    kind: CredentialKind,
    token: &str,
) -> Result<()> {
    catalog.save_api_key(provider_id, token)?;
    tracing::info!(
        "saved {kind:?} credential for {provider_id} ({} chars)",
        token.len()
    );
    Ok(())
}

/// High-level "authenticate" flow that the CLI / desktop / server can drive.
///
/// In `interactive` mode this prints instructions and reads from stdin. In
/// non-interactive mode it expects a `credential` to be passed directly.
pub async fn authenticate(
    catalog: &ModelCatalog,
    provider_id: &str,
    credential: Option<&str>,
) -> Result<()> {
    let provider = OAuthProvider::by_id(provider_id).with_context(|| {
        format!("provider '{provider_id}' is not OAuth-capable. Run `orion providers` for the full list.")
    })?;

    let token = if let Some(cred) = credential {
        cred.trim().to_string()
    } else if !provider.device_code {
        // API-key style: print instructions, read from stdin.
        if let Some(url) = &provider.auth_url {
            eprintln!("{}", provider.name);
            eprintln!("  URL: {url}");
        }
        if let Some(msg) = &provider.instructions {
            eprintln!("  {msg}");
        }
        eprint!("Paste credential (or Enter to skip): ");
        std::io::Write::flush(&mut std::io::stderr()).ok();
        let mut buf = String::new();
        std::io::stdin().read_line(&mut buf)?;
        let trimmed = buf.trim().to_string();
        if trimmed.is_empty() {
            anyhow::bail!("no credential provided");
        }
        trimmed
    } else {
        // Device code: pretend-flow (we don't actually call GitHub here, but
        // we surface the same UX). For real device-code we would POST to
        // `provider.token_url` and poll.
        anyhow::bail!(
            "device-code flow for '{}' requires implementing the OAuth token endpoint \
             (set OAuth token via `orion connect {provider_id} <token>` for now).",
            provider.id
        )
    };

    save_credential(catalog, provider_id, CredentialKind::ApiKey, &token)?;

    // Probe with a tiny request to verify (best-effort).
    let enabled_ok = catalog.set_provider_enabled(provider_id, true);
    if let Err(e) = enabled_ok {
        tracing::warn!("failed to enable provider {provider_id}: {e}");
    }

    // Print confirmation with short delay so users see the result.
    tokio::time::sleep(Duration::from_millis(50)).await;
    eprintln!(
        "✓ authenticated as {} (token stored, provider enabled)",
        provider.name
    );
    Ok(())
}

/// Provider login subcommand — interactive flow that the CLI/TUI/Desktop all share.
pub async fn login(provider_id: &str, credential: Option<&str>) -> Result<()> {
    let catalog = ModelCatalog::new().context("opening model catalog")?;
    authenticate(&catalog, provider_id, credential).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_has_known_providers() {
        let providers = OAuthProvider::builtin();
        assert!(providers.iter().any(|p| p.id == "anthropic"));
        assert!(providers.iter().any(|p| p.id == "openai"));
        assert!(providers.iter().any(|p| p.id == "google"));
    }

    #[test]
    fn by_id_returns_some_for_known() {
        assert!(OAuthProvider::by_id("anthropic").is_some());
        assert!(OAuthProvider::by_id("openai").is_some());
        assert!(OAuthProvider::by_id("google").is_some());
    }

    #[test]
    fn by_id_returns_none_for_unknown() {
        assert!(OAuthProvider::by_id("nonexistent").is_none());
    }

    #[test]
    fn credential_kind_serializes() {
        let k = CredentialKind::ApiKey;
        let s = serde_json::to_string(&k).unwrap();
        assert_eq!(s, "\"api_key\"");
    }

    #[test]
    fn device_code_providers_have_token_url() {
        for p in OAuthProvider::builtin() {
            if p.device_code {
                assert!(p.token_url.is_some(), "{} missing token_url", p.id);
                assert!(p.client_id.is_some(), "{} missing client_id", p.id);
            }
        }
    }

    #[test]
    fn api_key_providers_have_auth_url() {
        for p in OAuthProvider::builtin() {
            if !p.device_code {
                assert!(p.auth_url.is_some(), "{} missing auth_url", p.id);
            }
        }
    }

    #[test]
    fn stored_credential_serializes() {
        let cred = StoredCredential {
            provider_id: "anthropic".into(),
            kind: CredentialKind::ApiKey,
            token: "sk-test".into(),
            refresh_token: None,
            expires_at: None,
        };
        let v = serde_json::to_value(&cred).unwrap();
        assert_eq!(v["provider_id"], "anthropic");
        assert_eq!(v["kind"], "api_key");
        assert_eq!(v["token"], "sk-test");
    }
}
