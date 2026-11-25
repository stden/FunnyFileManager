//! Purpose: Unit tests for Jira API decoding, pagination, and stubbed execution.

mod config;
mod export_and_md;
mod export_edges;
mod http;
mod http_edges;

use std::collections::BTreeMap;

use serde_json::json;

use super::*;
use crate::values::jira::{JiraBusy, JiraSettings, DEFAULT_JQL};

struct StubTransport {
    responses: Vec<Result<serde_json::Value, JiraError>>,
    calls: Vec<String>,
}

impl StubTransport {
    fn new(responses: Vec<Result<serde_json::Value, JiraError>>) -> Self {
        Self {
            responses,
            calls: Vec::new(),
        }
    }
}

impl Transport for StubTransport {
    fn get(&mut self, path: &str) -> Result<serde_json::Value, JiraError> {
        self.calls.push(format!("GET {path}"));
        self.responses.remove(0)
    }

    fn post(
        &mut self,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value, JiraError> {
        let start = body
            .get("startAt")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0);
        self.calls.push(format!("POST {path}@{start}"));
        self.responses.remove(0)
    }
}

pub(super) fn env_lookup(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> + use<> {
    let map: BTreeMap<String, String> = pairs
        .iter()
        .map(|&(k, v)| (k.to_owned(), v.to_owned()))
        .collect();
    move |key| map.get(key).cloned()
}

#[test]
fn search_pagination_validates_totals_keys_and_user_info() {
    let mut tr = StubTransport::new(vec![
        Ok(json!({"name": "alice", "displayName": "Alice Example"})),
        Ok(json!({
            "total": 2,
            "issues": [{"key": "DEMO-1", "fields": {"summary": "One", "status": {"name": "Open"}}}]
        })),
        Ok(json!({
            "total": 2,
            "issues": [{"key": "DEMO-2", "fields": {"summary": "Two", "assignee": {"displayName": "Bob"}}}]
        })),
    ]);
    let req = JiraRequest::Fetch {
        jql: DEFAULT_JQL.into(),
        page_size: 1,
    };
    assert_eq!(req.busy().label(), JiraBusy::Fetching.label());
    let JiraReply::Fetched(Ok(JiraListing {
        user,
        issues,
        total,
        ..
    })) = execute_with(&mut tr, "https://jira.example.test", req)
    else {
        panic!("expected listing");
    };
    assert_eq!(
        (total, issues.len(), user.display_label()),
        (2, 2, "Alice Example")
    );
    assert!(user.matches_author("alice", ""));
}

#[test]
fn execute_with_rejects_broken_pagination() {
    let mut bad = StubTransport::new(vec![
        Ok(json!({})),
        Ok(json!({"name": "alice"})),
        Ok(json!({"issues": [], "total": 2})),
        Ok(json!({"name": "alice"})),
        Ok(json!({"total": 2, "issues": [{"key": "DEMO-1", "fields": {}}]})),
        Ok(json!({"total": 3, "issues": []})),
    ]);
    let req = JiraRequest::Fetch {
        jql: "x".into(),
        page_size: 1,
    };
    for _ in 0..3 {
        assert!(matches!(
            execute_with(&mut bad, "https://jira.example.test", req.clone()),
            JiraReply::Fetched(Err(_))
        ));
    }
}

#[test]
fn unavailable_returns_disabled_for_jira_requests() {
    let req = JiraRequest::Fetch {
        jql: "x".into(),
        page_size: 1,
    };
    assert!(matches!(
        unavailable(&req),
        JiraReply::Fetched(Err(JiraError::Disabled))
    ));
    let snap = JiraRequest::ExportSnapshot {
        jql: "x".into(),
        page_size: 5,
        output_dir: ".".into(),
    };
    assert_eq!(snap.busy().label(), JiraBusy::Exporting.label());
    assert!(matches!(
        unavailable(&snap),
        JiraReply::Exported(Err(JiraError::Disabled))
    ));
}

#[test]
fn api_helpers_fetch_myself_search_all_and_single_issue() {
    let mut tr = StubTransport::new(vec![
        Ok(json!({"name": "alice", "displayName": "Alice Example"})),
        Ok(json!({"total": 1, "issues": [{"key": "DEMO-1", "fields": {"summary": "One"}}]})),
        Ok(json!({"key": "DEMO-1", "fields": {"summary": "One"}})),
    ]);
    let me = api::fetch_myself(&mut tr).expect("me");
    assert_eq!(me.display_label(), "Alice Example");
    let (found, total) = api::search_all(&mut tr, "jql", 10, &["summary"]).expect("search");
    assert_eq!((found.len(), total), (1, 1));
    let single = api::fetch_single(&mut tr, "DEMO-1", &["summary"]).expect("single");
    assert_eq!(single.key, "DEMO-1");
    let parsed =
        api::parse_issue(json!({"key": "DEMO-2", "fields": {"summary": "Two"}})).expect("parse");
    assert_eq!(parsed.summary, "Two");
}

#[test]
fn api_rejects_malformed_pages_duplicates_and_decodes_comments() {
    assert!(api::parse_issue(json!({"fields": {}})).is_err());
    let commented = api::parse_issue(json!({
        "key": "DEMO-3",
        "fields": {
            "assignee": {"name": "", "displayName": ""},
            "comment": {"comments": [{"id": "9", "author": {"name": "a", "key": "k"}, "updated": "2026-11-22", "body": "ok"}]}
        }
    }))
    .expect("commented");
    assert_eq!(
        (commented.assignee.as_str(), commented.comments.len()),
        ("", 1)
    );

    let mut bad = StubTransport::new(vec![
        Ok(json!({"total": 0})),
        Ok(json!({"issues": []})),
        Ok(json!({"total": 2, "issues": [{"key": "DEMO-1"}, {"key": "DEMO-1"}]})),
    ]);
    for _ in 0..3 {
        assert!(api::search_all(&mut bad, "x", 10, &["summary"]).is_err());
    }
}
