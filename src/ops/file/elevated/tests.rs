//! Purpose: The administrator helper's protocol and work, without Windows:
//! requests and replies round-trip, the helper redoes a delete or a copy and
//! stops where FFM would, and FFM reports what it did.

use std::cell::RefCell;
use std::io::{self, Cursor, Write};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use tempfile::TempDir;

use super::protocol::{self, Failure, Reply};
use super::*;
use crate::models::{Denial, Panel};

/// A pipe end that keeps what was written to it.
#[derive(Clone, Default)]
struct Written(Rc<RefCell<Vec<u8>>>);

impl Write for Written {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Written {
    fn replies(&self) -> Vec<Reply> {
        let bytes = self.0.borrow().clone();
        let mut input = Cursor::new(bytes);
        std::iter::from_fn(|| protocol::receive::<Reply>(&mut input).expect("a reply")).collect()
    }
}

fn delete_of(sources: Vec<PathBuf>, base: &Path) -> ElevatedRetry {
    ElevatedRetry {
        operation: Operation::Delete,
        selected: sources.len(),
        sources,
        base: base.to_path_buf(),
        settings: None,
    }
}

fn failure_at(path: &Path, denial: Option<Denial>) -> Failure {
    Failure {
        path: path.to_path_buf(),
        error: "denied".to_owned(),
        denial,
    }
}

/// An app on `dir` in both panels.
fn app_in(dir: &Path) -> App {
    App::with_panels(
        Panel::new(dir.to_path_buf()).unwrap(),
        Panel::new(dir.to_path_buf()).unwrap(),
    )
}

fn request_lines(requests: &[ElevatedRetry]) -> Cursor<Vec<u8>> {
    let mut lines = Vec::new();
    for request in requests {
        protocol::send(&mut lines, request).expect("encoded");
    }
    Cursor::new(lines)
}

mod reports;
mod round_trip;
#[cfg(not(windows))]
mod unsupported;

#[test]
fn retry_as_administrator_does_nothing_without_a_retry() {
    let dir = TempDir::new().unwrap();
    let mut app = app_in(dir.path());
    assert!(!session_active());
    retry_as_administrator(&mut app);
    assert!(matches!(app.overlay, Overlay::None));

    let failed = FailedEntry::new("/srv/x/a.txt", &"Permission denied");
    let failure =
        OperationFailed::new(Operation::Delete, failed, 0, 0).because(Some(Denial::Rights));
    app.set_overlay(Overlay::OperationFailed(Box::new(failure)));
    retry_as_administrator(&mut app);
    assert!(matches!(app.overlay, Overlay::OperationFailed(_)));
}
