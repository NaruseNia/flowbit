/// Query to fetch project metadata: title and Status field options.
/// Variables: $owner (String!), $number (Int!), $statusField (String!)
pub const PROJECT_METADATA: &str = r#"
query($owner: String!, $number: Int!) {
  user(login: $owner) {
    projectV2(number: $number) {
      id
      title
      fields(first: 50) {
        nodes {
          ... on ProjectV2SingleSelectField {
            id
            name
            options {
              id
              name
            }
          }
        }
      }
    }
  }
}
"#;

/// Same query but for organization owner.
pub const PROJECT_METADATA_ORG: &str = r#"
query($owner: String!, $number: Int!) {
  organization(login: $owner) {
    projectV2(number: $number) {
      id
      title
      fields(first: 50) {
        nodes {
          ... on ProjectV2SingleSelectField {
            id
            name
            options {
              id
              name
            }
          }
        }
      }
    }
  }
}
"#;

/// Query to fetch project items with pagination.
/// Variables: $projectId (ID!), $cursor (String), $statusFieldId (ID!)
pub const PROJECT_ITEMS: &str = r#"
query($projectId: ID!, $cursor: String) {
  node(id: $projectId) {
    ... on ProjectV2 {
      items(first: 100, after: $cursor) {
        pageInfo {
          hasNextPage
          endCursor
        }
        nodes {
          id
          fieldValues(first: 20) {
            nodes {
              ... on ProjectV2ItemFieldSingleSelectValue {
                field {
                  ... on ProjectV2SingleSelectField {
                    name
                  }
                }
                name
              }
            }
          }
          content {
            ... on Issue {
              number
              title
              url
              createdAt
              updatedAt
              assignees(first: 10) {
                nodes { login }
              }
              labels(first: 10) {
                nodes { name }
              }
              repository { nameWithOwner }
            }
            ... on PullRequest {
              number
              title
              url
              createdAt
              updatedAt
              assignees(first: 10) {
                nodes { login }
              }
              labels(first: 10) {
                nodes { name }
              }
              repository { nameWithOwner }
            }
          }
        }
      }
    }
  }
}
"#;
