use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::path::PathBuf;
use std::process::Command;
use tracing::{debug, info};

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
    pub number: Option<u32>,
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

const CONFIG_TEMPLATE: &str = r#"[github]
# Token resolution order:
#   1. GITHUB_TOKEN environment variable
#   2. `gh auth token` (GitHub CLI)
#   3. This field
# token = "ghp_xxxx"

[project]
owner = "your-org-or-user"
number = 1
status_field = "Status"

# [ui]
# default_view = "board"    # "board" or "list"
# open_command = ""          # custom browser command (empty = OS default)
# date_format = "%Y-%m-%d"

# [filter]
# Default filters applied on startup (Esc to clear)
# assignee = "your-username"
# labels = ["bug"]
# kind = "issue"             # "issue" or "pr"
# status = "In Progress"
"#;

impl Config {
    /// Resolve GitHub token.
    /// Priority: GITHUB_TOKEN env → `gh auth token` → config file.
    pub fn github_token(&self) -> Result<String> {
        // 1. Environment variable
        if let Ok(token) = std::env::var("GITHUB_TOKEN") {
            if !token.is_empty() {
                debug!("Token resolved from GITHUB_TOKEN env var");
                return Ok(token);
            }
        }

        // 2. GitHub CLI
        if let Some(token) = Self::token_from_gh_cli() {
            debug!("Token resolved from gh auth token");
            return Ok(token);
        }

        // 3. Config file
        if let Some(token) = &self.github.token {
            if !token.is_empty() {
                debug!("Token resolved from config file");
                return Ok(token.clone());
            }
        }

        bail!(
            "GitHub token not found.\n\
             Set one of the following:\n\
             - GITHUB_TOKEN environment variable\n\
             - Run `gh auth login` (GitHub CLI)\n\
             - Add `token = \"ghp_xxxx\"` to [github] section in config file"
        )
    }

    /// Try to get token from `gh auth token`.
    fn token_from_gh_cli() -> Option<String> {
        let output = Command::new("gh")
            .args(["auth", "token"])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let token = String::from_utf8(output.stdout).ok()?;
        let token = token.trim().to_string();
        if token.is_empty() {
            None
        } else {
            Some(token)
        }
    }

    /// Load config from the default path (~/.config/flowbit/config.toml).
    /// If the file doesn't exist, creates a template and instructs the user.
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        if !path.exists() {
            Self::create_template(&path)?;
            bail!(
                "Config file created: {}\n\
                 Edit it with your project settings, then run flowbit again.",
                path.display()
            );
        }
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;
        let config: Config = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

        info!(path = %path.display(), "Config loaded");
        Ok(config)
    }

    /// Return the config file path: ~/.config/flowbit/config.toml
    /// Uses $XDG_CONFIG_HOME if set, otherwise ~/.config (not platform-specific).
    pub fn config_path() -> Result<PathBuf> {
        let config_dir = if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
            PathBuf::from(xdg)
        } else {
            dirs::home_dir()
                .context("Could not determine home directory")?
                .join(".config")
        };
        Ok(config_dir.join("flowbit").join("config.toml"))
    }

    /// Create a template config file at the given path.
    fn create_template(path: &PathBuf) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!(
                    "Failed to create config directory: {}",
                    parent.display()
                )
            })?;
        }
        std::fs::write(path, CONFIG_TEMPLATE).with_context(|| {
            format!("Failed to write config template: {}", path.display())
        })?;
        Ok(())
    }
}
