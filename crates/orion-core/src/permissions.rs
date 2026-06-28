pub mod bash_risk;
pub mod store;
pub mod trust;

use globset::{Glob, GlobMatcher};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Allow,
    Ask,
    Deny,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub pattern: String,
    pub action: Action,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PermissionConfig {
    #[serde(default)]
    pub rules: HashMap<String, Vec<Rule>>,
    #[serde(default)]
    pub defaults: HashMap<String, Action>,
}

impl PermissionConfig {
    pub fn safe_defaults() -> Self {
        let mut defaults = HashMap::new();
        defaults.insert("read".into(), Action::Allow);
        defaults.insert("write".into(), Action::Ask);
        defaults.insert("edit".into(), Action::Ask);
        defaults.insert("bash".into(), Action::Ask);
        defaults.insert("grep".into(), Action::Allow);
        defaults.insert("glob".into(), Action::Allow);
        defaults.insert("todowrite".into(), Action::Allow);
        defaults.insert("question".into(), Action::Allow);
        defaults.insert("webfetch".into(), Action::Ask);
        defaults.insert("websearch".into(), Action::Ask);
        defaults.insert("mcp_*".into(), Action::Ask);
        Self {
            rules: HashMap::new(),
            defaults,
        }
    }

    pub fn permissive() -> Self {
        let mut defaults = HashMap::new();
        for tool in [
            "read",
            "write",
            "edit",
            "bash",
            "grep",
            "glob",
            "todowrite",
            "question",
            "webfetch",
            "websearch",
            "mcp_*",
        ] {
            defaults.insert(tool.into(), Action::Allow);
        }
        Self {
            rules: HashMap::new(),
            defaults,
        }
    }
}

pub struct PermissionEngine {
    config: RwLock<PermissionConfig>,
    matchers: RwLock<HashMap<String, Vec<(GlobMatcher, Action)>>>,
}

impl PermissionEngine {
    pub fn new(config: PermissionConfig) -> Self {
        let matchers = config
            .rules
            .iter()
            .map(|(tool, rules)| {
                let compiled: Vec<_> = rules
                    .iter()
                    .filter_map(|r| {
                        Glob::new(&r.pattern)
                            .ok()
                            .map(|g| (g.compile_matcher(), r.action))
                    })
                    .collect();
                (tool.clone(), compiled)
            })
            .collect();

        Self {
            config: RwLock::new(config),
            matchers: RwLock::new(matchers),
        }
    }

    pub fn check(&self, tool: &str, action_desc: &str) -> Action {
        let matchers = self.matchers.read().unwrap();
        if let Some(rules) = matchers.get(tool) {
            let mut last_match: Option<Action> = None;
            for (glob, action) in rules {
                if glob.is_match(action_desc) {
                    last_match = Some(*action);
                }
            }
            if let Some(a) = last_match {
                return a;
            }
        }
        drop(matchers);

        let cfg = self.config.read().unwrap();
        cfg.defaults.get(tool).copied().unwrap_or(Action::Ask)
    }

    /// Like [`check`](Self::check), but returns `Some(action)` ONLY when an
    /// explicit rule matched. Returns `None` when there is no matching rule, so
    /// callers (the Trust Engine) can apply risk heuristics instead of the
    /// blanket per-tool default.
    pub fn check_explicit(&self, tool: &str, action_desc: &str) -> Option<Action> {
        let matchers = self.matchers.read().unwrap();
        let rules = matchers.get(tool)?;
        let mut last_match: Option<Action> = None;
        for (glob, action) in rules {
            if glob.is_match(action_desc) {
                last_match = Some(*action);
            }
        }
        last_match
    }

    pub fn add_rule(&self, tool: &str, pattern: &str, action: Action) -> Result<(), String> {
        let glob = Glob::new(pattern)
            .map_err(|e| format!("invalid glob pattern: {e}"))?
            .compile_matcher();
        {
            let mut cfg = self.config.write().unwrap();
            cfg.rules
                .entry(tool.to_string())
                .or_default()
                .push(Rule {
                    pattern: pattern.to_string(),
                    action,
                });
        }
        self.matchers
            .write()
            .unwrap()
            .entry(tool.to_string())
            .or_default()
            .push((glob, action));
        Ok(())
    }

    pub fn snapshot(&self) -> PermissionConfig {
        self.config.read().unwrap().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_apply_when_no_rules() {
        let cfg = PermissionConfig::safe_defaults();
        let eng = PermissionEngine::new(cfg);
        assert_eq!(eng.check("bash", "rm -rf /"), Action::Ask);
        assert_eq!(eng.check("read", "any path"), Action::Allow);
    }

    #[test]
    fn last_match_wins_for_glob() {
        let mut cfg = PermissionConfig::safe_defaults();
        cfg.rules.insert(
            "bash".into(),
            vec![
                Rule {
                    pattern: "*".into(),
                    action: Action::Ask,
                },
                Rule {
                    pattern: "git status*".into(),
                    action: Action::Allow,
                },
            ],
        );
        let eng = PermissionEngine::new(cfg);
        assert_eq!(eng.check("bash", "git status"), Action::Allow);
        assert_eq!(eng.check("bash", "rm -rf /"), Action::Ask);
    }

    #[test]
    fn add_rule_is_queryable_immediately() {
        let eng = PermissionEngine::new(PermissionConfig::safe_defaults());
        eng.add_rule("bash", "npm test*", Action::Allow).unwrap();
        assert_eq!(eng.check("bash", "npm test --watch"), Action::Allow);
        assert_eq!(eng.check("bash", "rm -rf /"), Action::Ask);
    }

    #[test]
    fn permissive_defaults_allow_everything() {
        let eng = PermissionEngine::new(PermissionConfig::permissive());
        assert_eq!(eng.check("bash", "rm -rf /"), Action::Allow);
    }

    #[test]
    fn unknown_tool_defaults_to_ask() {
        let cfg = PermissionConfig::safe_defaults();
        let eng = PermissionEngine::new(cfg);
        assert_eq!(eng.check("unknown_tool", "anything"), Action::Ask);
    }
}
