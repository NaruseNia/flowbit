use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub github: GithubConfig,
    pub project: ProjectConfig,
    #[serde(default)]
    pub ui: UiConfig,
    #[serde(default)]
    pub filter: FilterConfig,
}

#[derive(Debug, Deserialize)]
pub struct GithubConfig {
    pub token: Option<String>,
    #[serde(default = "default_api_base_url")]
    pub api_base_url: String,
}

#[derive(Debug, Deserialize)]
pub struct ProjectConfig {
    pub owner: String,
    pub number: u32,
    #[serde(default = "default_status_field")]
    pub status_field: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct UiConfig {
    pub default_view: Option<String>,
    pub open_command: Option<String>,
    pub date_format: Option<String>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct FilterConfig {
    pub assignee: Option<String>,
    pub labels: Option<Vec<String>>,
    pub kind: Option<String>,
    pub status: Option<String>,
}

fn default_api_base_url() -> String {
    "https://api.github.com".into()
}

fn default_status_field() -> String {
    "Status".into()
}

impl Config {
    /// Resolve GitHub token: GITHUB_TOKEN env var takes priority over config file.
    pub fn github_token(&self) -> Result<String> {
        if let Ok(token) = std::env::var("GITHUB_TOKEN") {
            if !token.is_empty() {
                return Ok(token);
            }
        }
        if let Some(token) = &self.github.token {
            if !token.is_empty() {
                return Ok(token.clone());
            }
        }
        bail!(
            "GitHub token not found. Set GITHUB_TOKEN env var or add token to config file."
        )
    }

    /// Load config from the default path (~/.config/flowbit/config.toml).
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        if !path.exists() {
            bail!(
                "Config file not found: {}\nCreate it with your GitHub token and project settings.",
                path.display()
            );
        }
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;
        let config: Config = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;
        Ok(config)
    }

    /// Return the config file path: ~/.config/flowbit/config.toml
    pub fn config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .context("Could not determine config directory")?;
        Ok(config_dir.join("flowbit").join("config.toml"))
    }
}
