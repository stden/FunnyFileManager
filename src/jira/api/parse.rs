//! Purpose: Decode Jira REST v2 issue, comment, and user JSON objects.

use serde_json::Value;

use crate::values::jira::{JiraComment, JiraError, JiraIssue};

/// Parse one Jira REST v2 issue JSON record into typed fields + raw snapshot JSON.
pub fn parse_issue(value: Value) -> Result<JiraIssue, JiraError> {
    let key = str_field(&value, "key");
    if key.is_empty() {
        return Err(JiraError::InvalidResponse(
            "Jira issue record is missing key".to_owned(),
        ));
    }
    let fields = value.get("fields").unwrap_or(&Value::Null);
    Ok(JiraIssue {
        key,
        summary: str_field(fields, "summary"),
        issue_type: nested_name(fields, "issuetype"),
        status: nested_name(fields, "status"),
        priority: nested_name(fields, "priority"),
        updated: str_field(fields, "updated"),
        assignee: nested_name_or_display(fields, "assignee"),
        description: str_field(fields, "description"),
        comments: parse_comments(fields),
        raw: value,
    })
}

fn parse_comments(fields: &Value) -> Vec<JiraComment> {
    let Some(list) = fields
        .get("comment")
        .and_then(|c| c.get("comments"))
        .and_then(Value::as_array)
    else {
        return Vec::new();
    };
    list.iter()
        .map(|entry| {
            let author = entry.get("author").unwrap_or(&Value::Null);
            JiraComment {
                id: str_field(entry, "id"),
                author_name: str_field(author, "name"),
                author_key: str_field(author, "key"),
                updated: str_field(entry, "updated"),
                body: str_field(entry, "body"),
            }
        })
        .collect()
}

pub(super) fn str_field(value: &Value, name: &str) -> String {
    value
        .get(name)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned()
}

fn nested_name(value: &Value, field: &str) -> String {
    value
        .get(field)
        .and_then(|obj| obj.get("name"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned()
}

fn nested_name_or_display(value: &Value, field: &str) -> String {
    let Some(obj) = value.get(field) else {
        return String::new();
    };
    for key in ["name", "displayName", "key"] {
        if let Some(text) = obj.get(key).and_then(Value::as_str) {
            if !text.is_empty() {
                return text.to_owned();
            }
        }
    }
    String::new()
}
