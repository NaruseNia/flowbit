use anyhow::{Context, Result, bail};
use octocrab::Octocrab;
use serde::de::DeserializeOwned;
use tracing::{debug, info, warn};

use super::queries;
use super::types::{ItemsResponse, MetadataResponse, ProjectListEntry, ProjectListResponse};
use crate::model::project_item::{ProjectItem, StatusColumn};

pub struct GithubClient {
    octocrab: Octocrab,
    owner: String,
    project_number: u32,
    status_field_name: String,
}

/// Result of fetching project data: metadata + items.
pub struct ProjectData {
    pub project_id: String,
    pub project_title: String,
    pub status_columns: Vec<StatusColumn>,
    pub items: Vec<ProjectItem>,
}

impl GithubClient {
    pub fn new(token: &str, api_base_url: &str, owner: &str, project_number: u32, status_field_name: &str) -> Result<Self> {
        let mut builder = Octocrab::builder().personal_token(token.to_string());

        if api_base_url != "https://api.github.com" {
            builder = builder.base_uri(api_base_url)?;
        }

        let octocrab = builder.build().context("Failed to build GitHub client")?;

        Ok(Self {
            octocrab,
            owner: owner.to_string(),
            project_number,
            status_field_name: status_field_name.to_string(),
        })
    }

    /// Set project number (used after interactive selection).
    pub fn set_project_number(&mut self, number: u32) {
        self.project_number = number;
    }

    // --- GraphQL helper ---

    async fn graphql<T: DeserializeOwned>(&self, query: &str, variables: &serde_json::Value) -> Result<T> {
        let response: serde_json::Value = self
            .octocrab
            .graphql(&serde_json::json!({
                "query": query,
                "variables": variables,
            }))
            .await
            .context("GraphQL request failed")?;

        if let Some(errors) = response.get("errors") {
            bail!("GraphQL errors: {}", errors);
        }

        serde_json::from_value(response).context("Failed to parse GraphQL response")
    }

    // --- Project list ---

    /// Fetch the list of projects for the configured owner.
    /// Tries user first, then organization.
    pub async fn list_projects(&self) -> Result<Vec<ProjectListEntry>> {
        let variables = serde_json::json!({ "owner": self.owner });

        // Try as user
        if let Ok(resp) = self.graphql::<ProjectListResponse>(queries::LIST_PROJECTS_USER, &variables).await {
            if let Some(owner) = resp.data.user {
                return Ok(owner.projects_v2.nodes);
            }
        }

        // Fallback to organization
        debug!("User query failed, trying organization");
        let resp = self.graphql::<ProjectListResponse>(queries::LIST_PROJECTS_ORG, &variables)
            .await
            .context("Failed to list projects (tried both user and organization)")?;

        let owner = resp.data.user.context("Owner not found")?;
        Ok(owner.projects_v2.nodes)
    }

    // --- Project data ---

    /// Fetch all project data: metadata + items.
    pub async fn fetch_project(&self) -> Result<ProjectData> {
        let (project_id, project_title, status_columns) = self.fetch_metadata().await?;
        let items = self.fetch_all_items(&project_id).await?;
        Ok(ProjectData {
            project_id,
            project_title,
            status_columns,
            items,
        })
    }

    /// Fetch project metadata (title, status field options).
    /// Tries user query first, then falls back to organization query.
    async fn fetch_metadata(&self) -> Result<(String, String, Vec<StatusColumn>)> {
        info!(owner = %self.owner, number = self.project_number, "Fetching project metadata");

        let variables = serde_json::json!({
            "owner": self.owner,
            "number": self.project_number as i64,
        });

        // Try as user first
        if let Ok(data) = self.graphql::<MetadataResponse>(queries::PROJECT_METADATA, &variables).await {
            if let Ok(result) = self.parse_metadata(data) {
                return Ok(result);
            }
        }

        // Fallback to organization
        debug!("User query failed, trying organization query");
        let data = self.graphql::<MetadataResponse>(queries::PROJECT_METADATA_ORG, &variables)
            .await
            .context("Failed to fetch project metadata (tried both user and organization)")?;
        self.parse_metadata(data)
    }

    fn parse_metadata(&self, response: MetadataResponse) -> Result<(String, String, Vec<StatusColumn>)> {
        let owner = response
            .data
            .user
            .context("Project owner not found")?;
        let project = owner
            .project_v2
            .context("Project not found. Check owner and project number.")?;

        let (field_id, columns) = project
            .status_columns(&self.status_field_name)
            .with_context(|| {
                format!(
                    "Status field '{}' not found in project. Available single-select fields: {}",
                    self.status_field_name,
                    project.fields.nodes.iter()
                        .filter_map(|f| f.name.as_deref())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })?;

        info!(
            project_title = %project.title,
            status_field_id = %field_id,
            column_count = columns.len(),
            "Project metadata loaded"
        );

        Ok((project.id.clone(), project.title.clone(), columns))
    }

    /// Fetch all project items with pagination.
    async fn fetch_all_items(&self, project_id: &str) -> Result<Vec<ProjectItem>> {
        let mut all_items = Vec::new();
        let mut cursor: Option<String> = None;
        let mut page = 0u32;

        loop {
            page += 1;
            info!(page, "Fetching project items");

            let variables = serde_json::json!({
                "projectId": project_id,
                "cursor": cursor,
            });

            let parsed: ItemsResponse = self.graphql(queries::PROJECT_ITEMS, &variables).await
                .context("Failed to fetch project items")?;

            let project = parsed.data.node.context("Project node not found in items response")?;
            let connection = project.items;

            let mut page_count = 0u32;
            let mut skipped = 0u32;
            for node in &connection.nodes {
                if let Some(item_node) = node {
                    if let Some(project_item) = item_node.to_project_item(&self.status_field_name) {
                        all_items.push(project_item);
                        page_count += 1;
                    } else {
                        skipped += 1;
                    }
                }
            }

            debug!(page, fetched = page_count, skipped, "Page processed");

            if !connection.page_info.has_next_page {
                break;
            }
            cursor = connection.page_info.end_cursor;
        }

        // Sort by updated_at desc, then number asc as tie-breaker
        all_items.sort_by(|a, b| {
            b.updated_at
                .cmp(&a.updated_at)
                .then(a.number.cmp(&b.number))
        });

        if all_items.len() > 500 {
            warn!(
                count = all_items.len(),
                "Large project detected (>500 items). Performance may be affected."
            );
        }

        info!(total = all_items.len(), "All items fetched");
        Ok(all_items)
    }
}
