//! Purpose: The helper's side of the pipe: a progress frame it paints is a reply
//! FFM can read back, a frame that cannot be sent is dropped, and a copy that
//! fails reports the entry it stopped at.

use std::cell::RefCell;
use std::io::{self, Cursor, Write};
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use tempfile::TempDir;

use super::super::protocol::{self, Reply};
use super::{run, Relay};
use crate::copy::{CopyPrompt, Purpose};
use crate::models::ElevatedRetry;
use crate::progress::{Operation, Painter, Phase, Snapshot, Tally};

/// A pipe end that keeps what was written to it.
#[derive(Clone, Default)]
struct Pipe(Rc<RefCell<Vec<u8>>>);

impl Write for Pipe {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Pipe {
    fn replies(&self) -> Vec<Reply> {
        let mut input = Cursor::new(self.0.borrow().clone());
        std::iter::from_fn(|| protocol::receive::<Reply>(&mut input).expect("a reply")).collect()
    }
}

/// Refuses every write, as a pipe whose FFM side has closed does.
struct Closed;

impl Write for Closed {
    fn write(&mut self, _bytes: &[u8]) -> io::Result<usize> {
        Err(io::Error::from(io::ErrorKind::BrokenPipe))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn progress_of(current: &str) -> Snapshot {
    Snapshot {
        operation: Operation::Delete,
        phase: Phase::Working,
        current: PathBuf::from(current),
        done: Tally { files: 2, bytes: 5 },
        total: Tally { files: 3, bytes: 9 },
        elapsed: Duration::ZERO,
    }
}

#[test]
fn a_painted_frame_reaches_ffm_as_progress() {
    let pipe = Pipe::default();
    let mut relay = Relay(Rc::new(RefCell::new(pipe.clone())));

    relay.paint(&progress_of("/work/a.txt"));

    match pipe.replies().as_slice() {
        [Reply::Progress(snapshot)] => {
            assert_eq!(snapshot.current, PathBuf::from("/work/a.txt"));
            assert_eq!(snapshot.done, Tally { files: 2, bytes: 5 });
        }
        other => panic!("expected one progress frame, got {} replies", other.len()),
    }
}

#[test]
fn a_frame_that_cannot_be_sent_is_dropped_without_failing_the_painter() {
    let mut relay = Relay(Rc::new(RefCell::new(Closed)));

    relay.paint(&progress_of("/work/a.txt"));
}

#[test]
fn a_copy_that_fails_reports_the_entry_it_stopped_at() {
    let root = TempDir::new().expect("test setup");
    let missing = root.path().join("no-such-source.txt");
    let retry = ElevatedRetry {
        operation: Operation::Copy,
        sources: vec![missing.clone()],
        base: root.path().to_path_buf(),
        selected: 1,
        settings: Some(CopyPrompt::for_purpose(
            Purpose::Copy,
            "no-such-source.txt",
            root.path(),
        )),
    };

    let outcome = run(&retry);

    let failed = outcome.failed.expect("the copy stopped at its first entry");
    assert_eq!(failed.path, missing);
    assert_ne!(failed.error, "");
    assert_eq!(outcome.done, 0);
}

#[test]
fn a_failed_delete_reports_what_remains() {
    let root = TempDir::new().expect("test setup");
    let removed = root.path().join("removed.txt");
    let missing = root.path().join("missing.txt");
    let remaining = root.path().join("remaining.txt");
    std::fs::write(&removed, "removed").expect("file");
    std::fs::write(&remaining, "remaining").expect("file");
    let retry = ElevatedRetry {
        operation: Operation::Delete,
        sources: vec![removed.clone(), missing.clone(), remaining.clone()],
        base: root.path().to_path_buf(),
        selected: 3,
        settings: None,
    };
    let outcome = run(&retry);
    assert_eq!(outcome.done, 1);
    assert_eq!(outcome.left, 1);
    assert_eq!(outcome.failed.expect("missing entry").path, missing);
    assert!(!removed.exists());
    assert!(remaining.exists());
}

#[test]
fn a_transfer_without_settings_is_rejected_without_touching_the_files() {
    let root = TempDir::new().expect("test setup");
    let source = root.path().join("source.txt");
    std::fs::write(&source, "untouched").expect("file");
    let retry = ElevatedRetry {
        operation: Operation::Copy,
        sources: vec![source.clone()],
        base: root.path().to_path_buf(),
        selected: 1,
        settings: None,
    };
    let outcome = run(&retry);
    assert_eq!(outcome.left, 1);
    assert_eq!(
        outcome.failed.expect("malformed request").error,
        "malformed request"
    );
    assert_eq!(std::fs::read_to_string(source).expect("file"), "untouched");
}
