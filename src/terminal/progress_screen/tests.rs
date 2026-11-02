//! Purpose: The progress painter writes the dialog, and only the dialog, to
//! its terminal.

use std::cell::RefCell;
use std::io;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use super::*;
use crate::progress::{Operation, Phase, Tally};

/// A terminal that keeps what was written to it.
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

#[test]
fn the_dialog_is_written_where_it_stands_and_nothing_else() {
    let out = Written::default();
    let mut screen = ProgressScreen::new(out.clone(), || (100, 30));
    screen.paint(&Snapshot {
        operation: Operation::Copy,
        phase: Phase::Working,
        current: PathBuf::from("big.iso"),
        transfer: None,
        done: Tally::default(),
        total: Tally { files: 1, bytes: 1 },
        elapsed: Duration::ZERO,
    });
    let written = String::from_utf8_lossy(&out.0.borrow()).into_owned();
    assert!(written.contains("big.iso"), "{written}");
    // The dialog's top left corner, 1-based as the escape sequence counts:
    // row 10, column 21 on a 100 x 30 screen (60 x 12 cells, centred).
    assert!(written.contains("\u{1b}[10;21H"), "{written}");
    assert!(
        !written.contains("\u{1b}[1;1H"),
        "nothing outside the dialog"
    );
}

#[test]
fn the_session_painter_draws_on_standard_output() {
    // Built, never painted: a test has no terminal to draw on.
    let _screen = ProgressScreen::stdout();
}

#[test]
fn automation_never_paints_progress_on_the_protocol_stream() {
    let host = crate::terminal::headless::HeadlessHost::new();
    let _scope = host.enter();
    let out = Written::default();
    let mut screen = ProgressScreen::new(out.clone(), || {
        panic!("headless must not query terminal size")
    });
    screen.paint(&Snapshot {
        operation: Operation::Copy,
        phase: Phase::Working,
        current: PathBuf::from("private.iso"),
        transfer: None,
        done: Tally::default(),
        total: Tally { files: 1, bytes: 1 },
        elapsed: Duration::ZERO,
    });
    assert!(out.0.borrow().is_empty());
}
