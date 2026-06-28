//! The Trust Engine: decide tool permissions by *consequence*, not by tool.
//!
//! The core idea that makes ORION less prompt-heavy than opencode: an action
//! is auto-allowed when it is **safe and reversible**, and only prompts when it
//! is genuinely risky. Three signals feed the decision:
//!
//! 1. **Workspace boundary** — reads/edits inside the project `cwd` are
//!    snapshot-reversible (see [`crate::core::snapshot`]) and git-tracked, so
//!    they never prompt. Escapes (`/etc`, `$HOME` dotfiles, `..` out of tree)
//!    do prompt.
//! 2. **Bash risk** — commands are classified by AST (see
//!    [`crate::permissions::bash_risk`]) instead of glob-matching raw strings.
//! 3. **Learned rules** — explicit `allow`/`ask`/`deny` rules (including the
//!    user's "always allow" choices) always win.
//!
//! A single master **full access** switch bypasses all of this and allows
//! everything, for users who don't want to be asked at all.

use crate::agents::AgentSpec;
use crate::permissions::bash_risk::{classify_bash, RiskClass};
use crate::permissions::{Action, PermissionEngine};
use crate::tools::ToolCall;
use std::path::{Component, Path, PathBuf};

/// Verdict about where a path lives relative to the project.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PathVerdict {
    /// Resolves to a location under the project working directory.
    pub inside_workspace: bool,
    /// Outside the workspace AND a system/secret location (`/etc`, `~/.ssh`…).
    pub sensitive: bool,
}

/// Outcome of [`decide`] for one tool call.
#[derive(Debug, Clone)]
pub struct Decision {
    pub action: Action,
    /// Human-readable justification, shown in approval dialogs.
    pub reason: String,
    /// True when the effect is undoable (file edit inside workspace) — the
    /// dispatcher emits an `Undoable` event so the UI can offer "Undo".
    pub reversible: bool,
    /// Bash risk class, when the tool was `bash`.
    pub risk: Option<RiskClass>,
    /// Glob to persist if the user picks "always allow".
    pub matched_pattern: Option<String>,
}

impl Decision {
    fn allow(reason: impl Into<String>) -> Self {
        Self { action: Action::Allow, reason: reason.into(), reversible: false, risk: None, matched_pattern: None }
    }
    fn allow_reversible(reason: impl Into<String>) -> Self {
        Self { action: Action::Allow, reason: reason.into(), reversible: true, risk: None, matched_pattern: None }
    }
    fn ask(reason: impl Into<String>, pattern: Option<String>) -> Self {
        Self { action: Action::Ask, reason: reason.into(), reversible: false, risk: None, matched_pattern: pattern }
    }
    fn deny(reason: impl Into<String>) -> Self {
        Self { action: Action::Deny, reason: reason.into(), reversible: false, risk: None, matched_pattern: None }
    }
    fn from_action(a: Action, reason: impl Into<String>, pattern: Option<String>) -> Self {
        Self { action: a, reason: reason.into(), reversible: false, risk: None, matched_pattern: pattern }
    }
}

// --------------------------------------------------------------------------
// Path containment
// --------------------------------------------------------------------------

fn expand_home(raw: &str) -> PathBuf {
    if let Some(rest) = raw.strip_prefix("~/").or_else(|| raw.strip_prefix("$HOME/")) {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    if raw == "~" || raw == "$HOME" {
        if let Some(home) = dirs::home_dir() {
            return home;
        }
    }
    PathBuf::from(raw)
}

/// Fold `.` and `..` components lexically (no filesystem access).
fn lexical_normalize(p: &Path) -> PathBuf {
    let mut out: Vec<Component> = Vec::new();
    for comp in p.components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                match out.last() {
                    Some(Component::Normal(_)) => {
                        out.pop();
                    }
                    _ => out.push(comp),
                }
            }
            other => out.push(other),
        }
    }
    out.iter().collect()
}

/// Canonicalize a path that may not exist yet by canonicalizing its nearest
/// existing ancestor and re-appending the remaining components.
fn canonicalize_existing(p: &Path) -> PathBuf {
    let mut ancestor = p.to_path_buf();
    let mut tail: Vec<std::ffi::OsString> = Vec::new();
    loop {
        if let Ok(c) = ancestor.canonicalize() {
            let mut result = c;
            for part in tail.iter().rev() {
                result.push(part);
            }
            return result;
        }
        match ancestor.file_name() {
            Some(name) => tail.push(name.to_os_string()),
            None => return p.to_path_buf(),
        }
        if !ancestor.pop() {
            return p.to_path_buf();
        }
    }
}

fn is_sensitive(p: &Path) -> bool {
    let s = p.to_string_lossy();
    const SYSTEM_PREFIXES: &[&str] = &[
        "/etc", "/usr", "/bin", "/sbin", "/lib", "/sys", "/proc", "/dev", "/boot", "/root",
        "/var", "/opt",
    ];
    if SYSTEM_PREFIXES.iter().any(|pre| s.starts_with(pre)) {
        return true;
    }
    // Dotfiles / secret dirs under $HOME (e.g. ~/.ssh, ~/.aws, ~/.config).
    if let Some(home) = dirs::home_dir() {
        if let Ok(rest) = p.strip_prefix(&home) {
            if rest
                .components()
                .next()
                .and_then(|c| c.as_os_str().to_str())
                .map(|c| c.starts_with('.'))
                .unwrap_or(false)
            {
                return true;
            }
        }
    }
    false
}

/// Classify where `raw` (relative to `cwd`) lives.
pub fn classify_path(cwd: &Path, raw: &str) -> PathVerdict {
    let expanded = expand_home(raw);
    let joined = if expanded.is_absolute() { expanded } else { cwd.join(expanded) };
    let normalized = lexical_normalize(&joined);

    let canon_cwd = lexical_normalize(&cwd.canonicalize().unwrap_or_else(|_| cwd.to_path_buf()));
    let canon_target = canonicalize_existing(&normalized);

    let inside = canon_target.starts_with(&canon_cwd);
    let sensitive = !inside && is_sensitive(&normalized);
    PathVerdict { inside_workspace: inside, sensitive }
}

/// Convenience: does `raw` resolve inside the project?
pub fn is_inside_workspace(cwd: &Path, raw: &str) -> bool {
    classify_path(cwd, raw).inside_workspace
}

// --------------------------------------------------------------------------
// Bash combined explicit-rule + risk evaluation
// --------------------------------------------------------------------------

fn most_restrictive(a: Action, b: Action) -> Action {
    match (a, b) {
        (Action::Deny, _) | (_, Action::Deny) => Action::Deny,
        (Action::Ask, _) | (_, Action::Ask) => Action::Ask,
        _ => Action::Allow,
    }
}

/// Evaluate a bash command string: explicit per-segment rules win, otherwise
/// fall back to AST risk classification. Returns the folded action plus the
/// worst risk class seen (for display).
pub fn bash_action(
    engine: &PermissionEngine,
    command_string: &str,
    cwd: &Path,
) -> (Action, Option<RiskClass>) {
    use crate::tools::bash_parser::parse_commands;
    let parsed = parse_commands(command_string);
    if parsed.is_empty() {
        return (engine.check("bash", command_string), None);
    }
    // Explicit per-segment rules win: any segment denied -> deny the whole call.
    let mut action = Action::Allow;
    let mut any_unruled = false;
    for cmd in &parsed {
        match engine.check_explicit("bash", &cmd.full_text) {
            Some(Action::Deny) => return (Action::Deny, None),
            Some(Action::Allow) => {}
            Some(Action::Ask) => action = most_restrictive(action, Action::Ask),
            None => any_unruled = true,
        }
    }
    // For segments without an explicit rule, fall back to AST risk. Classify the
    // whole string at once so redirects and `curl | sh` chains are caught.
    let mut risk = None;
    if any_unruled {
        let r = classify_bash(command_string, cwd);
        risk = Some(r);
        action = most_restrictive(action, r.to_action());
    }
    (action, risk)
}

// --------------------------------------------------------------------------
// The orchestrator
// --------------------------------------------------------------------------

const READ_ONLY_TOOLS: &[&str] = &["read", "grep", "glob", "list", "lsp", "todowrite"];
const FILE_WRITE_TOOLS: &[&str] = &["write", "edit", "apply_patch"];

/// Decide the permission for one tool call.
///
/// `tool` must already be the short tool name (see `tool_name` in dispatch).
/// Evaluation order (most restrictive layer wins):
/// 1. Agent gate — a read-only agent (e.g. Plan) can hard-deny a tool.
/// 2. Full access — the master switch allows everything else.
/// 3. Explicit/learned rules — `allow`/`ask`/`deny` the user configured.
/// 4. Trust heuristics — workspace boundary + bash risk.
pub fn decide(
    engine: &PermissionEngine,
    agent: Option<&AgentSpec>,
    full_access: bool,
    tool: &str,
    call: &ToolCall,
    action_desc: &str,
    cwd: &Path,
) -> Decision {
    // 1. Agent gate (kept even under full access: Plan mode is a hard guarantee).
    if let Some(spec) = agent {
        if !spec.can_use_tool(tool) {
            return Decision::deny(format!("the {} agent cannot use `{}`", spec.name, tool));
        }
    }

    // 2. Master "full access" switch.
    if full_access {
        return Decision::allow("full access mode is on");
    }

    // 3. Read-only tools: explicit rule may override, otherwise always allow.
    if READ_ONLY_TOOLS.contains(&tool) {
        if let Some(a) = engine.check_explicit(tool, action_desc) {
            return Decision::from_action(a, "matched a permission rule", Some(action_desc.to_string()));
        }
        return Decision::allow("read-only tool");
    }

    // 4. File-writing tools: reversible inside the workspace.
    if FILE_WRITE_TOOLS.contains(&tool) {
        match engine.check_explicit(tool, action_desc) {
            Some(Action::Deny) => return Decision::deny("matched a deny rule"),
            Some(Action::Allow) => {
                let inside = write_targets_inside(call, cwd);
                let mut d = Decision::allow("matched an allow rule");
                d.reversible = inside;
                return d;
            }
            Some(Action::Ask) => {
                return Decision::ask("matched an ask rule", Some(action_desc.to_string()));
            }
            None => {}
        }
        if write_targets_inside(call, cwd) {
            return Decision::allow_reversible("edits a file inside the project (undoable)");
        }
        return Decision::ask(
            "writes to a file outside the project",
            Some(action_desc.to_string()),
        );
    }

    // 5. Bash: explicit rules per segment, else AST risk classification.
    if tool == "bash" {
        let raw = call
            .arguments
            .get("command")
            .and_then(|v| v.as_str())
            .unwrap_or(action_desc);
        let (action, risk) = bash_action(engine, raw, cwd);
        let reason = match risk {
            Some(r) => format!("shell command ({})", r.label()),
            None => "shell command".to_string(),
        };
        let pattern = sticky_pattern_for(call, raw);
        return Decision { action, reason, reversible: false, risk, matched_pattern: Some(pattern) };
    }

    // 6. Everything else (webfetch, websearch, mcp_*, unknown): rule or default.
    if let Some(a) = engine.check_explicit(tool, action_desc) {
        return Decision::from_action(a, "matched a permission rule", Some(action_desc.to_string()));
    }
    Decision::from_action(
        engine.check(tool, action_desc),
        "default policy for this tool",
        Some(action_desc.to_string()),
    )
}

/// Are all of a write/edit/apply_patch call's targets inside the workspace?
fn write_targets_inside(call: &ToolCall, cwd: &Path) -> bool {
    let targets = crate::core::snapshot::SnapshotManager::extract_targets(
        std::slice::from_ref(call),
        cwd,
    );
    !targets.is_empty()
        && targets
            .iter()
            .all(|p| is_inside_workspace(cwd, &p.to_string_lossy()))
}

/// Build a *scoped* glob for "always allow" — never the catch-all `*`.
pub fn sticky_pattern_for(call: &ToolCall, action_desc: &str) -> String {
    if call.name == "bash" {
        // First two words + "*": `npm test --watch` -> `npm test*`.
        let head: Vec<&str> = action_desc.split_whitespace().take(2).collect();
        if head.is_empty() {
            action_desc.to_string()
        } else {
            format!("{}*", head.join(" "))
        }
    } else {
        action_desc.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::permissions::{PermissionConfig, PermissionEngine, Rule};
    use serde_json::json;

    fn engine_default() -> PermissionEngine {
        PermissionEngine::new(PermissionConfig::safe_defaults())
    }

    fn tmp_workspace() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("trust-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.canonicalize().unwrap()
    }

    #[test]
    fn path_inside_vs_outside() {
        let cwd = tmp_workspace();
        assert!(classify_path(&cwd, "src/main.rs").inside_workspace);
        assert!(classify_path(&cwd, "./a/b/c.txt").inside_workspace);

        let esc = classify_path(&cwd, "../../etc/passwd");
        assert!(!esc.inside_workspace);

        let etc = classify_path(&cwd, "/etc/passwd");
        assert!(!etc.inside_workspace);
        assert!(etc.sensitive);
    }

    #[test]
    fn home_dotfiles_are_sensitive() {
        let cwd = tmp_workspace();
        let v = classify_path(&cwd, "~/.ssh/id_rsa");
        assert!(!v.inside_workspace);
        assert!(v.sensitive);
    }

    #[test]
    fn write_inside_is_allow_reversible() {
        let cwd = tmp_workspace();
        let eng = engine_default();
        let call = ToolCall {
            id: "1".into(),
            name: "write".into(),
            arguments: json!({"path": "src/lib.rs", "content": "x"}),
        };
        let d = decide(&eng, None, false, "write", &call, "src/lib.rs", &cwd);
        assert_eq!(d.action, Action::Allow);
        assert!(d.reversible);
    }

    #[test]
    fn write_outside_asks() {
        let cwd = tmp_workspace();
        let eng = engine_default();
        let call = ToolCall {
            id: "1".into(),
            name: "write".into(),
            arguments: json!({"path": "/etc/hosts", "content": "x"}),
        };
        let d = decide(&eng, None, false, "write", &call, "/etc/hosts", &cwd);
        assert_eq!(d.action, Action::Ask);
        assert!(!d.reversible);
    }

    #[test]
    fn read_is_always_allowed() {
        let cwd = tmp_workspace();
        let eng = engine_default();
        let call = ToolCall { id: "1".into(), name: "read".into(), arguments: json!({"path": "/etc/passwd"}) };
        let d = decide(&eng, None, false, "read", &call, "/etc/passwd", &cwd);
        assert_eq!(d.action, Action::Allow);
    }

    #[test]
    fn full_access_allows_everything() {
        let cwd = tmp_workspace();
        let eng = engine_default();
        let call = ToolCall {
            id: "1".into(),
            name: "bash".into(),
            arguments: json!({"command": "rm -rf /"}),
        };
        let d = decide(&eng, None, true, "bash", &call, "rm -rf /", &cwd);
        assert_eq!(d.action, Action::Allow);
    }

    #[test]
    fn bash_readonly_allows_destructive_asks() {
        let cwd = tmp_workspace();
        let eng = engine_default();
        let safe = ToolCall { id: "1".into(), name: "bash".into(), arguments: json!({"command": "ls -la"}) };
        assert_eq!(decide(&eng, None, false, "bash", &safe, "ls -la", &cwd).action, Action::Allow);

        let danger = ToolCall { id: "2".into(), name: "bash".into(), arguments: json!({"command": "rm -rf node_modules"}) };
        assert_eq!(decide(&eng, None, false, "bash", &danger, "rm -rf node_modules", &cwd).action, Action::Ask);
    }

    #[test]
    fn learned_rule_beats_heuristic() {
        let cwd = tmp_workspace();
        let mut cfg = PermissionConfig::safe_defaults();
        cfg.rules.insert(
            "bash".into(),
            vec![Rule { pattern: "rm *".into(), action: Action::Allow }],
        );
        let eng = PermissionEngine::new(cfg);
        let danger = ToolCall { id: "1".into(), name: "bash".into(), arguments: json!({"command": "rm file.txt"}) };
        // explicit allow rule overrides the Destructive heuristic
        assert_eq!(decide(&eng, None, false, "bash", &danger, "rm file.txt", &cwd).action, Action::Allow);
    }

    #[test]
    fn plan_agent_denies_bash() {
        use crate::agents::AgentRegistry;
        let cwd = tmp_workspace();
        let eng = engine_default();
        let reg = AgentRegistry::with_builtins();
        let plan = reg.get("plan").unwrap().clone();
        let call = ToolCall { id: "1".into(), name: "bash".into(), arguments: json!({"command": "ls"}) };
        let d = decide(&eng, Some(&plan), false, "bash", &call, "ls", &cwd);
        assert_eq!(d.action, Action::Deny);
    }
}
