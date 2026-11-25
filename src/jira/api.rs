//! Purpose: Jira REST API v2 queries, key validation, and pagination.

mod parse;

pub use parse::parse_issue;

use std::collections::HashSet;

use serde_json::{json, Value};

use super::client::Transport;
use super::export::validate_issue_key;
use crate::values::jira::{JiraError, JiraIssue, JiraUser};
use parse::str_field;

/// Fetch the authenticated user via `/rest/api/2/myself`.
pub fn fetch_myself(transport: &mut dyn Transport) -> Result<JiraUser, JiraError> {
    let value = transport.get("/rest/api/2/myself")?;
    let (name, key) = (str_field(&value, "name"), str_field(&value, "key"));
    if name.is_empty() && key.is_empty() {
        return Err(JiraError::InvalidResponse(
            "Jira did not return an authenticated username".to_owned(),
        ));
    }
    Ok(JiraUser {
        name,
        key,
        display_name: str_field(&value, "displayName"),
    })
}

/// Paginate through `POST /rest/api/2/search` verifying total count and unique keys.
pub fn search_all(
    transport: &mut dyn Transport,
    jql: &str,
    page_size: u16,
    fields: &[&str],
) -> Result<(Vec<JiraIssue>, usize), JiraError> {
    let (mut issues, mut start_at, mut reported_total) = (Vec::new(), 0usize, None);
    let clamped = page_size.clamp(1, 100);
    while reported_total.is_none_or(|total| start_at < total) {
        let body = json!({
            "jql": jql,
            "startAt": start_at,
            "maxResults": clamped,
            "fields": fields,
        });
        let (batch, total) = decode_page(
            &transport.post("/rest/api/2/search", &body)?,
            reported_total,
        )?;
        reported_total = Some(total);
        if batch.is_empty() {
            break;
        }
        start_at = start_at.saturating_add(batch.len());
        issues.extend(batch);
    }
    let total = reported_total.unwrap_or(0);
    validate_results(&issues, total)?;
    Ok((issues, total))
}

/// Fetch one validated Jira issue key via `/rest/api/2/issue/{key}`.
pub fn fetch_single(
    transport: &mut dyn Transport,
    key: &str,
    fields: &[&str],
) -> Result<JiraIssue, JiraError> {
    validate_issue_key(key)?;
    let path = format!("/rest/api/2/issue/{key}?fields={}", fields.join(","));
    parse_issue(transport.get(&path)?)
}

fn decode_page(
    page: &Value,
    prior_total: Option<usize>,
) -> Result<(Vec<JiraIssue>, usize), JiraError> {
    let raw_issues = page
        .get("issues")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            JiraError::InvalidResponse(
                "Unexpected Jira search response: missing issues list".to_owned(),
            )
        })?;
    let total_u64 = page.get("total").and_then(Value::as_u64).ok_or_else(|| {
        JiraError::InvalidResponse(
            "Unexpected Jira search response: missing total count".to_owned(),
        )
    })?;
    let total = usize::try_from(total_u64).unwrap_or(usize::MAX);
    if let Some(prev) = prior_total.filter(|&prev| prev != total) {
        return Err(JiraError::InvalidResponse(format!(
            "Jira total changed during fetch: {prev} -> {total}"
        )));
    }
    let batch = raw_issues
        .iter()
        .cloned()
        .map(parse_issue)
        .collect::<Result<Vec<_>, _>>()?;
    Ok((batch, total))
}

fn validate_results(issues: &[JiraIssue], total: usize) -> Result<(), JiraError> {
    if issues.len() != total {
        return Err(JiraError::InvalidResponse(format!(
            "Incomplete Jira fetch: expected {total}, got {}",
            issues.len()
        )));
    }
    let mut seen = HashSet::with_capacity(issues.len());
    for issue in issues {
        if !seen.insert(issue.key.as_str()) {
            return Err(JiraError::InvalidResponse(format!(
                "Duplicate Jira key returned: {}",
                issue.key
            )));
        }
    }
    Ok(())
}
