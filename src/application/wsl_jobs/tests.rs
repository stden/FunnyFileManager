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
