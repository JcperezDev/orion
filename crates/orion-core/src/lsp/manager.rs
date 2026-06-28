//! LSP server configurations per file extension.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Configured language server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspServerConfig {
    /// Command to run (e.g. "rust-analyzer").
    pub command: String,
    /// Command-line args.
    #[serde(default)]
    pub args: Vec<String>,
    /// Environment variables to set.
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// File extensions this server handles (e.g. ["rs"]). Used for routing.
    #[serde(default)]
    pub extensions: Vec<String>,
    /// Glob patterns this server handles. If non-empty, takes precedence over `extensions`.
    #[serde(default)]
    pub patterns: Vec<String>,
    /// File extension this server handles (alias for `extensions[0]`).
    #[serde(default)]
    pub language_id: Option<String>,
}

impl LspServerConfig {
    /// Match a file against this server's filters. Returns true if the server
    /// should handle this file.
    pub fn matches(&self, path: &Path) -> bool {
        if !self.patterns.is_empty() {
            let path_str = path.to_string_lossy();
            for pat in &self.patterns {
                if glob_matches(pat, &path_str) {
                    return true;
                }
            }
            return false;
        }
        if !self.extensions.is_empty() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                let ext_with_dot = ext.trim_start_matches('.');
                return self
                    .extensions
                    .iter()
                    .any(|e| e.trim_start_matches('.') == ext_with_dot);
            }
        }
        if let Some(lang) = &self.language_id {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                return lang.eq_ignore_ascii_case(ext);
            }
        }
        false
    }
}

fn glob_matches(pattern: &str, s: &str) -> bool {
    // Simple glob: supports `*` and `**`. Falls back to equality.
    if !pattern.contains('*') {
        return pattern == s;
    }
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.len() == 2 {
        let (prefix, suffix) = (parts[0], parts[1]);
        return s.starts_with(prefix) && s.ends_with(suffix) && s.len() >= prefix.len() + suffix.len();
    }
    // More complex patterns: fall back to equality.
    pattern == s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_by_extension() {
        let cfg = LspServerConfig {
            command: "rust-analyzer".into(),
            args: vec![],
            env: HashMap::new(),
            extensions: vec!["rs".into()],
            patterns: vec![],
            language_id: None,
        };
        assert!(cfg.matches(Path::new("/tmp/foo.rs")));
        assert!(!cfg.matches(Path::new("/tmp/foo.py")));
    }

    #[test]
    fn matches_by_pattern() {
        let cfg = LspServerConfig {
            command: "tsserver".into(),
            args: vec![],
            env: HashMap::new(),
            extensions: vec![],
            patterns: vec!["*.ts".into()],
            language_id: None,
        };
        assert!(cfg.matches(Path::new("src/main.ts")));
        assert!(!cfg.matches(Path::new("src/main.js")));
    }

    #[test]
    fn matches_by_language_id() {
        let cfg = LspServerConfig {
            command: "pyright".into(),
            args: vec![],
            env: HashMap::new(),
            extensions: vec![],
            patterns: vec![],
            language_id: Some("py".into()),
        };
        assert!(cfg.matches(Path::new("foo.py")));
        assert!(!cfg.matches(Path::new("foo.txt")));
    }

    #[test]
    fn no_match_returns_false() {
        let cfg = LspServerConfig {
            command: "x".into(),
            args: vec![],
            env: HashMap::new(),
            extensions: vec!["rs".into()],
            patterns: vec![],
            language_id: None,
        };
        assert!(!cfg.matches(Path::new("foo")));
        assert!(!cfg.matches(Path::new("foo.unknown")));
    }
}
