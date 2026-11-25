//! Purpose: Unit tests for Jira operations (`open`, commands, export actions, and `publish`).

use serde_json::json;

use super::actions::{
    begin_edit_jql, queue_export_all, queue_export_selected, queue_snapshot, reload, submit_jql,
    view_selected,
};
use super::*;
use crate::models::services::jira::JiraMode;
use crate::models::{Panel, RowMotion};
use crate::values::jira::{ExportReport, JiraIssue, JiraListing, JiraUser};

fn test_app(dir: &std::path::Path) -> App {
    let panel = Panel::new(dir.to_path_buf()).expect("panel");
    App::with_panels(panel.clone(), panel)
}

fn sample_issue(key: &str) -> JiraIssue {
    JiraIssue {
        key: key.into(),
        summary: "Test task".into(),
        issue_type: "Story".into(),
        status: "Open".into(),
        priority: "High".into(),
        updated: "2026-11-22".into(),
        assignee: "alice".into(),
        description: "Details".into(),
        comments: Vec::new(),
        raw: json!({"key": key}),
    }
}

#[test]
fn open_and_commands_populate_jira_overlay_and_requests() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = test_app(temp.path());
    command(&mut app, JiraCommand::Browse).expect("browse");
    assert!(matches!(app.overlay, Overlay::Jira(_)));

    command(&mut app, JiraCommand::Jql("project = DEMO".into())).expect("jql");
    let Overlay::Jira(d) = &app.overlay else {
        panic!("expected Jira overlay");
    };
    assert_eq!(d.jql, "project = DEMO");
    open_with_jql(&mut app, Some("project = ALT"));
    assert!(matches!(&app.overlay, Overlay::Jira(d) if d.jql == "project = ALT"));

    command(&mut app, JiraCommand::Export(Some("DEMO-1".into()))).expect("export key");
    assert!(matches!(app.overlay, Overlay::Jira(_)));
    command(&mut app, JiraCommand::Export(None)).expect("export dialog");
    command(
        &mut app,
        JiraCommand::Snapshot(Some("assignee = currentUser()".into())),
    )
    .expect("snap");
    let _guard =
        crate::test_support::EnvVarGuard::set(crate::test_support::CONFIG_HOME, temp.path());
    command(&mut app, JiraCommand::Edit).expect("edit config");
    assert!(matches!(app.overlay, Overlay::Editor(_)));
}

#[test]
fn publish_and_actions_drive_detail_export_and_jql_reloads() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = test_app(temp.path());
    open(&mut app);
    let (generation, id) = match &mut app.overlay {
        Overlay::Jira(d) => (Arc::clone(&d.generation), d.take_queued().expect("fetch").0),
        _ => panic!("expected Jira"),
    };
    let listing = JiraListing {
        user: JiraUser {
            name: "alice".into(),
            ..JiraUser::default()
        },
        issues: vec![sample_issue("DEMO-7")],
        total: 1,
        jql: "jql".into(),
        base_url: "https://jira.example.test".into(),
    };
    publish(&mut app, &generation, id, JiraReply::Fetched(Ok(listing)));
    view_selected(&mut app);
    assert!(matches!(&app.overlay, Overlay::Jira(d) if matches!(d.mode, JiraMode::Detail { .. })));
    exercise_jql_and_export(&mut app, temp.path());
}

fn exercise_jql_and_export(app: &mut App, dir: &std::path::Path) {
    begin_edit_jql(app);
    if let Overlay::Jira(d) = &mut app.overlay {
        if let JiraMode::EditJql(ed) = &mut d.mode {
            ed.set_text("project = DEMO");
        }
        d.move_cursor(RowMotion::First);
    }
    submit_jql(app);
    reload(app);
    queue_export_selected(app, false);
    queue_export_all(app, true);
    queue_snapshot(app);
    let (g2, id2) = match &mut app.overlay {
        Overlay::Jira(d) => (Arc::clone(&d.generation), d.take_queued().expect("snap").0),
        _ => panic!("expected Jira"),
    };
    let report = ExportReport {
        summary: "ok".into(),
        output_dir: dir.to_path_buf(),
        written: 3,
        skipped: 0,
    };
    publish(app, &g2, id2, JiraReply::Exported(Ok(report)));
    publish(app, &g2, id2, JiraReply::Fetched(Err(JiraError::Disabled)));
}

fn jira_env(home: &std::path::Path, url: &str, token: &str) -> crate::test_support::EnvVarsGuard {
    use std::path::Path;
    crate::test_support::EnvVarsGuard::set(&[
        (crate::test_support::CONFIG_HOME, home),
        ("JIRA_URL", Path::new(url)),
        ("JIRA_PERSONAL_TOKEN", Path::new(token)),
        ("ATLASSIAN_JIRA_PERSONAL_TOKEN", Path::new("")),
        ("JIRA_API_TOKEN", Path::new("")),
        ("JIRA_TOKEN", Path::new("")),
        ("JIRA_PAT", Path::new("")),
    ])
}

#[test]
fn network_mode_reports_missing_jira_config_and_ignores_stale_publish() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = test_app(temp.path());
    app.live.jira = JiraAccess::Network;
    let _env = jira_env(temp.path(), "", "");
    open_with_jql(&mut app, None);
    assert!(matches!(&app.overlay, Overlay::Jira(d) if d.setup_error.is_some()));
    command(&mut app, JiraCommand::Export(Some("DEMO-1".into()))).expect("export");
    command(&mut app, JiraCommand::Snapshot(None)).expect("snap");
    assert!(matches!(&app.overlay, Overlay::Jira(d) if d.setup_error.is_some()));

    let stale_id = crate::models::services::jira::JiraRequestId::default();
    publish(
        &mut app,
        &Arc::new(()),
        stale_id,
        JiraReply::Fetched(Err(JiraError::Worker)),
    );
    app.close_overlay();
    publish(
        &mut app,
        &Arc::new(()),
        stale_id,
        JiraReply::Fetched(Err(JiraError::Worker)),
    );
}

#[test]
fn network_mode_loads_default_settings_jql_and_ready_dialogs() {
    let temp = tempfile::TempDir::new().expect("temp");
    let cfg_dir = temp.path().join("ffm");
    std::fs::create_dir_all(&cfg_dir).expect("mkdir");
    std::fs::write(
        cfg_dir.join(crate::config::document::FILE_NAME),
        "jira:\n  url: https://jira.example.test\n  jql: project = YML\n",
    )
    .expect("yaml");
    let _env = jira_env(temp.path(), "", "tok-123");
    let mut app = test_app(temp.path());
    app.live.jira = JiraAccess::Network;
    open_with_jql(&mut app, None);
    assert!(matches!(&app.overlay, Overlay::Jira(d) if d.jql == "project = YML"));
    command(&mut app, JiraCommand::Export(Some("DEMO-1".into()))).expect("export ready");
}
