//! Purpose: Convert Jira wiki markup to Markdown matching `sync_kn_tickets.py` & `export_my_jira.py`.

use crate::values::jira::{JiraIssue, JiraUser};

const BLOCK_REPLACEMENTS: &[(&str, &str)] = &[
    ("{code:python}", "```python"),
    ("{code:bash}", "```bash"),
    ("{code:java}", "```java"),
    ("{code:yaml}", "```yaml"),
    ("{code:json}", "```json"),
    ("{code:xml}", "```xml"),
    ("{code:yml}", "```yml"),
    ("{code:sql}", "```sql"),
    ("{code:sh}", "```sh"),
    ("{code}", "```"),
    ("h2. ", "## "),
    ("h3. ", "### "),
    ("h4. ", "#### "),
];

/// Convert Jira wiki markup into cleaned Markdown matching `sync_kn_tickets.py`.
#[must_use]
pub fn jira_to_md(text: &str) -> String {
    if text.trim().is_empty() {
        return String::new();
    }
    let mut replaced = text.replace('\u{00a0}', " ");
    for &(from, to) in BLOCK_REPLACEMENTS {
        replaced = replaced.replace(from, to);
    }
    let mut lines: Vec<String> = replaced
        .lines()
        .map(|line| format_line(line.trim_end()))
        .collect();
    while lines.first().is_some_and(String::is_empty) {
        lines.remove(0);
    }
    while lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    lines.join("\n")
}

fn format_line(line: &str) -> String {
    if line.trim_start().starts_with("* ") {
        return line.to_owned();
    }
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    while let Some((before, tail)) = rest.split_once('*') {
        out.push_str(before);
        if let Some((inner, after)) = tail.split_once('*') {
            out.push_str("**");
            out.push_str(inner);
            out.push_str("**");
            rest = after;
            continue;
        }
        out.push('*');
        rest = tail;
    }
    out.push_str(rest);
    out
}

/// Render `issue.md` containing header link, description, and all comments.
#[must_use]
pub fn render_issue_md(base_url: &str, issue: &JiraIssue) -> String {
    let url = format!("{}/browse/{}", base_url.trim_end_matches('/'), issue.key);
    let mut lines = vec![
        format!(
            "### [{}]({url}) {} {}",
            issue.key, issue.status, issue.summary
        ),
        String::new(),
    ];
    let desc = jira_to_md(&issue.description);
    if !desc.is_empty() {
        lines.push(desc);
        lines.push(String::new());
    }
    for comment in &issue.comments {
        let author = if comment.author_name.is_empty() {
            "?"
        } else {
            &comment.author_name
        };
        let day: String = comment.updated.chars().take(10).collect();
        lines.push(format!("\n#### {author} {day}"));
        let body = jira_to_md(&comment.body);
        if !body.is_empty() {
            lines.push(body);
        }
    }
    lines.join("\n")
}

/// Render `tickets.md` Markdown index matching `export_my_jira.py`.
#[must_use]
pub fn render_tickets_index(
    base_url: &str,
    user: &JiraUser,
    issues: &[JiraIssue],
    exported_at: &str,
    jql: &str,
) -> String {
    let base = base_url.trim_end_matches('/');
    let mut lines = vec![
        format!("# Jira tickets assigned to {}", user.display_label()),
        String::new(),
        format!("Exported: {exported_at}"),
        format!("JQL: `{jql}`"),
        format!("Jira-reported total: {}", issues.len()),
        String::new(),
        "| Key | Summary | Type | Status | Priority | Updated |".to_owned(),
        "|---|---|---|---|---|---|".to_owned(),
    ];
    for issue in issues {
        lines.push(format!(
            "| [{}]({base}/browse/{}) | {} | {} | {} | {} | {} |",
            issue.key,
            issue.key,
            cell(&issue.summary),
            cell(&issue.issue_type),
            cell(&issue.status),
            cell(&issue.priority),
            cell(&issue.updated),
        ));
    }
    lines.push(String::new());
    lines.join("\n")
}

fn cell(raw: &str) -> String {
    raw.replace('|', "\\|").replace(['\r', '\n'], " ")
}
