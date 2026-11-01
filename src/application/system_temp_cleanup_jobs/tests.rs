//! Purpose: Worker completion remains useful after closing and stale scans do not repaint.
use super::*;
use crate::models::{Panel, TempCleanupDialog};
use std::{fs, time::Duration};
fn fixture() -> (tempfile::TempDir, App, TempPlan) {
    let temp = tempfile::TempDir::new().unwrap();
    let root = temp.path().join("temp");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("old"), b"old").unwrap();
    let now = SystemTime::now();
    fs::File::options()
        .write(true)
        .open(root.join("old"))
        .unwrap()
        .set_modified(now - Duration::from_hours(192))
        .unwrap();
    let source = TempSource::Directory {
        path: root.clone(),
        home: None,
    };
    let plan = crate::ops::system_temp_cleanup::scan(&source, now).unwrap();
    let panel = Panel::new(root).unwrap();
    let mut app = App::with_panels(panel.clone(), panel);
    app.live.temp_cleanup = source;
    (temp, app, plan)
}
fn pending(generation: Arc<()>) -> (TempCleanupJobs, mpsc::Sender<Result<Answer, TempError>>) {
    let (sender, receiver) = mpsc::channel();
    (
        TempCleanupJobs {
            pending: Some(Pending {
                generation,
                receiver,
            }),
        },
        sender,
    )
}
#[test]
fn discarded_closed_scan_never_reopens_or_requests_an_idle_redraw() {
    let (_temp, mut app, plan) = fixture();
    let (mut jobs, sender) = pending(Arc::new(()));
    sender.send(Ok(Answer::Scanned(plan))).unwrap();
    assert_eq!(jobs.pump(&mut app), Redraw::Unchanged);
    assert_eq!(jobs.pump(&mut app), Redraw::Unchanged);
    assert!(matches!(app.overlay, Overlay::None));
}
#[test]
fn previous_generation_scan_cannot_populate_a_new_dialog() {
    let (_temp, mut app, plan) = fixture();
    let (mut jobs, sender) = pending(Arc::new(()));
    crate::ops::system_temp_cleanup::open(&mut app);
    sender.send(Ok(Answer::Scanned(plan))).unwrap();
    assert_eq!(jobs.collect(&mut app), Redraw::Unchanged);
    let Overlay::TempCleanup(dialog) = &app.overlay else {
        panic!("new dialog");
    };
    assert!(matches!(dialog.state, TempCleanupState::QueuedScan(_)));
}
#[test]
fn closed_deletion_publishes_its_summary_and_refreshes_both_file_panels() {
    let (_temp, mut app, plan) = fixture();
    let (mut jobs, sender) = pending(Arc::new(()));
    let outcome = crate::ops::system_temp_cleanup::remove(&plan);
    sender
        .send(Ok(Answer::Deleted {
            outcome,
            root: plan.root,
        }))
        .unwrap();
    assert_eq!(jobs.pump(&mut app), Redraw::Needed);
    assert!(app
        .status_text()
        .contains("Removed 3 B in 1 temporary files"));
    for panel in [&app.left_panel, &app.right_panel] {
        assert!(!panel.entries.iter().any(|entry| entry.name == "old"));
    }
    assert!(matches!(app.overlay, Overlay::None));
    assert_eq!(jobs.pump(&mut app), Redraw::Unchanged);
}
#[test]
fn disconnected_worker_shows_failure_once_without_polling_forever() {
    let (_temp, mut app, _plan) = fixture();
    let dialog = TempCleanupDialog::new(TempSource::Unavailable);
    let (mut jobs, sender) = pending(Arc::clone(&dialog.generation));
    app.set_overlay(Overlay::TempCleanup(Box::new(dialog)));
    drop(sender);
    assert_eq!(jobs.collect(&mut app), Redraw::Needed);
    let Overlay::TempCleanup(dialog) = &app.overlay else {
        panic!("failed dialog");
    };
    assert!(matches!(
        dialog.state,
        TempCleanupState::Failed(TempError::Worker)
    ));
    assert_eq!(jobs.pump(&mut app), Redraw::Unchanged);
}
