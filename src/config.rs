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

impl Config {
    /// Load config from the given directory, or use current directory.
    pub fn load(project_root: &Path) -> Result<Self> {
        let config_path = project_root.join(DUMBCODER_DIR).join(CONFIG_FILE);
        if config_path.exists() {
            let content =
                std::fs::read_to_string(&config_path).context("Failed to read config file")?;
            let config: Config =
                toml::from_str(&content).context("Failed to parse config file")?;
            Ok(config)
        } else {
            Ok(Config::default())
        }
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
    pub fn find_project_root() -> Result<PathBuf> {
        let mut dir = std::env::current_dir()?;
        loop {
            if dir.join(DUMBCODER_DIR).is_dir() || dir.join(".git").is_dir() {
                return Ok(dir);
            }
            if !dir.pop() {
                return std::env::current_dir().context("Cannot determine project root");
            }
        }
    }
}
