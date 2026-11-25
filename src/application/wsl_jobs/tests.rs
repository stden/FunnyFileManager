//! Failed worker handoffs must repaint the error, then become idle again.

use super::*;
use crate::{models::Panel, status::Status};
use tempfile::TempDir;

struct NoExecution;

impl WslBackend for NoExecution {
    fn execute(&self, _: &crate::models::WslOperation) -> Result<WslOutput, String> {
        panic!("a disconnected fixture must not execute WSL")
    }
}

#[test]
fn stopped_workers_report_submission_and_pending_failures_once() {
    for submitted in [false, true] {
        let directory = TempDir::new().expect("temporary directory");
        let mut app = App::new().expect("application");
        app.left_panel = Panel::new(directory.path().to_owned()).expect("panel");
        app.right_panel = Panel::new(directory.path().to_owned()).expect("panel");
        let id = app
            .wsl_requests
            .enqueue(crate::models::WslOperation::Update);
        let pending = if submitted {
            drop(app.wsl_requests.take());
            vec![Pending {
                id,
                action: "Background/WSL/Update",
                started: Instant::now(),
            }]
        } else {
            Vec::new()
        };
        let mut jobs = disconnected(pending);

        assert_eq!(jobs.pump(&mut app), Redraw::Needed);
        assert!(matches!(&app.status, Status::Error(error) if error == "WSL worker stopped"));
        assert_eq!(jobs.pump(&mut app), Redraw::Unchanged);
    }
}

fn disconnected(pending: Vec<Pending>) -> WslJobs {
    let (requests, incoming) = mpsc::channel();
    let (outgoing, results) = mpsc::channel();
    drop(incoming);
    drop(outgoing);
    WslJobs {
        backend: Arc::new(NoExecution),
        worker: Some(Worker {
            requests,
            results,
            pending,
        }),
    }
}

#[test]
fn pending_labels_cover_refresh_update_packages_and_open_operations() {
    use crate::models::{DistributionName, WslOperation};

    let distro = DistributionName::new("Ubuntu").expect("distro");
    let mut app = App::new().expect("app");
    app.wsl_requests.enqueue(WslOperation::Refresh);
    app.wsl_requests
        .enqueue(WslOperation::UpdatePackages(distro.clone()));
    app.wsl_requests.enqueue(WslOperation::Open(distro));
    let actions: Vec<_> = app
        .wsl_requests
        .take()
        .iter()
        .map(|req| Pending::new(req).action)
        .collect();
    assert_eq!(
        actions,
        [
            "Background/WSL/Refresh",
            "Background/WSL/UpdatePackages",
            "Background/WSL/Open",
        ]
    );
}

#[test]
fn worker_spawn_failure_and_unknown_finish_id_are_safe() {
    let mut jobs = WslJobs::new(Arc::new(NoExecution));
    let huge = std::thread::Builder::new().stack_size(1 << (usize::BITS - 2));
    let err = jobs
        .submit_request(
            WslRequest {
                id: WslRequestId::new(1),
                operation: crate::models::WslOperation::Refresh,
            },
            huge,
        )
        .unwrap_err();
    assert!(err.contains("Cannot start WSL worker"));
    assert!(jobs.worker.is_none());

    let directory = TempDir::new().expect("temporary directory");
    let mut app = App::new().expect("application");
    app.left_panel = Panel::new(directory.path().to_owned()).expect("panel");
    app.right_panel = Panel::new(directory.path().to_owned()).expect("panel");
    let mut disconnected = disconnected(Vec::new());
    let worker = disconnected.worker.as_mut().expect("worker");
    worker.finish(&mut app, WslRequestId::new(999));
}
