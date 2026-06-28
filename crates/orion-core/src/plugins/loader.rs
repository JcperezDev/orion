use crate::mcp::client::McpRegistry;
use crate::plugins::{ExternalTool, PluginDescriptor};
use crate::tools::{PermissionKind, ToolRegistry};
use anyhow::{Context, Result};
use std::path::Path;
use std::sync::Arc;

/// Scans a directory for plugin TOML files and registers them in a ToolRegistry.
pub struct PluginLoader;

impl PluginLoader {
    /// Load all plugins from `.toml` files in the given directory.
    /// Returns (count_loaded, errors) for transparency.
    pub fn load_from_dir(dir: &Path, registry: &mut ToolRegistry) -> (usize, Vec<String>) {
        let mut loaded = 0;
        let mut errors = Vec::new();

        let dir = match std::fs::read_dir(dir) {
            Ok(d) => d,
            Err(e) => {
                errors.push(format!("cannot read plugin dir '{}': {e}", dir.display()));
                return (0, errors);
            }
        };

        for entry in dir.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("toml") {
                continue;
            }
            match Self::load_one(&path) {
                Ok(tool) => {
                    registry.register(Arc::new(tool));
                    loaded += 1;
                }
                Err(e) => {
                    errors.push(format!("{}: {e}", path.display()));
                }
            }
        }

        (loaded, errors)
    }

    fn load_one(path: &Path) -> Result<ExternalTool> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("read plugin '{}'", path.display()))?;
        let desc: PluginDescriptor = toml::from_str(&content)
            .with_context(|| format!("parse plugin '{}'", path.display()))?;

        let def = desc.plugin;
        let permission = match def.requires_permission.as_deref() {
            Some("bash") | Some("shell") => PermissionKind::Bash,
            Some("filesystem") | Some("fs") => PermissionKind::Filesystem,
            Some("network") | Some("http") => PermissionKind::Network,
            Some("interactive") => PermissionKind::Interactive,
            _ => PermissionKind::None,
        };

        let parameters = if def.parameters.is_null() || def.parameters.as_object().map_or(false, |o| o.is_empty()) {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "input": {
                        "type": "string",
                        "description": "Input passed to the external tool"
                    }
                },
                "required": ["input"]
            })
        } else {
            def.parameters
        };

        Ok(ExternalTool::new(
            &def.name,
            &def.description,
            parameters,
            &def.command,
            def.args,
            Some(permission),
        ))
    }

    /// Load MCP-based plugins from a directory and spawn them.
    /// Returns (loaded_tool_count, errors).
    pub async fn load_mcp_from_dir(
        dir: &Path,
        mcp_registry: &McpRegistry,
    ) -> (usize, Vec<String>) {
        let mut loaded = 0;
        let mut errors = Vec::new();

        let dir = match std::fs::read_dir(dir) {
            Ok(d) => d,
            Err(e) => {
                errors.push(format!("cannot read plugin dir '{}': {e}", dir.display()));
                return (0, errors);
            }
        };

        for entry in dir.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("toml") {
                continue;
            }
            let content = match std::fs::read_to_string(&path) {
                Ok(c) => c,
                Err(e) => {
                    errors.push(format!("{}: {e}", path.display()));
                    continue;
                }
            };
            let desc: PluginDescriptor = match toml::from_str(&content) {
                Ok(d) => d,
                Err(e) => {
                    errors.push(format!("{}: {e}", path.display()));
                    continue;
                }
            };

            let def = desc.plugin;
            let Some(mcp) = def.mcp_server else {
                // Skip non-MCP plugins — they're handled by load_from_dir
                continue;
            };

            let env_vars: Vec<(&str, &str)> = mcp.env.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
            match mcp_registry
                .spawn(&def.name, &mcp.command, &mcp.args, &env_vars)
                .await
            {
                Ok(tools) => {
                    loaded += tools.len();
                }
                Err(e) => {
                    errors.push(format!("{}: mcp spawn error: {e}", path.display()));
                }
            }
        }

        (loaded, errors)
    }

    /// Load a single plugin from a TOML string (useful for testing).
    pub fn load_from_str(toml_str: &str) -> Result<ExternalTool> {
        let desc: PluginDescriptor = toml::from_str(toml_str)?;
        let def = desc.plugin;
        let permission = match def.requires_permission.as_deref() {
            Some("bash") | Some("shell") => PermissionKind::Bash,
            Some("filesystem") | Some("fs") => PermissionKind::Filesystem,
            Some("network") | Some("http") => PermissionKind::Network,
            Some("interactive") => PermissionKind::Interactive,
            _ => PermissionKind::None,
        };
        let parameters = if def.parameters.is_null() || def.parameters.as_object().map_or(false, |o| o.is_empty()) {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "input": {
                        "type": "string",
                        "description": "Input passed to the external tool"
                    }
                },
                "required": ["input"]
            })
        } else {
            def.parameters
        };
        Ok(ExternalTool::new(
            &def.name,
            &def.description,
            parameters,
            &def.command,
            def.args,
            Some(permission),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::Tool;

    #[test]
    fn parse_minimal_plugin() {
        let toml = r#"
[plugin]
name = "echo-tool"
description = "Echoes input back"
command = "/bin/echo"
args = []
"#;
        let tool = PluginLoader::load_from_str(toml).unwrap();
        assert_eq!(tool.name(), "echo-tool");
        assert_eq!(tool.command, "/bin/echo");
        assert!(tool.parameters().get("properties").is_some());
    }

    #[test]
    fn parse_plugin_with_permission() {
        let toml = r#"
[plugin]
name = "dangerous"
description = "Runs a shell script"
command = "/usr/local/bin/script.sh"
requires_permission = "bash"
"#;
        let tool = PluginLoader::load_from_str(toml).unwrap();
        assert_eq!(tool.name(), "dangerous");
        assert_eq!(tool.requires_permission(), PermissionKind::Bash);
    }

    #[test]
    fn parse_plugin_with_custom_params() {
        let toml = r#"
[plugin]
name = "weather"
description = "Get weather"
command = "curl"
args = ["-s", "wttr.in"]

[plugin.parameters]
type = "object"

[plugin.parameters.properties.city]
type = "string"
description = "City name"

required = ["city"]
"#;
        let tool = PluginLoader::load_from_str(toml).unwrap();
        assert_eq!(tool.name(), "weather");
        assert_eq!(tool.args, vec!["-s", "wttr.in"]);
    }

    #[test]
    fn load_from_dir_handles_missing_dir() {
        let mut reg = ToolRegistry::new();
        let (count, errors) = PluginLoader::load_from_dir(
            Path::new("/tmp/orion-nonexistent-plugin-dir-12345"),
            &mut reg,
        );
        assert_eq!(count, 0);
        assert!(!errors.is_empty());
    }
}
