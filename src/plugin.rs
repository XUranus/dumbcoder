use serde::Deserialize;
use std::path::Path;

use crate::config::{Config, DUMBCODER_DIR};

#[derive(Debug, Deserialize)]
pub struct Plugin {
    pub name: String,
    pub description: String,
    pub system_prompt: String,
}

/// Load all plugins from .dumbcoder/plugins/*.toml
pub fn load_plugins(root: &Path) -> Vec<Plugin> {
    let plugins_dir = root.join(DUMBCODER_DIR).join("plugins");
    if !plugins_dir.is_dir() {
        return Vec::new();
    }

    let mut plugins = Vec::new();
    let entries = match std::fs::read_dir(&plugins_dir) {
        Ok(e) => e,
        Err(_) => return plugins,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map_or(true, |e| e != "toml") {
            continue;
        }

        match std::fs::read_to_string(&path) {
            Ok(content) => match toml::from_str::<Plugin>(&content) {
                Ok(plugin) => plugins.push(plugin),
                Err(e) => {
                    eprintln!("  Warning: failed to parse plugin {}: {e}", path.display());
                }
            },
            Err(e) => {
                eprintln!("  Warning: failed to read plugin {}: {e}", path.display());
            }
        }
    }

    plugins
}

/// Resolve system prompt for a built-in command.
/// Checks config.prompts first, falls back to default.
pub fn resolve_prompt(config: &Config, command: &str, default: &str) -> String {
    config
        .prompts
        .get(command)
        .cloned()
        .unwrap_or_else(|| default.to_string())
}
