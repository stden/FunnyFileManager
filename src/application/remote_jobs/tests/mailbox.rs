//! A worker mailbox handles silence, disconnection, and stale replies without network.

use super::*;
use crate::status::Status;

fn running_request(app: &mut App) -> RemoteRequest {
    assert!(crate::ops::remote::actions::handle(app, Action::Edit));
    app.remote_requests.take().expect("queued request")
}

#[test]
fn empty_channel_keeps_worker_pending_until_it_disconnects() {
    let dir = tempfile::TempDir::new().expect("temporary directory");
    let (mut app, _link) = fixture(&dir);
    let request = running_request(&mut app);
    let (sender, receiver) = mpsc::channel();
    let mut jobs = RemoteJobs {
        pending: Some(Pending {
            id: request.id,
            receiver,
        }),
    };
    assert!(matches!(jobs.pump(&mut app), Redraw::Unchanged));
    assert!(!app.remote_requests.is_idle());
    drop(sender);
    assert!(matches!(jobs.pump(&mut app), Redraw::Needed));
    assert!(app.remote_requests.is_idle());
    assert!(
        matches!(&app.status, Status::Error(message) if message.contains("worker stopped unexpectedly"))
    );
}

#[test]
fn a_stale_worker_reply_does_not_publish_or_finish_the_current_request() {
    let dir = tempfile::TempDir::new().expect("temporary directory");
    let (mut app, _link) = fixture(&dir);
    let mut request = running_request(&mut app);
    let current_id = request.id;
    request.id += 1;
    let (sender, receiver) = mpsc::channel();
    sender
        .send((request, Ok(RemoteOutcome::Done)))
        .expect("reply");
    let mut jobs = RemoteJobs {
        pending: Some(Pending {
            id: current_id,
            receiver,
        }),
    };
    let before = app.status.clone();
    assert!(matches!(jobs.collect(&mut app), Redraw::Needed));
    assert_eq!(app.status, before);
    assert!(!app.remote_requests.is_idle());
    assert!(app.remote_requests.finish(current_id));
}

#[test]
fn a_request_already_running_does_not_start_a_second_worker() {
    let dir = tempfile::TempDir::new().expect("temporary directory");
    let (mut app, _link) = fixture(&dir);
    let request = running_request(&mut app);
    let mut jobs = RemoteJobs::default();
    assert!(matches!(jobs.pump(&mut app), Redraw::Unchanged));
    assert!(jobs.pending.is_none());
    assert!(app.remote_requests.finish(request.id));
}
