//! Purpose: Tests for opening the task list: the live viewer, as a new
//! session of FFM's own process, where a host is installed; the text report
//! where none is.

use std::sync::Arc;
use std::time::Instant;

use tempfile::TempDir;

use super::open;
use crate::models::{App, ControlRefusal, Operation, Panel, Pid, TaskList, TaskSession};
use crate::prompt::Privilege;
use crate::test_support::processes::{one_core, process, FixtureHost};

fn app(root: &TempDir) -> App {
    let panel = || Panel::from_entries(root.path().to_path_buf(), Vec::new());
    App::with_panels(panel(), panel())
}

fn session(app: &mut App) -> TaskSession {
    app.task_list_mut().expect("the viewer is open").session()
}

#[test]
fn with_a_host_the_viewer_opens_as_a_new_session_each_time() {
    let root = TempDir::new().expect("dir");
    let mut app = app(&root);
    app.task_host
        .install(Arc::new(FixtureHost::new(Vec::new())));
    open(&mut app).expect("the viewer opens");
    let first = session(&mut app);
    open(&mut app).expect("the viewer opens again");
    assert_ne!(session(&mut app), first);
    let list = app.task_list_mut().expect("the viewer is open");
    assert_eq!(list.privilege(), Privilege::current());
}

/// The list knows FFM's own PID, so the confirmation refuses it.
#[test]
fn the_viewer_refuses_to_signal_ffm_itself() {
    let root = TempDir::new().expect("dir");
    let mut app = app(&root);
    app.task_host
        .install(Arc::new(FixtureHost::new(Vec::new())));
    open(&mut app).expect("the viewer opens");
    let own = Pid::new(std::process::id()).expect("FFM's PID");
    let list: &mut TaskList = app.task_list_mut().expect("the viewer is open");
    let now = Instant::now();
    list.take_request(now);
    list.apply_sample(Ok(one_core(vec![process(own, "ffm")])), now);
    let frozen = list.freeze_targets(Operation::Signal);
    assert_eq!(frozen.refusal(), Some(ControlRefusal::OwnProcess));
}

#[cfg(unix)]
#[test]
fn without_a_host_the_report_opens_in_the_viewer() {
    let root = TempDir::new().expect("dir");
    let mut app = app(&root);
    open(&mut app).expect("ps runs");
    let crate::models::Overlay::Viewer(viewer) = &app.overlay else {
        panic!("the report is open");
    };
    assert_eq!(viewer.title(), "Task list");
    assert!(viewer
        .lines
        .first()
        .is_some_and(|line| line.contains("PID")));
}
