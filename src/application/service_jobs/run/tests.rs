//! Purpose: Unit tests for `run_live_jira` offline export and config error/valid paths.

use std::path::Path;

use super::run_live_jira;
use crate::test_support::{EnvVarsGuard, CONFIG_HOME};
use crate::values::jira::{JiraError, JiraReply, JiraRequest};

#[test]
fn run_live_jira_exports_tickets_locally() {
    let temp = tempfile::TempDir::new().expect("temp");
    let reply = run_live_jira(JiraRequest::ExportTickets {
        issues: Vec::new(),
        user: None,
        base_url: "https://jira.example.test".into(),
        output_dir: temp.path().to_path_buf(),
        force: false,
    });
    assert!(matches!(reply, JiraReply::Exported(Ok(_))));
}

#[test]
fn run_live_jira_handles_fetch_missing_and_invalid_ca_configs() {
    let temp = tempfile::TempDir::new().expect("temp");
    let g1 = EnvVarsGuard::set(&[(CONFIG_HOME, temp.path()), ("JIRA_URL", Path::new(""))]);
    let req = JiraRequest::Fetch {
        jql: "jql".into(),
        page_size: 10,
    };
    assert!(matches!(
        run_live_jira(req.clone()),
        JiraReply::Fetched(Err(JiraError::Config(_)))
    ));
    drop(g1);

    let _g2 = EnvVarsGuard::set(&[
        (CONFIG_HOME, temp.path()),
        ("JIRA_URL", Path::new("https://jira.example.test")),
        ("JIRA_PERSONAL_TOKEN", Path::new("tok")),
        ("JIRA_CA_CERT", Path::new("/nonexistent/ca.pem")),
    ]);
    assert!(matches!(
        run_live_jira(req),
        JiraReply::Fetched(Err(JiraError::Config(_)))
    ));
}
