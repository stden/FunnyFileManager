//! Purpose: Unit tests for AI chat roles and workspace context prompt formatting.

use super::{Role, WorkspaceContext};

#[test]
fn role_labels_cover_user_and_assistant() {
    assert_eq!(Role::User.as_api_str(), "user");
    assert_eq!(Role::Assistant.as_api_str(), "assistant");
    assert_eq!(Role::User.prefix(), "You");
    assert_eq!(Role::Assistant.prefix(), "AI");
}

#[test]
fn prompt_summary_handles_empty_and_populated_selections() {
    let minimal = WorkspaceContext {
        active_dir: "/srv/work".to_owned(),
        opposite_dir: "/srv/data".to_owned(),
        selected: None,
        marked: Vec::new(),
        os: "linux",
    };
    let summary = minimal.prompt_summary();
    assert!(summary.contains("OS: linux"));
    assert!(!summary.contains("Selected item:"));
    assert!(!summary.contains("Marked items:"));

    let rich = WorkspaceContext {
        selected: Some("file.rs".to_owned()),
        marked: vec!["one.rs".to_owned(), "two.rs".to_owned()],
        ..minimal
    };
    let rich_summary = rich.prompt_summary();
    assert!(rich_summary.contains("Selected item: file.rs"));
    assert!(rich_summary.contains("Marked items: one.rs, two.rs"));
}
