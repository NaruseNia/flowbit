mod api;
mod cache;
mod config;
mod logging;
mod model;

use std::io::{self, Write};

use anyhow::{Result, bail};
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

    // Resolve project number (interactive selection if not configured)
    let project_number = match config.project.number {
        Some(n) => n,
        None => select_project(&token, &config).await?,
    };

    info!(owner = %config.project.owner, number = project_number, "Config loaded");

    // Initialize cache
    let cache_store = CacheStore::new()?;

    // Build API client
    let mut client = GithubClient::new(
        &token,
        &config.github.api_base_url,
        &config.project.owner,
        0, // placeholder
        &config.project.status_field,
    )?;
    client.set_project_number(project_number);

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

/// Interactive project selection when `number` is not set in config.
async fn select_project(token: &str, config: &Config) -> Result<u32> {
    // Create a temporary client with dummy project number
    let client = GithubClient::new(
        token,
        &config.github.api_base_url,
        &config.project.owner,
        0,
        &config.project.status_field,
    )?;

    eprintln!("Fetching projects for {}...", config.project.owner);
    let projects = client.list_projects().await?;

    let open_projects: Vec<_> = projects.iter().filter(|p| !p.closed).collect();
    if open_projects.is_empty() {
        bail!("No open projects found for '{}'", config.project.owner);
    }

    eprintln!();
    eprintln!("Select a project:");
    eprintln!();
    for (i, p) in open_projects.iter().enumerate() {
        eprintln!(
            "  [{}] #{} — {} ({} items)",
            i + 1,
            p.number,
            p.title,
            p.items.total_count,
        );
    }
    eprintln!();

    loop {
        eprint!("Enter number [1-{}]: ", open_projects.len());
        io::stderr().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if let Ok(idx) = input.parse::<usize>() {
            if idx >= 1 && idx <= open_projects.len() {
                let selected = open_projects[idx - 1];
                eprintln!();
                eprintln!("Selected: #{} — {}", selected.number, selected.title);

                // Save to config for next time
                if let Err(e) = save_project_number(selected.number) {
                    eprintln!("Note: Could not save selection to config: {}", e);
                } else {
                    eprintln!("Saved to config. Next run will use this project automatically.");
                }

                return Ok(selected.number);
            }
        }
        eprintln!("Invalid input. Try again.");
    }
}

/// Write `number = N` into the [project] section of the config file.
fn save_project_number(number: u32) -> Result<()> {
    let path = Config::config_path()?;
    let content = std::fs::read_to_string(&path)?;

    // Insert `number = N` after the `owner = ...` line in [project] section
    let mut lines: Vec<String> = content.lines().map(String::from).collect();
    let mut inserted = false;

    for i in 0..lines.len() {
        if lines[i].starts_with("owner") && lines[i].contains('=') {
            // Check if next line is already a number line
            if i + 1 < lines.len() && lines[i + 1].trim().starts_with("number") {
                lines[i + 1] = format!("number = {}", number);
            } else {
                lines.insert(i + 1, format!("number = {}", number));
            }
            inserted = true;
            break;
        }
    }

    if !inserted {
        bail!("Could not find [project] owner line in config");
    }

    std::fs::write(&path, lines.join("\n") + "\n")?;
    Ok(())
}
