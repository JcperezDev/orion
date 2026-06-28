//! Skill system — load `SKILL.md` files for on-demand instruction injection.
//!
//! Skills are folders containing a `SKILL.md` file with YAML frontmatter.
//! Format is compatible with Claude's skill convention:
//!
//! ```markdown
//! ---
//! name: my-skill
//! description: One-line description of what this skill does.
//! license: MIT
//! compatibility: orion>=0.1.0
//! metadata:
//!   audience: developers
//! ---
//!
//! # My Skill
//!
//! Instructions in plain markdown. The agent loads the skill on demand
//! when its description matches the user's request.
//! ```
//!
//! Skills are loaded from these directories (in order):
//! 1. `.opencode/skills/`
//! 2. `.claude/skills/`
//! 3. `.agents/skills/`
//! 4. `~/.orion/skills/`
//! 5. `~/.claude/skills/`

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Parsed SKILL.md file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub name: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compatibility: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audience: Option<String>,
    /// Path the skill was loaded from.
    #[serde(skip)]
    pub path: PathBuf,
    /// The body of the SKILL.md (everything after the frontmatter).
    #[serde(skip)]
    pub body: String,
}

/// In-memory registry of discovered skills.
#[derive(Debug, Default, Clone)]
pub struct SkillRegistry {
    skills: HashMap<String, Skill>,
}

impl SkillRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Discover skills from a list of directories.
    pub fn discover<I, P>(&mut self, dirs: I) -> Result<usize>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        let mut count = 0;
        for dir in dirs {
            let dir = dir.as_ref();
            if !dir.exists() {
                continue;
            }
            for entry in std::fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    let skill_md = path.join("SKILL.md");
                    if skill_md.exists() {
                        if let Ok(skill) = parse_skill_file(&skill_md) {
                            self.skills.insert(skill.name.clone(), skill);
                            count += 1;
                        }
                    }
                }
            }
        }
        Ok(count)
    }

    /// Auto-discover skills from standard locations.
    pub fn auto_discover(&mut self) -> Result<usize> {
        let mut roots: Vec<PathBuf> = Vec::new();
        if let Ok(cwd) = std::env::current_dir() {
            for sub in [".opencode/skills", ".claude/skills", ".agents/skills"] {
                roots.push(cwd.join(sub));
            }
        }
        if let Some(home) = dirs::home_dir() {
            roots.push(home.join(".orion/skills"));
            roots.push(home.join(".claude/skills"));
            roots.push(home.join(".opencode/skills"));
        }
        self.discover(roots)
    }

    /// Register a skill manually.
    pub fn register(&mut self, skill: Skill) {
        self.skills.insert(skill.name.clone(), skill);
    }

    /// Get a skill by name.
    pub fn get(&self, name: &str) -> Option<&Skill> {
        self.skills.get(name)
    }

    /// All registered skills.
    pub fn list(&self) -> Vec<&Skill> {
        let mut v: Vec<&Skill> = self.skills.values().collect();
        v.sort_by(|a, b| a.name.cmp(&b.name));
        v
    }

    /// Count registered skills.
    pub fn len(&self) -> usize {
        self.skills.len()
    }

    pub fn is_empty(&self) -> bool {
        self.skills.is_empty()
    }

    /// Find skills whose description matches a query (case-insensitive keyword match).
    pub fn search(&self, query: &str) -> Vec<&Skill> {
        let query_lower = query.to_lowercase();
        let keywords: Vec<&str> = query_lower.split_whitespace().collect();
        let mut scored: Vec<(&Skill, usize)> = self
            .skills
            .values()
            .map(|s| {
                let desc = s.description.to_lowercase();
                let name = s.name.to_lowercase();
                let mut score = 0;
                for kw in &keywords {
                    if desc.contains(kw) {
                        score += 2;
                    }
                    if name.contains(kw) {
                        score += 3;
                    }
                    if s.body.to_lowercase().contains(kw) {
                        score += 1;
                    }
                }
                (s, score)
            })
            .filter(|(_, s)| *s > 0)
            .collect();
        scored.sort_by(|a, b| b.1.cmp(&a.1));
        scored.into_iter().map(|(s, _)| s).collect()
    }
}

/// Parse a SKILL.md file. Returns the parsed Skill.
pub fn parse_skill_file(path: &Path) -> Result<Skill> {
    let raw = std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    parse_skill_str(&raw, path.to_path_buf())
}

/// Parse a SKILL.md from a string. `path` is just for tracking.
pub fn parse_skill_str(raw: &str, path: PathBuf) -> Result<Skill> {
    let (frontmatter, body) = split_frontmatter(raw)?;
    let mut skill: Skill = serde_yaml_or_json_from_str(&frontmatter).with_context(|| "parsing YAML frontmatter")?;
    skill.body = body;
    skill.path = path;
    if skill.name.is_empty() {
        anyhow::bail!("SKILL.md missing 'name' in frontmatter");
    }
    if skill.description.is_empty() {
        anyhow::bail!("SKILL.md missing 'description' in frontmatter");
    }
    Ok(skill)
}

fn split_frontmatter(raw: &str) -> Result<(String, String)> {
    let trimmed = raw.trim_start();
    let rest = trimmed
        .strip_prefix("---")
        .ok_or_else(|| anyhow::anyhow!("missing '---' frontmatter opener"))?;
    let after_open = rest
        .strip_prefix('\n')
        .or_else(|| rest.strip_prefix("\r\n"))
        .unwrap_or(rest);
    let close = after_open
        .find("\n---")
        .ok_or_else(|| anyhow::anyhow!("missing '---' frontmatter closer"))?;
    let frontmatter = &after_open[..close];
    let after_close = &after_open[close + 4..];
    let body_start = after_close
        .strip_prefix('\n')
        .or_else(|| after_close.strip_prefix("\r\n"))
        .unwrap_or(after_close);
    Ok((frontmatter.to_string(), body_start.to_string()))
}

/// Try YAML first, fall back to JSON.
fn serde_yaml_or_json_from_str(s: &str) -> Result<Skill> {
    // First try YAML; we don't add a dep just for this so we use a tiny
    // hand-rolled parser limited to flat frontmatter (name/description/etc.).
    parse_simple_yaml(s)
}

/// Flat YAML frontmatter parser. Handles `key: value` lines, simple strings,
/// arrays via `[a, b]`, and nested `metadata:` blocks.
fn parse_simple_yaml(s: &str) -> Result<Skill> {
    let mut skill = Skill {
        name: String::new(),
        description: String::new(),
        license: None,
        compatibility: None,
        audience: None,
        path: PathBuf::new(),
        body: String::new(),
    };
    let mut in_metadata = false;
    for line in s.lines() {
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            continue;
        }
        let indent_level = trimmed.chars().take_while(|c| *c == ' ').count();
        let content = trimmed.trim_start();
        if indent_level == 0 && content.starts_with("metadata:") {
            in_metadata = true;
            continue;
        }
        if indent_level == 0 {
            in_metadata = false;
        }
        if in_metadata && indent_level > 0 {
            // Skip metadata for now; we capture `audience` if present at top level.
            if let Some(rest) = content.strip_prefix("audience:") {
                skill.audience = Some(unquote(rest.trim()));
            }
            continue;
        }
        if let Some(rest) = content.strip_prefix("name:") {
            skill.name = unquote(rest.trim());
        } else if let Some(rest) = content.strip_prefix("description:") {
            skill.description = unquote(rest.trim());
        } else if let Some(rest) = content.strip_prefix("license:") {
            skill.license = Some(unquote(rest.trim()));
        } else if let Some(rest) = content.strip_prefix("compatibility:") {
            skill.compatibility = Some(unquote(rest.trim()));
        } else if let Some(rest) = content.strip_prefix("audience:") {
            skill.audience = Some(unquote(rest.trim()));
        }
    }
    Ok(skill)
}

fn unquote(s: &str) -> String {
    let s = s.trim();
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        if s.len() >= 2 {
            return s[1..s.len() - 1].to_string();
        }
    }
    s.to_string()
}

/// Render a skill's body into a prompt-friendly summary.
pub fn render_skill_summary(skill: &Skill) -> String {
    let mut out = String::new();
    out.push_str(&format!("# {} ({})\n\n", skill.name, skill.description));
    if let Some(license) = &skill.license {
        out.push_str(&format!("_License: {license}_\n\n"));
    }
    let body = skill.body.trim();
    let truncated = if body.len() > 1500 {
        format!("{}\n\n[…truncated, {} chars total]", &body[..1500], body.len())
    } else {
        body.to_string()
    };
    out.push_str(&truncated);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_skill(dir: &Path, name: &str, description: &str, body: &str) -> std::path::PathBuf {
        let skill_dir = dir.join(name);
        std::fs::create_dir_all(&skill_dir).unwrap();
        let path = skill_dir.join("SKILL.md");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(
            f,
            "---\nname: {name}\ndescription: {description}\n---\n\n{body}"
        )
        .unwrap();
        path
    }

    fn temp_root() -> PathBuf {
        let tmp = std::env::temp_dir().join(format!(
            "orion-skills-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&tmp).unwrap();
        tmp
    }

    #[test]
    fn parse_simple_skill() {
        let raw = "---\nname: my-skill\ndescription: Does cool things\n---\n\n# My Skill\n\nInstructions here.";
        let skill = parse_skill_str(raw, PathBuf::from("/tmp/x")).unwrap();
        assert_eq!(skill.name, "my-skill");
        assert_eq!(skill.description, "Does cool things");
        assert!(skill.body.contains("Instructions here"));
    }

    #[test]
    fn parse_with_quoted_values() {
        let raw = "---\nname: \"quoted\"\ndescription: 'single quoted'\n---\nbody";
        let skill = parse_skill_str(raw, PathBuf::from("/tmp")).unwrap();
        assert_eq!(skill.name, "quoted");
        assert_eq!(skill.description, "single quoted");
    }

    #[test]
    fn parse_with_optional_fields() {
        let raw = "---\nname: x\ndescription: y\nlicense: MIT\ncompatibility: orion>=0.1\naudience: developers\n---\nbody";
        let skill = parse_skill_str(raw, PathBuf::from("/tmp")).unwrap();
        assert_eq!(skill.license.as_deref(), Some("MIT"));
        assert_eq!(skill.compatibility.as_deref(), Some("orion>=0.1"));
        assert_eq!(skill.audience.as_deref(), Some("developers"));
    }

    #[test]
    fn parse_missing_name_fails() {
        let raw = "---\ndescription: x\n---\nbody";
        let r = parse_skill_str(raw, PathBuf::from("/tmp"));
        assert!(r.is_err());
    }

    #[test]
    fn parse_missing_frontmatter_fails() {
        let raw = "# no frontmatter\nbody";
        let r = parse_skill_str(raw, PathBuf::from("/tmp"));
        assert!(r.is_err());
    }

    #[test]
    fn discover_finds_skills_in_directory() {
        let root = temp_root();
        write_skill(&root, "skill-a", "Alpha", "alpha body");
        write_skill(&root, "skill-b", "Beta", "beta body");
        write_skill(&root, "not-a-skill", "", "missing description"); // should fail to parse

        let mut reg = SkillRegistry::new();
        let n = reg.discover([&root]).unwrap();
        // 2 skills parse; one fails because description is empty
        assert_eq!(n, 2);
        assert_eq!(reg.len(), 2);
        assert!(reg.get("skill-a").is_some());
        assert!(reg.get("skill-b").is_some());

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn search_ranks_by_relevance() {
        let mut reg = SkillRegistry::new();
        reg.register(Skill {
            name: "rust-testing".into(),
            description: "Write Rust tests".into(),
            license: None,
            compatibility: None,
            audience: None,
            path: PathBuf::new(),
            body: String::new(),
        });
        reg.register(Skill {
            name: "python-helpers".into(),
            description: "Python utilities".into(),
            license: None,
            compatibility: None,
            audience: None,
            path: PathBuf::new(),
            body: String::new(),
        });
        let hits = reg.search("rust testing");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].name, "rust-testing");
    }

    #[test]
    fn search_no_match_returns_empty() {
        let mut reg = SkillRegistry::new();
        reg.register(Skill {
            name: "x".into(),
            description: "y".into(),
            license: None,
            compatibility: None,
            audience: None,
            path: PathBuf::new(),
            body: String::new(),
        });
        assert!(reg.search("zzz").is_empty());
    }

    #[test]
    fn render_summary_includes_metadata() {
        let skill = Skill {
            name: "demo".into(),
            description: "Demo skill".into(),
            license: Some("MIT".into()),
            compatibility: None,
            audience: None,
            path: PathBuf::new(),
            body: "Some content.".into(),
        };
        let s = render_skill_summary(&skill);
        assert!(s.contains("# demo"));
        assert!(s.contains("License: MIT"));
        assert!(s.contains("Some content"));
    }

    #[test]
    fn render_truncates_long_body() {
        let long = "a".repeat(2000);
        let skill = Skill {
            name: "long".into(),
            description: "Long".into(),
            license: None,
            compatibility: None,
            audience: None,
            path: PathBuf::new(),
            body: long,
        };
        let s = render_skill_summary(&skill);
        assert!(s.contains("truncated"));
    }
}
