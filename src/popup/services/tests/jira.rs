//! Purpose: Key routing tests for the Jira issue browser, detail reader, and JQL editor.

use super::{app_in, press};
use crate::models::services::jira::{JiraDialog, JiraMode};
use crate::models::Overlay;
use crate::values::jira::{JiraIssue, JiraRequest};
use crossterm::event::{KeyCode, KeyModifiers};
use serde_json::json;

fn issue(key: &str, summary: &str, status: &str, assignee: &str) -> JiraIssue {
    JiraIssue {
        key: key.into(),
        summary: summary.into(),
        issue_type: "Story".into(),
        status: status.into(),
        priority: "High".into(),
        updated: "2026-11-22".into(),
        assignee: assignee.into(),
        description: "Body".into(),
        comments: Vec::new(),
        raw: json!({"key": key}),
    }
}

fn seed_jira(dir: &std::path::Path) -> crate::models::App {
    let mut app = app_in(dir);
    let mut dialog = JiraDialog::new_setup(
        String::new(),
        "assignee = currentUser()".into(),
        dir.to_path_buf(),
    );
    dialog.issues = vec![
        issue("DEMO-1", "Alpha task", "Open", "alice"),
        issue("DEMO-2", "Beta bug", "Done", "bob"),
    ];
    app.set_overlay(Overlay::Jira(Box::new(dialog)));
    app
}

#[test]
fn jira_browse_navigation_filter_marks_and_exports() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = seed_jira(temp.path());
    press(&mut app, KeyCode::Down, KeyModifiers::NONE);
    press(&mut app, KeyCode::Char(' '), KeyModifiers::NONE);
    press(&mut app, KeyCode::F(5), KeyModifiers::SHIFT);
    press(&mut app, KeyCode::F(6), KeyModifiers::NONE);
    press(&mut app, KeyCode::F(7), KeyModifiers::NONE);
    press(&mut app, KeyCode::Char('s'), KeyModifiers::CONTROL);
    press(&mut app, KeyCode::Char('r'), KeyModifiers::CONTROL);
    press(&mut app, KeyCode::Tab, KeyModifiers::NONE);
    press(&mut app, KeyCode::Char('b'), KeyModifiers::NONE);
    press(&mut app, KeyCode::Backspace, KeyModifiers::NONE);
    press(&mut app, KeyCode::Char('z'), KeyModifiers::NONE);
    press(&mut app, KeyCode::Esc, KeyModifiers::NONE);
    let Overlay::Jira(d) = &app.overlay else {
        panic!("jira")
    };
    assert_eq!(d.filter, "");
    assert!(matches!(d.queued, Some((_, JiraRequest::Fetch { .. }))));
    press(&mut app, KeyCode::Esc, KeyModifiers::NONE);
    assert!(matches!(app.overlay, Overlay::None));
}

#[test]
fn jira_detail_jql_edit_and_config_editor_transitions() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = seed_jira(temp.path());
    press(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    press(&mut app, KeyCode::Tab, KeyModifiers::NONE);
    press(&mut app, KeyCode::Down, KeyModifiers::NONE);
    press(&mut app, KeyCode::PageDown, KeyModifiers::NONE);
    press(&mut app, KeyCode::Up, KeyModifiers::NONE);
    press(&mut app, KeyCode::PageUp, KeyModifiers::NONE);
    press(&mut app, KeyCode::Home, KeyModifiers::NONE);
    press(&mut app, KeyCode::End, KeyModifiers::NONE);
    press(&mut app, KeyCode::F(3), KeyModifiers::NONE);
    press(&mut app, KeyCode::F(2), KeyModifiers::NONE);
    press(&mut app, KeyCode::Esc, KeyModifiers::NONE);
    press(&mut app, KeyCode::Char('j'), KeyModifiers::CONTROL);
    press(&mut app, KeyCode::Char(' '), KeyModifiers::NONE);
    press(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    let Overlay::Jira(d) = &app.overlay else {
        panic!("jira")
    };
    assert!(matches!(d.mode, JiraMode::Browse));
    let _home =
        crate::test_support::EnvVarGuard::set(crate::test_support::CONFIG_HOME, temp.path());
    press(&mut app, KeyCode::F(4), KeyModifiers::NONE);
    assert!(matches!(app.overlay, Overlay::Editor(_)));
}

#[test]
fn jira_handlers_ignore_non_jira_overlay_and_non_jql_editor_pick() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = app_in(temp.path());
    super::super::jira::process(&mut app, KeyCode::Esc, KeyModifiers::NONE);
    super::super::jira::browse_nav_or_filter(&mut app, KeyCode::Tab, KeyModifiers::NONE);
    let mut none_overlay = Overlay::None;
    assert!(super::super::jira::pick_jql(&mut none_overlay).is_none());
    let dialog = JiraDialog::new_setup(String::new(), "jql".into(), temp.path().to_path_buf());
    let mut browse_overlay = Overlay::Jira(Box::new(dialog));
    assert!(super::super::jira::pick_jql(&mut browse_overlay).is_none());
}
