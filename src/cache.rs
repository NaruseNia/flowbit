use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{debug, info, warn};

use crate::model::project_item::{ProjectItem, StatusColumn};

#[derive(Debug, Serialize, Deserialize)]
pub struct CachedSnapshot {
    pub fetched_at: DateTime<Utc>,
    pub project_title: String,
    pub status_columns: Vec<StatusColumn>,
    pub items: Vec<ProjectItem>,
}

pub struct CacheStore {
    path: PathBuf,
}

impl CacheStore {
    pub fn new() -> Result<Self> {
        let cache_dir =
            dirs::cache_dir().context("Could not determine cache directory")?;
        let path = cache_dir.join("flowbit").join("cache.json");
        Ok(Self { path })
    }

    /// Save a snapshot to the cache file.
    pub fn save(&self, snapshot: &CachedSnapshot) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create cache directory: {}", parent.display()))?;
        }

        let json = serde_json::to_string_pretty(snapshot)
            .context("Failed to serialize cache")?;
        std::fs::write(&self.path, json)
            .with_context(|| format!("Failed to write cache file: {}", self.path.display()))?;

        info!(path = %self.path.display(), "Cache saved");
        Ok(())
    }

    /// Load a snapshot from the cache file, if it exists.
    pub fn load(&self) -> Option<CachedSnapshot> {
        if !self.path.exists() {
            debug!("No cache file found");
            return None;
        }

        match std::fs::read_to_string(&self.path) {
            Ok(content) => match serde_json::from_str::<CachedSnapshot>(&content) {
                Ok(snapshot) => {
                    info!(
                        fetched_at = %snapshot.fetched_at,
                        items = snapshot.items.len(),
                        "Cache loaded"
                    );
                    Some(snapshot)
                }
                Err(e) => {
                    warn!(error = %e, "Failed to parse cache file, ignoring");
                    None
                }
            },
            Err(e) => {
                warn!(error = %e, "Failed to read cache file, ignoring");
                None
            }
        }
    }
}
