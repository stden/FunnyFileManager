//! Purpose: Transport requests preserve job boundaries, cancellation and terminal responsiveness.
use super::*;
use crate::automation::options::Profile;

fn queued_session() -> (Session, mpsc::Receiver<Command>) {
    let (sender, receiver) = mpsc::sync_channel(1);
    let shared = Shared::default();
    shared
        .update(json!({"running": true, "background_pending": false}))
        .unwrap();
    (
        Session {
            sender,
            shared,
            host: HeadlessHost::new(),
            stop: Arc::new(AtomicBool::new(false)),
            worker: None,
        },
        receiver,
    )
}

fn options(root: &std::path::Path) -> Options {
    Options {
        root: root.to_owned(),
        size: Size::new(80, 24),
        profile: Profile::Isolated,
    }
}

#[test]
fn queued_commands_are_exclusive_and_disconnected_workers_are_reported() {
    let (session, receiver) = queued_session();
    let first = session.input(vec!["Down".into()], None, None).unwrap();
    assert_eq!(first["job_id"], 1);
    assert_eq!(
        session.input(vec![], None, None).unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );
    let command = receiver.recv().unwrap();
    assert!(matches!(command.operation, Operation::Input(_)));
    session.shared.complete(command.id, None).unwrap();
    drop(receiver);
    assert_eq!(
        session.input(vec![], None, None).unwrap_err().kind(),
        io::ErrorKind::BrokenPipe
    );
    let state = session.state().unwrap();
    assert_eq!(state["busy"], false);
    assert_eq!(state["job"]["error"], "session worker is unavailable");
}

#[test]
fn busy_cancellation_is_immediate_and_wait_is_bounded() {
    let (mut session, receiver) = queued_session();
    session.shared.reserve().unwrap();
    let _scope = session.host.enter();
    assert_eq!(
        session.wait(10_001).unwrap_err().kind(),
        io::ErrorKind::InvalidInput
    );
    assert_eq!(session.wait(20).unwrap()["state"]["busy"], true);
    session.input(vec!["Esc".into()], None, None).unwrap();
    assert_eq!(crate::terminal::headless::cancelled(), Some(true));
    assert!(receiver.try_recv().is_err());
    assert_eq!(session.close().unwrap()["state"]["running"], false);
    assert!(session.stop.load(Ordering::Acquire));
    assert_eq!(session.state().unwrap()["job"]["status"], "finished");
}

#[test]
fn invalid_requests_leave_the_session_idle() {
    let (session, _receiver) = queued_session();
    for (width, height) in [(19, 24), (401, 24), (80, 7), (80, 201)] {
        assert_eq!(
            session.resize(width, height).unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
    }
    assert_eq!(
        session
            .action("missing-context", "help")
            .unwrap_err()
            .kind(),
        io::ErrorKind::InvalidInput
    );
    assert_eq!(
        session
            .action("panels", "missing-action")
            .unwrap_err()
            .kind(),
        io::ErrorKind::InvalidInput
    );
    assert_eq!(session.state().unwrap()["busy"], false);
}

#[test]
fn real_session_resizes_and_closes_without_user_configuration() {
    let root = tempfile::TempDir::new().unwrap();
    let mut session = Session::start(options(root.path())).unwrap();
    session.resize(90, 28).unwrap();
    let state = session.wait(1000).unwrap();
    assert_eq!(state["state"]["screen"]["width"], 90);
    assert_eq!(state["state"]["screen"]["height"], 28);
    assert_eq!(session.close().unwrap()["state"]["running"], false);
    assert!(session.worker.is_none());
    assert_eq!(
        session.input(vec![], None, None).unwrap_err().kind(),
        io::ErrorKind::BrokenPipe
    );
}

#[test]
fn worker_initialization_failure_is_returned_to_the_transport() {
    let root = tempfile::TempDir::new().unwrap();
    let result = Session::start(options(&root.path().join("missing")));
    assert!(result
        .err()
        .unwrap()
        .to_string()
        .contains("cannot initialize the FFM session"));
}

#[test]
fn regular_files_cannot_become_session_roots() {
    let root = tempfile::TempDir::new().unwrap();
    let file = root.path().join("not-a-directory");
    std::fs::write(&file, "file").unwrap();
    let result = Options::parse(&["--root".into(), file.into_os_string()]);
    assert_eq!(result.err().unwrap().kind(), io::ErrorKind::InvalidInput);
}

#[cfg(unix)]
mod external;
