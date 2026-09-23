//! Unit tests: a FIFO or a socket under the cursor is refused, never opened.
//!
//! `open` on a FIFO waits in the kernel until a writer turns up, and a transfer
//! runs on the event-loop thread. F5 on a named pipe used to freeze the whole
//! file manager with no key able to reach it — and, once a writer did come,
//! leave an empty regular file behind, reported as "Copied".

use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use super::tests_transfer_policies::{app_between, make_fifo, mark, settings};
use super::*;
use crate::copy::Purpose;

use tempfile::TempDir;

/// Far longer than refusing one entry takes; short enough that a transfer
/// stuck in `open` fails the test instead of hanging the suite.
const STUCK_AFTER: Duration = Duration::from_secs(5);

/// Run `transfer` on its own thread and return what it returned, or `None`
/// when it was still running after [`STUCK_AFTER`].
///
/// A stuck transfer is sitting in `open` on `fifo`. Opening the other end
/// pairs with it and lets it read end-of-file, so the thread still finishes
/// and the test reports the hang instead of hanging on the join.
fn unless_stuck<T: Send + 'static>(
    fifo: &Path,
    transfer: impl FnOnce() -> T + Send + 'static,
) -> Option<T> {
    let (sender, receiver) = mpsc::channel();
    let worker = thread::spawn(move || {
        // The receiver is gone only once the test has already failed.
        let _ = sender.send(transfer());
    });
    let outcome = receiver.recv_timeout(STUCK_AFTER).ok();
    while !worker.is_finished() {
        release_one_reader(fifo);
    }
    worker.join().expect("the transfer thread");
    outcome
}

/// Open `fifo` for writing and close it at once, which ends the wait of a
/// reader stuck in `open`. Non-blocking: with no reader it fails, not waits.
fn release_one_reader(fifo: &Path) {
    use std::os::unix::fs::OpenOptionsExt;

    // ENXIO just means no reader is waiting at this instant; try again.
    drop(
        fs::OpenOptions::new()
            .write(true)
            .custom_flags(libc::O_NONBLOCK)
            .open(fifo),
    );
    thread::sleep(Duration::from_millis(10));
}

/// Press F5 (or F6) on `name` in `from` and accept the dialog aimed at `to`;
/// the status line it ends with.
fn transfer_status(from: &Path, to: &Path, name: &str, purpose: Purpose) -> String {
    let mut app = app_between(from, to);
    mark(&mut app, name);
    let mut dialog = settings(to);
    dialog.purpose = purpose;
    copy_with(&mut app, &dialog);
    app.status.to_string()
}

#[test]
fn f5_on_a_fifo_is_refused_without_waiting_for_a_writer() {
    let (from, to) = (
        TempDir::new().expect("setup"),
        TempDir::new().expect("setup"),
    );
    let Some(fifo) = make_fifo(&from.path().join("pipe")) else {
        return;
    };
    let (source_dir, target) = (from.path().to_path_buf(), to.path().to_path_buf());

    let status = unless_stuck(&fifo, move || {
        transfer_status(&source_dir, &target, "pipe", Purpose::Copy)
    })
    .expect("F5 sat in open() waiting for a writer on the FIFO");

    assert!(status.contains("not a regular file"), "{status}");
    assert!(
        fs::symlink_metadata(to.path().join("pipe")).is_err(),
        "nothing is left at the destination"
    );
}

/// The quick copy and the F6 fallback across filesystems share the one copy
/// routine, so they refuse it the same way — and the move keeps the pipe.
#[test]
fn the_quick_copy_and_the_move_fallback_refuse_a_fifo_too() {
    let _guard = RENAME_TEST_MUTEX
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let (from, to) = (
        TempDir::new().expect("setup"),
        TempDir::new().expect("setup"),
    );
    let Some(fifo) = make_fifo(&from.path().join("pipe")) else {
        return;
    };
    let (source_dir, target) = (from.path().to_path_buf(), to.path().to_path_buf());

    let statuses = unless_stuck(&fifo, move || {
        let mut app = app_between(&source_dir, &target);
        mark(&mut app, "pipe");
        handle_copy(&mut app).expect("refresh");
        let quick = app.status.to_string();
        FORCE_RENAME_FAILURE.store(true, Ordering::SeqCst);
        let moved = transfer_status(&source_dir, &target, "pipe", Purpose::Move);
        FORCE_RENAME_FAILURE.store(false, Ordering::SeqCst);
        [quick, moved]
    })
    .expect("a transfer sat in open() waiting for a writer on the FIFO");

    for status in &statuses {
        assert!(status.contains("not a regular file"), "{status}");
    }
    assert!(fifo.exists(), "the move did not remove the pipe");
    assert!(fs::read_dir(to.path()).expect("list").next().is_none());
}

/// A socket cannot be opened at all; it is refused for what it is instead of
/// failing with "No such device or address".
#[test]
fn f5_on_a_socket_says_it_is_not_a_regular_file() {
    let (from, to) = (
        TempDir::new().expect("setup"),
        TempDir::new().expect("setup"),
    );
    let _listener = std::os::unix::net::UnixListener::bind(from.path().join("control.sock"))
        .expect("test setup");

    let status = transfer_status(from.path(), to.path(), "control.sock", Purpose::Copy);

    assert!(status.contains("not a regular file"), "{status}");
    assert!(fs::symlink_metadata(to.path().join("control.sock")).is_err());
}
