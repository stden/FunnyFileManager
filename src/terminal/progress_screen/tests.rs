//! Purpose: The progress painter writes the dialog, and only the dialog, to
//! its terminal.

use std::cell::RefCell;
use std::io;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use super::*;
use crate::progress::{Operation, PanelHeader, Phase, Tally};

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
    let screen = ProgressScreen::stdout();
    let (width, height) = (screen.size)();
    assert!(width > 0 && height > 0);
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

#[test]
fn directory_read_banner_writes_only_on_non_headless_terminal() {
    let out = Written::default();
    let mut screen = ProgressScreen::new(out.clone(), || (120, 30));
    let sample = DirectoryRead {
        files: 629_371,
        header: Some(PanelHeader {
            x: 60,
            y: 1,
            width: 60,
        }),
    };
    screen.paint_directory_read(&sample);
    let written = String::from_utf8_lossy(&out.0.borrow()).into_owned();
    assert!(written.contains("Reading: 629371 files"), "{written}");
    assert!(written.contains("\u{1b}[2;62H"), "{written}");
    assert!(!written.contains("\u{1b}[1;1H"));

    let host = crate::terminal::headless::HeadlessHost::new();
    let _scope = host.enter();
    let silent = Written::default();
    let mut headless_screen = ProgressScreen::new(silent.clone(), || {
        panic!("headless must not query terminal size")
    });
    headless_screen.paint_directory_read(&sample);
    assert_eq!(silent.0.borrow().len(), 0);
}
