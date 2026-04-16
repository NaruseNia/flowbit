use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::model::project_item::{ItemKind, ProjectItem, StatusColumn};

// --- Project List Response ---

#[derive(Debug, Deserialize)]
pub struct ProjectListResponse {
    pub data: ProjectListData,
}

#[derive(Debug, Deserialize)]
pub struct ProjectListData {
    #[serde(alias = "organization")]
    pub user: Option<ProjectListOwner>,
}

#[derive(Debug, Deserialize)]
pub struct ProjectListOwner {
    #[serde(rename = "projectsV2")]
    pub projects_v2: ProjectListConnection,
}

#[derive(Debug, Deserialize)]
pub struct ProjectListConnection {
    pub nodes: Vec<ProjectListEntry>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProjectListEntry {
    pub number: u32,
    pub title: String,
    #[serde(rename = "shortDescription", default)]
    pub short_description: Option<String>,
    pub closed: bool,
    pub items: ItemCount,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ItemCount {
    #[serde(rename = "totalCount")]
    pub total_count: u32,
}

// --- Project Metadata Response ---

#[derive(Debug, Deserialize)]
pub struct MetadataResponse {
    pub data: MetadataData,
}

#[derive(Debug, Deserialize)]
pub struct MetadataData {
    #[serde(alias = "organization")]
    pub user: Option<MetadataOwner>,
}

#[derive(Debug, Deserialize)]
pub struct MetadataOwner {
    #[serde(rename = "projectV2")]
    pub project_v2: Option<MetadataProject>,
}

#[derive(Debug, Deserialize)]
pub struct MetadataProject {
    pub id: String,
    pub title: String,
    pub fields: FieldNodes,
}

#[derive(Debug, Deserialize)]
pub struct FieldNodes {
    pub nodes: Vec<FieldNode>,
}

/// Fields in the project. Non-single-select fields appear as empty objects `{}`
/// from the GraphQL fragment, so all fields are optional.
#[derive(Debug, Deserialize)]
pub struct FieldNode {
    pub id: Option<String>,
    pub name: Option<String>,
    #[serde(default)]
    pub options: Vec<FieldOption>,
}

#[derive(Debug, Deserialize)]
pub struct FieldOption {
    pub id: String,
    pub name: String,
}

// --- Project Items Response ---

#[derive(Debug, Deserialize)]
pub struct ItemsResponse {
    pub data: ItemsData,
}

#[derive(Debug, Deserialize)]
pub struct ItemsData {
    pub node: Option<ItemsProject>,
}

#[derive(Debug, Deserialize)]
pub struct ItemsProject {
    pub items: ItemsConnection,
}

#[derive(Debug, Deserialize)]
pub struct ItemsConnection {
    #[serde(rename = "pageInfo")]
    pub page_info: PageInfo,
    pub nodes: Vec<Option<ItemNode>>,
}

#[derive(Debug, Deserialize)]
pub struct PageInfo {
    #[serde(rename = "hasNextPage")]
    pub has_next_page: bool,
    #[serde(rename = "endCursor")]
    pub end_cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ItemNode {
    pub id: String,
    #[serde(rename = "fieldValues")]
    pub field_values: FieldValueNodes,
    pub content: Option<ItemContent>,
}

#[derive(Debug, Deserialize)]
pub struct FieldValueNodes {
    pub nodes: Vec<Option<FieldValueNode>>,
}

#[derive(Debug, Deserialize)]
pub struct FieldValueNode {
    pub field: Option<FieldRef>,
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct FieldRef {
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum ItemContent {
    Issue {
        number: u64,
        title: String,
        url: String,
        #[serde(rename = "createdAt")]
        created_at: DateTime<Utc>,
        #[serde(rename = "updatedAt")]
        updated_at: DateTime<Utc>,
        assignees: AssigneeNodes,
        labels: LabelNodes,
        repository: RepoRef,
    },
    PullRequest {
        number: u64,
        title: String,
        url: String,
        #[serde(rename = "createdAt")]
        created_at: DateTime<Utc>,
        #[serde(rename = "updatedAt")]
        updated_at: DateTime<Utc>,
        assignees: AssigneeNodes,
        labels: LabelNodes,
        repository: RepoRef,
    },
}

#[derive(Debug, Deserialize)]
pub struct AssigneeNodes {
    pub nodes: Vec<Option<AssigneeNode>>,
}

#[derive(Debug, Deserialize)]
pub struct AssigneeNode {
    pub login: String,
}

#[derive(Debug, Deserialize)]
pub struct LabelNodes {
    pub nodes: Vec<Option<LabelNode>>,
}

#[derive(Debug, Deserialize)]
pub struct LabelNode {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct RepoRef {
    #[serde(rename = "nameWithOwner")]
    pub name_with_owner: String,
}

// --- Conversion ---

impl MetadataProject {
    /// Extract status columns for the given status field name.
    pub fn status_columns(&self, status_field_name: &str) -> Option<(String, Vec<StatusColumn>)> {
        for field in &self.fields.nodes {
            let Some(name) = &field.name else { continue };
            let Some(id) = &field.id else { continue };
            if name == status_field_name {
                let columns = field
                    .options
                    .iter()
                    .map(|opt| StatusColumn {
                        id: opt.id.clone(),
                        name: opt.name.clone(),
                    })
                    .collect();
                return Some((id.clone(), columns));
            }
        }
        None
    }
}

impl ItemNode {
    /// Convert to domain ProjectItem, extracting the status value for the given field name.
    pub fn to_project_item(&self, status_field_name: &str) -> Option<ProjectItem> {
        let content = self.content.as_ref()?;

        let status = self.extract_status(status_field_name);

        match content {
            ItemContent::Issue {
                number,
                title,
                url,
                created_at,
                updated_at,
                assignees,
                labels,
                repository,
            }
            | ItemContent::PullRequest {
                number,
                title,
                url,
                created_at,
                updated_at,
                assignees,
                labels,
                repository,
            } => {
                let kind = if url.contains("/pull/") {
                    ItemKind::PullRequest
                } else {
                    ItemKind::Issue
                };

                Some(ProjectItem {
                    id: self.id.clone(),
                    kind,
                    repo: repository.name_with_owner.clone(),
                    number: *number,
                    title: title.clone(),
                    url: url.clone(),
                    status,
                    assignees: assignees
                        .nodes
                        .iter()
                        .filter_map(|n| n.as_ref().map(|a| a.login.clone()))
                        .collect(),
                    labels: labels
                        .nodes
                        .iter()
                        .filter_map(|n| n.as_ref().map(|l| l.name.clone()))
                        .collect(),
                    created_at: *created_at,
                    updated_at: *updated_at,
                })
            }
        }
    }

    fn extract_status(&self, status_field_name: &str) -> Option<String> {
        for fv in self.field_values.nodes.iter().flatten() {
            if let Some(field_ref) = &fv.field
                && field_ref.name.as_deref() == Some(status_field_name)
            {
                return fv.name.clone();
            }
        }
        None
    }
}
