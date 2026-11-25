//! Purpose: Unit tests for Jira wiki-to-Markdown conversion, ticket export, and atomic snapshots.

use serde_json::json;

use super::super::export::{export_snapshot_at, export_tickets, validate_issue_key, SnapshotStamp};
use super::super::markdown::{jira_to_md, render_issue_md, render_tickets_index};
use crate::values::jira::{JiraComment, JiraIssue, JiraUser};

fn sample_comments() -> Vec<JiraComment> {
    vec![
        JiraComment {
            id: "101".into(),
            author_name: "bob".into(),
            author_key: "bob".into(),
            updated: "2026-11-20T10:00:00".into(),
            body: "First note".into(),
        },
        JiraComment {
            id: "202".into(),
            author_name: "alice".into(),
            author_key: "alice".into(),
            updated: "2026-11-21T12:00:00".into(),
            body: "*Ready* now".into(),
        },
    ]
}

fn sample_issue() -> JiraIssue {
    JiraIssue {
        key: "DEMO-42".into(),
        summary: "Deploy service | v2".into(),
        issue_type: "Story".into(),
        status: "In Progress".into(),
        priority: "High".into(),
        updated: "2026-11-22T09:30:00.000+0200".into(),
        assignee: "alice".into(),
        description: [
            "\n\nh2. Goal\n\u{00a0}*Check* items:\n* keep list item\n",
            "{code",
            ":bash}\necho ok\n{code}\n\n",
        ]
        .concat(),
        comments: sample_comments(),
        raw: json!({"key": "DEMO-42", "fields": {"summary": "Deploy service | v2"}}),
    }
}

fn alice() -> JiraUser {
    JiraUser {
        name: "alice".into(),
        key: "alice".into(),
        display_name: "Alice Example".into(),
    }
}

#[test]
fn jira_markdown_and_index_match_my_jira_formats() {
    let issue = sample_issue();
    let md = jira_to_md(&issue.description);
    assert!(md.starts_with("## Goal\n **Check** items:\n* keep list item\n```bash\necho ok\n```"));
    assert_eq!(jira_to_md("   \n "), "");
    assert_eq!(jira_to_md("unclosed *star"), "unclosed *star");

    let rendered = render_issue_md("https://jira.example.test/", &issue);
    assert!(rendered.starts_with(
        "### [DEMO-42](https://jira.example.test/browse/DEMO-42) In Progress Deploy service | v2"
    ));
    assert!(rendered.contains("#### alice 2026-11-21\n**Ready** now"));

    let index = render_tickets_index(
        "https://jira.example.test",
        &alice(),
        &[issue],
        "2026-11-22T10:00:00+02:00",
        "assignee = currentUser()",
    );
    assert!(index.contains("# Jira tickets assigned to Alice Example"));
    assert!(index.contains("| [DEMO-42](https://jira.example.test/browse/DEMO-42) | Deploy service \\| v2 | Story | In Progress | High |"));
}

fn export_one(dir: &std::path::Path, force: bool) -> crate::values::jira::ExportReport {
    let issue = sample_issue();
    export_tickets(
        dir,
        "https://jira.example.test",
        Some(&alice()),
        &[issue],
        force,
    )
    .expect("export")
}

#[test]
fn export_tickets_writes_layout_and_preserves_existing() {
    let temp = tempfile::TempDir::new().expect("temp");
    let r1 = export_one(temp.path(), false);
    assert_eq!((r1.written, r1.skipped), (4, 0));
    assert_eq!(
        std::fs::read_to_string(temp.path().join("DEMO-42/comment.id")).expect("id"),
        "202"
    );
    assert_eq!(
        std::fs::read_to_string(temp.path().join("DEMO-42/comment.md")).expect("comment"),
        "**Ready** now"
    );
    std::fs::write(temp.path().join("DEMO-42/desc.md"), "local edit").expect("edit");
    let r2 = export_one(temp.path(), false);
    assert_eq!((r2.written, r2.skipped), (0, 4));
    assert_eq!(
        std::fs::read_to_string(temp.path().join("DEMO-42/desc.md")).expect("kept"),
        "local edit"
    );
}

#[test]
fn export_tickets_force_overwrites_existing_files() {
    let temp = tempfile::TempDir::new().expect("temp");
    std::fs::create_dir_all(temp.path().join("DEMO-42")).expect("mkdir");
    std::fs::write(temp.path().join("DEMO-42/desc.md"), "old").expect("seed");
    let r3 = export_one(temp.path(), true);
    assert_eq!(r3.written, 4);
    assert!(std::fs::read_to_string(temp.path().join("DEMO-42/desc.md"))
        .expect("over")
        .contains("## Goal"));
}

fn try_snap(
    dir: &std::path::Path,
    stamp: &SnapshotStamp<'_>,
) -> Result<crate::values::jira::ExportReport, crate::values::jira::JiraError> {
    export_snapshot_at(
        dir,
        "https://jira.example.test",
        &alice(),
        &[sample_issue()],
        1,
        "assignee = currentUser()",
        stamp,
    )
}

#[test]
fn validate_issue_key_and_atomic_snapshot() {
    for bad in ["../escape", "bad-key", "DEMO", "DEMO-abc", "123-1"] {
        assert!(validate_issue_key(bad).is_err());
    }
    let temp = tempfile::TempDir::new().expect("temp");
    let stamp = SnapshotStamp {
        date_stamp: "2026-11-22",
        time_stamp: "101500",
        exported_at: "2026-11-22T10:15:00+02:00",
    };
    let first = try_snap(temp.path(), &stamp).expect("first snap");
    assert!(
        first.output_dir.ends_with("2026-11-22")
            && first.output_dir.join("metadata.json").is_file()
    );
    assert!(
        first.output_dir.join("issues.json").is_file()
            && first.output_dir.join("tickets.md").is_file()
    );
    let second = try_snap(temp.path(), &stamp).expect("second snap");
    assert!(second.output_dir.ends_with("2026-11-22-101500"));
    assert!(try_snap(temp.path(), &stamp).is_err());
}

#[test]
fn export_snapshot_writes_dated_directory() {
    let temp = tempfile::TempDir::new().expect("temp");
    let report = super::super::export::export_snapshot(
        temp.path(),
        "https://jira.example.test",
        &alice(),
        &[sample_issue()],
        1,
        "assignee = currentUser()",
    )
    .expect("snapshot");
    assert!(report.output_dir.join("tickets.md").is_file());
}
