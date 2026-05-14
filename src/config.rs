use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub const DUMBCODER_DIR: &str = ".dumbcoder";
pub const CONFIG_FILE: &str = "config.toml";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub model: ModelConfig,
    #[serde(default)]
    pub index: IndexConfig,
    #[serde(default)]
    pub security: SecurityConfig,
    #[serde(default)]
    pub commands: CommandsConfig,
    #[serde(default)]
    pub prompts: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    #[serde(default = "default_provider")]
    pub provider: String,
    #[serde(default = "default_base_url")]
    pub base_url: String,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub timeout_seconds: Option<u64>,
    #[serde(default = "default_context_limit")]
    pub context_limit: usize,
    #[serde(default)]
    pub providers: Vec<ProviderEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderEntry {
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_db_path")]
    pub db_path: String,
    #[serde(default = "default_ignore")]
    pub ignore: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    #[serde(default = "default_false")]
    pub allow_write: bool,
    #[serde(default = "default_false")]
    pub allow_network: bool,
    #[serde(default = "default_max_command_seconds")]
    pub max_command_seconds: u64,
    #[serde(default = "default_max_output_bytes")]
    pub max_output_bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandsConfig {
    #[serde(default = "default_allowed_commands")]
    pub allow: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            model: ModelConfig::default(),
            index: IndexConfig::default(),
            security: SecurityConfig::default(),
            commands: CommandsConfig::default(),
            prompts: HashMap::new(),
        }
    }
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            provider: default_provider(),
            base_url: default_base_url(),
            model: default_model(),
            api_key: None,
            timeout_seconds: None,
            context_limit: default_context_limit(),
            providers: Vec::new(),
        }
    }
}

impl ModelConfig {
    pub fn validate(&self) -> Result<()> {
        match self.provider.as_str() {
            "ollama" => Ok(()),
            "openai" => {
                if self.api_key.is_none() {
                    anyhow::bail!("provider 'openai' requires api_key in [model] config");
                }
                Ok(())
            }
            "openai_compatible" => Ok(()),
            other => anyhow::bail!(
                "unknown provider '{}'. Supported: ollama, openai, openai_compatible",
                other
            ),
        }
    }
}

impl Default for IndexConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            db_path: default_db_path(),
            ignore: default_ignore(),
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            allow_write: default_false(),
            allow_network: default_false(),
            max_command_seconds: default_max_command_seconds(),
            max_output_bytes: default_max_output_bytes(),
        }
    }
}

impl Default for CommandsConfig {
    fn default() -> Self {
        Self {
            allow: default_allowed_commands(),
        }
    }
}

fn default_provider() -> String {
    "ollama".into()
}
fn default_base_url() -> String {
    "http://127.0.0.1:11434".into()
}
fn default_model() -> String {
    "qwen2.5-coder:7b".into()
}
fn default_context_limit() -> usize {
    8000
}
fn default_true() -> bool {
    true
}
fn default_false() -> bool {
    false
}
fn default_db_path() -> String {
    ".dumbcoder/index".into()
}
fn default_max_command_seconds() -> u64 {
    60
}
fn default_max_output_bytes() -> usize {
    20000
}
fn default_ignore() -> Vec<String> {
    vec![
        ".git".into(),
        "target".into(),
        "node_modules".into(),
        "dist".into(),
        "build".into(),
    ]
}
fn default_allowed_commands() -> Vec<String> {
    vec![
        "rg".into(),
        "git status".into(),
        "git diff".into(),
        "git log".into(),
        "git show".into(),
    ]
}

/// Returns the global config directory: ~/.config/dumbcoder/
pub fn global_config_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".config").join("dumbcoder"))
}

/// Returns the global config file path: ~/.config/dumbcoder/config.toml
fn global_config_path() -> Option<PathBuf> {
    global_config_dir().map(|d| d.join(CONFIG_FILE))
}

impl Config {
    /// Load config with global → project merge strategy.
    ///
    /// 1. Load global config from ~/.config/dumbcoder/config.toml (if exists)
    /// 2. Load project config from <root>/.dumbcoder/config.toml (if exists)
    /// 3. Merge: project values override global values for explicitly set fields
    pub fn load(project_root: &Path) -> Result<Self> {
        let global_path = global_config_path();
        let project_path = project_root.join(DUMBCODER_DIR).join(CONFIG_FILE);

        let global_val = global_path
            .as_ref()
            .filter(|p| p.exists())
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| toml::from_str::<toml::Value>(&s).ok());

        let project_val = if project_path.exists() {
            let content = std::fs::read_to_string(&project_path)
                .context("Failed to read project config file")?;
            Some(
                toml::from_str::<toml::Value>(&content)
                    .context("Failed to parse project config file")?,
            )
        } else {
            None
        };

        // Merge: project overrides global
        let merged = match (global_val, project_val) {
            (Some(mut global), Some(project)) => {
                merge_toml(&mut global, &project);
                global
            }
            (Some(global), None) => global,
            (None, Some(project)) => project,
            (None, None) => return Ok(Config::default()),
        };

        let config: Config = merged
            .try_into()
            .context("Failed to parse merged config")?;
        Ok(config)
    }

    /// Save config to the given directory.
    pub fn save(&self, project_root: &Path) -> Result<()> {
        let dir = project_root.join(DUMBCODER_DIR);
        std::fs::create_dir_all(&dir).context("Failed to create .dumbcoder directory")?;
        let config_path = dir.join(CONFIG_FILE);
        let content = toml::to_string_pretty(self).context("Failed to serialize config")?;
        std::fs::write(&config_path, content).context("Failed to write config file")?;
        Ok(())
    }

    /// Find project root by looking for .dumbcoder directory or git repo.
    /// Returns (root, is_project_root) — if false, no .dumbcoder/ found and
    /// caller should rely on global config.
    pub fn find_project_root() -> Result<(PathBuf, bool)> {
        let cwd = std::env::current_dir()?;
        let mut dir = cwd.clone();
        loop {
            if dir.join(DUMBCODER_DIR).is_dir() {
                return Ok((dir, true));
            }
            if dir.join(".git").is_dir() {
                return Ok((dir, true));
            }
            if !dir.pop() {
                // No .dumbcoder or .git found — use cwd with global config
                return Ok((cwd, false));
            }
        }
    }
}

/// Recursively merge `other` into `base`. For tables, merge keys.
/// For non-table values, `other` overrides `base`.
fn merge_toml(base: &mut toml::Value, other: &toml::Value) {
    match (base, other) {
        (toml::Value::Table(base_map), toml::Value::Table(other_map)) => {
            for (key, other_val) in other_map {
                if let Some(base_val) = base_map.get_mut(key) {
                    merge_toml(base_val, other_val);
                } else {
                    base_map.insert(key.clone(), other_val.clone());
                }
            }
        }
        (base, other) => {
            *base = other.clone();
        }
    }
}
