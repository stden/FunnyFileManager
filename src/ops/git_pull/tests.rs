//! Purpose: Keep menu execution confined to the captured active folder.

use super::*;
use crate::models::Panel;

fn app(path: &Path) -> App {
    let panel = Panel::new(path.to_owned()).expect("panel");
    App::with_panels(panel.clone(), panel)
}

#[test]
fn an_idle_pending_check_starts_no_operation() {
    let temp = tempfile::TempDir::new().expect("temporary directory");
    let mut app = app(temp.path());
    run_pending(&mut app);
    assert!(app.git_pull_root.is_none());
    assert!(matches!(app.status, Status::KeyHints));
}

#[test]
fn menu_selection_captures_the_active_directory_and_runs_once() {
    let temp = tempfile::TempDir::new().expect("temporary directory");
    let mut app = app(temp.path());
    crate::ops::menu::execute(&mut app, crate::menu::MenuCommand::GitPullAll);
    assert_eq!(app.git_pull_root.as_deref(), Some(temp.path()));
    run_with(&mut app, |path, _, _| {
        assert_eq!(path, temp.path());
        Ok(run::Summary {
            updated: 2,
            ..Default::default()
        })
    });
    assert!(app.git_pull_root.is_none());
    assert!(app.status_text().contains("2 updated"));
    run_with(&mut app, |_, _, _| panic!("nothing is queued"));
}

#[test]
fn failures_and_cancellation_leave_the_application_usable() {
    let temp = tempfile::TempDir::new().expect("temporary directory");
    let mut app = app(temp.path());
    for kind in [io::ErrorKind::Other, io::ErrorKind::Interrupted] {
        queue(&mut app).expect("queue");
        run_with(&mut app, |_, _, _| {
            Err(io::Error::new(kind, "fixture outcome"))
        });
        assert!(app.lifecycle.is_running());
        assert!(app.git_pull_root.is_none());
    }
    assert!(app.status_text().contains("cancelled"));
    queue(&mut app).expect("queue");
    run_with(&mut app, |_, _, _| {
        Ok(run::Summary {
            failed: 1,
            ..Default::default()
        })
    });
    assert!(matches!(app.status, Status::Error(_)));
}

#[test]
fn a_deleted_panel_directory_cannot_queue_a_pull() {
    let temp = tempfile::TempDir::new().expect("temporary directory");
    let mut app = app(temp.path());
    temp.close().expect("remove temporary directory");
    assert!(queue(&mut app).is_err());
    assert!(app.git_pull_root.is_none());
}
