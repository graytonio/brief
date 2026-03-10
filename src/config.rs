/// Config file parsing and writing for ~/.brief/config.toml.
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Top-level configuration stored in ~/.brief/config.toml.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// Global standards entry — injected into every session.
    #[serde(default)]
    pub global: GlobalConfig,

    /// Per-language entries keyed by language name.
    #[serde(default)]
    pub languages: HashMap<String, LanguageConfig>,
}

/// Global section of the config.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GlobalConfig {
    /// Remote URL returning plain-text CLAUDE.md content.
    pub url: Option<String>,

    /// How long (seconds) to use a cached copy before re-fetching. Default: 3600.
    #[serde(default = "default_cache_ttl")]
    pub cache_ttl: u64,

    /// URL of the team config TOML used by `brief sync`.
    pub team_config_url: Option<String>,
}

fn default_cache_ttl() -> u64 {
    3600
}

/// Per-language entry in the config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageConfig {
    /// Remote URL returning plain-text CLAUDE.md content.
    pub url: String,

    /// Filenames used to detect this language when walking up the directory tree.
    #[serde(default)]
    pub detect: Vec<String>,
}

impl Config {
    /// Load the config from disk, returning a default if the file doesn't exist.
    pub fn load() -> Result<Self> {
        let path = config_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let contents = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;
        let config: Config = toml::from_str(&contents)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;
        Ok(config)
    }

    /// Persist the config to disk, creating parent directories as needed.
    pub fn save(&self) -> Result<()> {
        let path = config_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create config directory: {}", parent.display())
            })?;
        }
        let contents = toml::to_string_pretty(self).context("Failed to serialise config")?;
        fs::write(&path, contents)
            .with_context(|| format!("Failed to write config file: {}", path.display()))?;
        Ok(())
    }
}

/// Returns the path to ~/.brief/config.toml.
pub fn config_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    Ok(home.join(".brief").join("config.toml"))
}

/// Returns the path to the ~/.brief/ directory.
pub fn brief_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    Ok(home.join(".brief"))
}

/// Returns the path to ~/.brief/cache/.
pub fn cache_dir() -> Result<PathBuf> {
    Ok(brief_dir()?.join("cache"))
}

/// Returns the path to the optional token file ~/.brief/.token.
pub fn token_path() -> Result<PathBuf> {
    Ok(brief_dir()?.join(".token"))
}

/// Default detection filenames for well-known languages.
pub fn default_detect_files(language: &str) -> Vec<String> {
    match language {
        "rust" => vec!["Cargo.toml".into()],
        "kotlin" => vec!["build.gradle.kts".into(), "build.gradle".into()],
        "python" => vec!["pyproject.toml".into(), "setup.py".into()],
        "javascript" | "typescript" => vec!["package.json".into()],
        "go" => vec!["go.mod".into()],
        _ => vec![],
    }
}
