//! Unit tests: a progress dialog painted over the last frame is wiped by a
//! full repaint once the operation is over, and nothing else is.

use std::time::Duration;

use ratatui::backend::{ClearType, WindowSize};
use ratatui::buffer::Cell;
use ratatui::layout::{Position, Size};

use super::*;
use crate::application::loop_state::{draw_frame, repaint_after_progress};
use crate::models::Panel;
use crate::progress::{Painter, Snapshot};

struct Nowhere;

impl Painter for Nowhere {
    fn paint(&mut self, _snapshot: &Snapshot) {}
}

fn drawn() -> (tempfile::TempDir, App, Terminal<WritableTestBackend>) {
    let directory = tempfile::TempDir::new().unwrap();
    let mut app = App::with_panels(
        Panel::new(directory.path().to_path_buf()).unwrap(),
        Panel::new(directory.path().to_path_buf()).unwrap(),
    );
    let mut terminal = Terminal::new(WritableTestBackend::new(80, 24)).unwrap();
    draw_frame(&mut terminal, &mut app).unwrap();
    (directory, app, terminal)
}

#[test]
fn a_painted_dialog_clears_the_screen_for_a_full_frame() {
    let (_directory, _app, mut terminal) = drawn();
    let _installed = crate::progress::install_with(Box::new(Nowhere), Duration::ZERO);
    let run = crate::progress::begin(crate::progress::Operation::Delete, &[]);
    crate::progress::entering(std::path::Path::new("x"));
    drop(run);

    repaint_after_progress(&mut terminal).unwrap();

    assert_eq!(
        buffer_text(&terminal).trim().len(),
        0,
        "the screen was cleared"
    );
}

#[test]
fn without_a_dialog_the_frame_stays() {
    let (_directory, _app, mut terminal) = drawn();
    repaint_after_progress(&mut terminal).unwrap();
    assert_ne!(buffer_text(&terminal).trim().len(), 0);
}

/// A host that never answers `CSI 6 n`. `Terminal::clear` asks that and waits.
struct BlindCursor(WritableTestBackend);

impl BlindCursor {
    fn new() -> Self {
        Self(WritableTestBackend::new(80, 24))
    }

    fn text(&self) -> String {
        crate::test_support::buffer_string(self.0.buffer())
    }
}

impl Write for BlindCursor {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Write::flush(&mut self.0)
    }
}

impl Backend for BlindCursor {
    type Error = io::Error;

    fn draw<'a, I>(&mut self, content: I) -> io::Result<()>
    where
        I: Iterator<Item = (u16, u16, &'a Cell)>,
    {
        self.0.draw(content)
    }

    fn hide_cursor(&mut self) -> io::Result<()> {
        self.0.hide_cursor()
    }

    fn show_cursor(&mut self) -> io::Result<()> {
        self.0.show_cursor()
    }

    fn get_cursor_position(&mut self) -> io::Result<Position> {
        Err(io::Error::other(
            "The cursor position could not be read within a normal duration",
        ))
    }

    fn set_cursor_position<P: Into<Position>>(&mut self, position: P) -> io::Result<()> {
        self.0.set_cursor_position(position)
    }

    fn clear(&mut self) -> io::Result<()> {
        self.0.clear()
    }

    fn clear_region(&mut self, clear_type: ClearType) -> io::Result<()> {
        self.0.clear_region(clear_type)
    }

    fn size(&self) -> io::Result<Size> {
        self.0.size()
    }

    fn window_size(&mut self) -> io::Result<WindowSize> {
        self.0.window_size()
    }

    fn flush(&mut self) -> io::Result<()> {
        Backend::flush(&mut self.0)
    }
}

#[test]
fn a_repaint_after_progress_does_not_ask_where_the_cursor_is() {
    let directory = tempfile::TempDir::new().unwrap();
    let mut app = App::with_panels(
        Panel::new(directory.path().to_path_buf()).unwrap(),
        Panel::new(directory.path().to_path_buf()).unwrap(),
    );
    let mut terminal = Terminal::new(BlindCursor::new()).unwrap();
    draw_frame(&mut terminal, &mut app).unwrap();
    let _installed = crate::progress::install_with(Box::new(Nowhere), Duration::ZERO);
    let run = crate::progress::begin(crate::progress::Operation::Copy, &[]);
    crate::progress::entering(std::path::Path::new("big.img"));
    drop(run);

    repaint_after_progress(&mut terminal).unwrap();
    assert_eq!(terminal.backend().text().trim().len(), 0);

    draw_frame(&mut terminal, &mut app).unwrap();
    assert_ne!(terminal.backend().text().trim().len(), 0);
}
