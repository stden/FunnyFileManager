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

#[test]
fn connect_network_drives_job_reloads_local_panels_on_completion() {
    use crate::models::{MountPoint, NetworkAddress, NetworkDrive, NetworkProtocol};

    let temp = tempfile::TempDir::new().expect("temp dir");
    let mut right = Panel::new(temp.path().to_path_buf()).expect("panel");
    right.path = temp.path().join("missing-dir");
    let mut app = App::with_panels(Panel::new(temp.path().to_path_buf()).expect("panel"), right);
    std::fs::write(temp.path().join("fresh.txt"), b"ok").expect("write");
    let drive = NetworkDrive {
        path: MountPoint::try_from("Z:\\".to_owned()).expect("mount"),
        remote: NetworkAddress::try_from("\\\\example.test\\share".to_owned()).expect("remote"),
        protocol: NetworkProtocol::Smb,
    };
    let cmd = MaintenanceCommand::ConnectNetworkDrives(vec![drive]);
    assert_eq!(cmd.label(), "Connect 1 network drive(s)");
    app.maintenance_request = Some(cmd);

    let mut jobs = MaintenanceJobs::default();
    jobs.pump_with(&mut app, |_| Ok("Network drives: 1 connected".to_owned()));
    finish(&mut jobs, &mut app);
    assert!(app.left_panel.entries.iter().any(|e| e.name == "fresh.txt"));

    app.maintenance_request = Some(MaintenanceCommand::ConnectNetworkDrives(Vec::new()));
    assert_eq!(jobs.pump(&mut app), Redraw::Needed);
    finish(&mut jobs, &mut app);
}

fn finish(jobs: &mut MaintenanceJobs, app: &mut App) {
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while jobs.pending.is_some() {
        assert!(std::time::Instant::now() < deadline);
        jobs.pump_with(app, |_| panic!("no request"));
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn volume_and_git_sync_commands_report_failures_through_pump() {
    let temp = tempfile::TempDir::new().expect("temp dir");
    let mut app = app();
    let mut jobs = MaintenanceJobs::default();

    let volume = VolumeTarget {
        mount: temp.path().join("missing-vol"),
        id: "none".into(),
    };
    app.maintenance_request = Some(MaintenanceCommand::Volume(VolumeOperation::Trim, volume));
    assert_eq!(jobs.pump(&mut app), Redraw::Needed);
    finish(&mut jobs, &mut app);
    assert!(matches!(app.status, Status::Error(_)));

    let sync = GitSyncTarget {
        root: temp.path().join("missing-repo"),
        branch: "main".to_owned(),
    };
    app.maintenance_request = Some(MaintenanceCommand::SyncBranch(sync));
    assert_eq!(jobs.pump(&mut app), Redraw::Needed);
    finish(&mut jobs, &mut app);
    assert!(matches!(app.status, Status::Error(_)));
}

#[test]
fn spawn_failure_and_panicking_worker_report_errors() {
    let mut app = app();
    let mut jobs = MaintenanceJobs::default();
    let target = GitSyncTarget {
        root: "repo".into(),
        branch: "main".into(),
    };
    app.maintenance_request = Some(MaintenanceCommand::SyncBranch(target.clone()));
    let huge = std::thread::Builder::new().stack_size(1 << (usize::BITS - 2));
    assert_eq!(
        jobs.pump_with_builder(&mut app, huge, |_| Ok(String::new())),
        Redraw::Needed
    );
    assert!(jobs.pending.is_none());
    assert!(matches!(app.status, Status::Error(_)));

    app.maintenance_request = Some(MaintenanceCommand::SyncBranch(target));
    jobs.pump_with(&mut app, |_| panic!("maintenance panic"));
    finish(&mut jobs, &mut app);
    assert!(
        matches!(&app.status, Status::Error(msg) if msg.contains("maintenance worker stopped"))
    );
}
