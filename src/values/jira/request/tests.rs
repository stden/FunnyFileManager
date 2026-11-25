//! Purpose: Unit tests for `JiraBusy`, `JiraReply::fail`, and `JiraError` formatting.

use std::path::PathBuf;

use super::{JiraBusy, JiraError, JiraReply, JiraRequest};

#[test]
fn jira_error_display_formats_every_variant() {
    assert!(JiraError::Disabled.to_string().contains("Unavailable"));
    assert_eq!(JiraError::Config("cfg".to_owned()).to_string(), "cfg");
    assert_eq!(JiraError::Network("net".to_owned()).to_string(), "net");
    assert_eq!(
        JiraError::InvalidResponse("bad".to_owned()).to_string(),
        "bad"
    );
    assert_eq!(
        JiraError::Http(404, "missing".to_owned()).to_string(),
        "Jira HTTP 404: missing"
    );
    assert_eq!(
        JiraError::Io("disk full".to_owned()).to_string(),
        "Jira export failed: disk full"
    );
    assert_eq!(
        JiraError::Worker.to_string(),
        "Jira worker stopped unexpectedly"
    );
}

#[test]
fn jira_request_busy_and_reply_fail_cover_fetch_and_export_variants() {
    let fetch = JiraRequest::Fetch {
        jql: "x".to_owned(),
        page_size: 10,
    };
    assert_eq!(fetch.busy(), JiraBusy::Fetching);
    assert!(matches!(
        JiraReply::fail(&fetch, JiraError::Worker),
        JiraReply::Fetched(Err(JiraError::Worker))
    ));
    let export = JiraRequest::ExportKey {
        key: "DEMO-1".to_owned(),
        output_dir: PathBuf::from("."),
        force: false,
    };
    assert_eq!(export.busy(), JiraBusy::Exporting);
    assert!(matches!(
        JiraReply::fail(&export, JiraError::Io("oops".to_owned())),
        JiraReply::Exported(Err(JiraError::Io(_)))
    ));
}
