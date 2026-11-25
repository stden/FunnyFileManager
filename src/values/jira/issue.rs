//! Purpose: Jira user, comment, and issue models.

/// Authenticated Jira user returned by `/rest/api/2/myself`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct JiraUser {
    pub name: String,
    pub key: String,
    pub display_name: String,
}

impl JiraUser {
    /// Non-empty display label preferring `display_name`, then `name`, then `key`.
    #[must_use]
    pub fn display_label(&self) -> &str {
        if !self.display_name.is_empty() {
            &self.display_name
        } else if !self.name.is_empty() {
            &self.name
        } else {
            &self.key
        }
    }

    /// Whether `author_name` or `author_key` identifies this user.
    #[must_use]
    pub fn matches_author(&self, author_name: &str, author_key: &str) -> bool {
        (!self.name.is_empty() && (author_name == self.name || author_key == self.name))
            || (!self.key.is_empty() && (author_name == self.key || author_key == self.key))
    }
}

/// One comment on a Jira issue.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct JiraComment {
    pub id: String,
    pub author_name: String,
    pub author_key: String,
    pub updated: String,
    pub body: String,
}

/// Parsed Jira issue alongside its raw JSON object for full `issues.json` snapshots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JiraIssue {
    pub key: String,
    pub summary: String,
    pub issue_type: String,
    pub status: String,
    pub priority: String,
    pub updated: String,
    pub assignee: String,
    pub description: String,
    pub comments: Vec<JiraComment>,
    pub raw: serde_json::Value,
}

#[cfg(test)]
mod tests;
