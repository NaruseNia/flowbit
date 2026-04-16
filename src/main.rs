mod api;
mod cache;
mod config;
mod logging;
mod model;

use anyhow::Result;
use chrono::Utc;
use tracing::{error, info};

use api::client::GithubClient;
use cache::{CacheStore, CachedSnapshot};
use config::Config;
use model::filter::Filter;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging (file-based)
    let _log_guard = logging::init()?;
    info!("flowbit starting");

    // Load config
    let config = Config::load()?;
    let token = config.github_token()?;
    info!(owner = %config.project.owner, number = config.project.number, "Config loaded");

    // Initialize cache
    let cache_store = CacheStore::new()?;

    // Build API client
    let client = GithubClient::new(
        &token,
        &config.github.api_base_url,
        &config.project.owner,
        config.project.number,
        &config.project.status_field,
    )?;

    // Fetch project data
    let data = match client.fetch_project().await {
        Ok(data) => {
            info!(title = %data.project_title, items = data.items.len(), "Project data fetched");

            // Save to cache
            let snapshot = CachedSnapshot {
                fetched_at: Utc::now(),
                project_title: data.project_title.clone(),
                status_columns: data.status_columns.clone(),
                items: data.items.clone(),
            };
            if let Err(e) = cache_store.save(&snapshot) {
                error!(error = %e, "Failed to save cache");
            }

            data
        }
        Err(e) => {
            error!(error = %e, "Failed to fetch project data, trying cache");

            // Try loading from cache
            if let Some(snapshot) = cache_store.load() {
                eprintln!(
                    "[STALE] Using cached data from {}",
                    snapshot.fetched_at.format("%Y-%m-%d %H:%M:%S UTC")
                );
                api::client::ProjectData {
                    project_id: String::new(),
                    project_title: snapshot.project_title,
                    status_columns: snapshot.status_columns,
                    items: snapshot.items,
                }
            } else {
                return Err(e.context("No cached data available"));
            }
        }
    };

    // Apply default filter if configured
    let default_filter = Filter::from(&config.filter);
    let items: Vec<_> = if default_filter.is_empty() {
        data.items
    } else {
        let summary = default_filter.display_summary();
        eprintln!("[filter] {}", summary);
        data.items
            .into_iter()
            .filter(|item| default_filter.matches(item))
            .collect()
    };

    // Output as JSON (Phase 1 CLI output — replaced by TUI in Phase 2)
    println!("Project: {}", data.project_title);
    println!("Status columns: {:?}", data.status_columns.iter().map(|c| &c.name).collect::<Vec<_>>());
    println!("Items: {}", items.len());
    println!();

    let json = serde_json::to_string_pretty(&items)?;
    println!("{}", json);

    info!("flowbit finished");
    Ok(())
}
