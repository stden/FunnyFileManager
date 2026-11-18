//! Verify completion and concurrent requests without starting a disk utility.

use super::{MaintenanceJobs, Redraw};
use crate::models::{App, MaintenanceCommand};
use crate::models::{GitSyncTarget, Panel, VolumeOperation, VolumeTarget};
use crate::status::Status;
use std::io;
use std::{path::PathBuf, sync::mpsc, time::Duration};

fn app() -> App {
    let panel = Panel::from_entries(PathBuf::new(), Vec::new());
    App::with_panels(panel.clone(), panel)
}

#[test]
fn completed_git_operation_keeps_its_report_after_navigation() {
    let mut app = app();
    let target = GitSyncTarget {
        root: "repo".into(),
        branch: "feature".into(),
    };
    app.maintenance_request = Some(MaintenanceCommand::SyncBranch(target));
    let mut jobs = MaintenanceJobs::default();
    jobs.pump_with(&mut app, |request| {
        assert!(
            matches!(request, MaintenanceCommand::SyncBranch(target) if target.branch == "feature")
        );
        Ok("Rebased onto origin/main".to_owned())
    });
    app.left_panel.path = "another-repository".into();
    finish(&mut jobs, &mut app);
    assert!(
        matches!(&app.status, Status::CommandOutput(message) if message.contains("feature") && message.contains("completed"))
    );
    assert!(app
        .console
        .visible(10)
        .iter()
        .any(|line| line.text() == "Rebased onto origin/main"));
}

#[test]
fn one_volume_operation_runs_at_a_time_and_errors_reach_the_console() {
    let mut app = app();
    let target = VolumeTarget {
        mount: PathBuf::from("C:\\"),
        id: "test".into(),
    };
    let mut jobs = MaintenanceJobs::default();
    let (release, wait) = mpsc::channel();
    app.maintenance_request = Some(MaintenanceCommand::Volume(
        VolumeOperation::Trim,
        target.clone(),
    ));
    assert_eq!(jobs.pump_with(&mut app, move |request| {
        assert!(matches!(request, MaintenanceCommand::Volume(VolumeOperation::Trim, volume) if volume.id == "test"));
        wait.recv_timeout(Duration::from_secs(10)).unwrap();
        Err(io::Error::other("Access denied"))
    }), Redraw::Needed);
    app.maintenance_request = Some(MaintenanceCommand::Volume(
        VolumeOperation::Defragment,
        target,
    ));
    jobs.pump_with(&mut app, |_| panic!("second job started"));
    assert!(matches!(&app.status, Status::Error(message) if message.contains("already running")));
    release.send(()).unwrap();
    finish(&mut jobs, &mut app);
    assert!(matches!(&app.status, Status::Error(message) if message.contains("Access denied")));
    assert!(app
        .console
        .visible(10)
        .iter()
        .any(|line| line.text().contains("Access denied")));
}

fn finish(jobs: &mut MaintenanceJobs, app: &mut App) {
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while jobs.pending.is_some() {
        assert!(std::time::Instant::now() < deadline);
        jobs.pump_with(app, |_| panic!("no request"));
        std::thread::sleep(Duration::from_millis(1));
    }
}
