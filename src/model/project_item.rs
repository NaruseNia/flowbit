use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ItemKind {
    Issue,
    PullRequest,
}

impl fmt::Display for ItemKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ItemKind::Issue => write!(f, "Issue"),
            ItemKind::PullRequest => write!(f, "PR"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectItem {
    pub id: String,
    pub kind: ItemKind,
    pub repo: String,
    pub number: u64,
    pub title: String,
    pub url: String,
    pub status: Option<String>,
    pub assignees: Vec<String>,
    pub labels: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A status column in the project board, preserving the order defined in the project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusColumn {
    pub id: String,
    pub name: String,
}
