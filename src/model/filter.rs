use super::project_item::{ItemKind, ProjectItem};
use crate::config::FilterConfig;

#[derive(Debug, Default, Clone)]
pub struct Filter {
    pub text: Option<String>,
    pub status: Option<String>,
    pub assignee: Option<String>,
    pub label: Option<String>,
    pub kind: Option<ItemKind>,
}

impl Filter {
    /// Check if this filter matches a given item. All conditions are AND-combined.
    pub fn matches(&self, item: &ProjectItem) -> bool {
        if let Some(text) = &self.text {
            let t = text.to_lowercase();
            let title_match = item.title.to_lowercase().contains(&t);
            let ref_match = format!("{}#{}", item.repo, item.number)
                .to_lowercase()
                .contains(&t);
            let number_match = if let Some(stripped) = t.strip_prefix('#') {
                stripped == item.number.to_string()
            } else {
                false
            };
            if !title_match && !ref_match && !number_match {
                return false;
            }
        }

        if let Some(status) = &self.status {
            match &item.status {
                Some(s) if s.to_lowercase() == status.to_lowercase() => {}
                _ => return false,
            }
        }

        if let Some(assignee) = &self.assignee {
            let a = assignee.to_lowercase();
            if !item.assignees.iter().any(|x| x.to_lowercase() == a) {
                return false;
            }
        }

        if let Some(label) = &self.label {
            let l = label.to_lowercase();
            if !item.labels.iter().any(|x| x.to_lowercase() == l) {
                return false;
            }
        }

        if let Some(kind) = &self.kind
            && &item.kind != kind
        {
            return false;
        }

        true
    }

    /// Returns true if no conditions are set.
    pub fn is_empty(&self) -> bool {
        self.text.is_none()
            && self.status.is_none()
            && self.assignee.is_none()
            && self.label.is_none()
            && self.kind.is_none()
    }

    /// Parse a filter query string like "label:bug assignee:alice is:pr fix login".
    /// Unrecognized tokens are treated as title text search.
    pub fn parse(query: &str) -> Self {
        let mut filter = Filter::default();
        let mut text_parts = Vec::new();

        for token in query.split_whitespace() {
            if let Some(val) = token.strip_prefix("label:") {
                filter.label = Some(val.to_string());
            } else if let Some(val) = token.strip_prefix("assignee:") {
                filter.assignee = Some(val.to_string());
            } else if let Some(val) = token.strip_prefix("status:") {
                filter.status = Some(val.to_string());
            } else if let Some(val) = token.strip_prefix("is:") {
                match val.to_lowercase().as_str() {
                    "pr" | "pullrequest" => filter.kind = Some(ItemKind::PullRequest),
                    "issue" => filter.kind = Some(ItemKind::Issue),
                    _ => text_parts.push(token.to_string()),
                }
            } else {
                text_parts.push(token.to_string());
            }
        }

        if !text_parts.is_empty() {
            filter.text = Some(text_parts.join(" "));
        }

        filter
    }

    /// Build a display string summarizing current filter conditions.
    pub fn display_summary(&self) -> String {
        let mut parts = Vec::new();
        if let Some(text) = &self.text {
            parts.push(format!("\"{}\"", text));
        }
        if let Some(label) = &self.label {
            parts.push(format!("label:{}", label));
        }
        if let Some(assignee) = &self.assignee {
            parts.push(format!("assignee:{}", assignee));
        }
        if let Some(status) = &self.status {
            parts.push(format!("status:{}", status));
        }
        if let Some(kind) = &self.kind {
            match kind {
                ItemKind::Issue => parts.push("is:issue".into()),
                ItemKind::PullRequest => parts.push("is:pr".into()),
            }
        }
        parts.join(" ")
    }
}

impl From<&FilterConfig> for Filter {
    fn from(config: &FilterConfig) -> Self {
        let kind = config
            .kind
            .as_ref()
            .and_then(|k| match k.to_lowercase().as_str() {
                "issue" => Some(ItemKind::Issue),
                "pr" | "pullrequest" => Some(ItemKind::PullRequest),
                _ => None,
            });

        // For default filter, only first label is used (simple v1 behavior).
        let label = config
            .labels
            .as_ref()
            .and_then(|labels| labels.first().cloned());

        Filter {
            text: None,
            status: config.status.clone(),
            assignee: config.assignee.clone(),
            label,
            kind,
        }
    }
}
