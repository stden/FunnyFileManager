//! Purpose: surface terminal size and clear failures from `LoopState::prepare_frame`
//! and `repaint_after_progress` on every platform and feature configuration.

use super::{repaint_after_progress, LoopState};
use crate::models::{App, Panel};
use crate::progress::{Operation, Painter, Snapshot};
use crate::wsl::SystemWslBackend;
use crate::WritableTestBackend;
use ratatui::backend::{Backend, ClearType, WindowSize};
use ratatui::layout::{Position, Size};
use ratatui::Terminal;
use std::io::{self, Write};
use std::sync::Arc;
use std::time::Duration;

struct FailBackend {
    inner: WritableTestBackend,
    fail_size: bool,
    fail_clear: bool,
}

impl FailBackend {
    fn new(width: u16, height: u16) -> Self {
        Self {
            inner: WritableTestBackend::new(width, height),
            fail_size: false,
            fail_clear: false,
        }
    }
}

impl Write for FailBackend {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Write::flush(&mut self.inner)
    }
}

impl Backend for FailBackend {
    type Error = io::Error;

    fn draw<'a, I>(&mut self, content: I) -> io::Result<()>
    where
        I: Iterator<Item = (u16, u16, &'a ratatui::buffer::Cell)>,
    {
        self.inner.draw(content)
    }

    fn hide_cursor(&mut self) -> io::Result<()> {
        self.inner.hide_cursor()
    }

    fn show_cursor(&mut self) -> io::Result<()> {
        self.inner.show_cursor()
    }

    fn get_cursor_position(&mut self) -> io::Result<Position> {
        self.inner.get_cursor_position()
    }

    fn set_cursor_position<P: Into<Position>>(&mut self, pos: P) -> io::Result<()> {
        self.inner.set_cursor_position(pos)
    }

    fn clear(&mut self) -> io::Result<()> {
        if self.fail_clear {
            return Err(io::Error::other("clear boom"));
        }
        self.inner.clear()
    }

    fn clear_region(&mut self, clear_type: ClearType) -> io::Result<()> {
        if self.fail_clear {
            return Err(io::Error::other("clear boom"));
        }
        self.inner.clear_region(clear_type)
    }

    fn size(&self) -> io::Result<Size> {
        if self.fail_size {
            return Err(io::Error::other("size boom"));
        }
        self.inner.size()
    }

    fn window_size(&mut self) -> io::Result<WindowSize> {
        self.inner.window_size()
    }

    fn flush(&mut self) -> io::Result<()> {
        Backend::flush(&mut self.inner)
    }
}

struct Nowhere;

impl Painter for Nowhere {
    fn paint(&mut self, _: &Snapshot) {}
}

fn app_in_temp(dir: &tempfile::TempDir) -> App {
    let left = Panel::new(dir.path().to_path_buf()).expect("panel");
    App::with_panels(left.clone(), left)
}

#[test]
fn prepare_frame_surfaces_terminal_size_failure() {
    let dir = tempfile::TempDir::new().expect("temp");
    let mut app = app_in_temp(&dir);
    let mut terminal = Terminal::new(FailBackend::new(80, 24)).expect("terminal");
    terminal.backend_mut().fail_size = true;
    let mut state = LoopState::new(Arc::new(SystemWslBackend));
    let err = state
        .prepare_frame(&mut terminal, &mut app)
        .expect_err("size error");
    assert!(err.to_string().contains("size boom"));
}

#[test]
fn prepare_frame_surfaces_terminal_clear_failure_after_resize() {
    let dir = tempfile::TempDir::new().expect("temp");
    let mut app = app_in_temp(&dir);
    let mut terminal = Terminal::new(FailBackend::new(80, 24)).expect("terminal");
    let mut state = LoopState::new(Arc::new(SystemWslBackend));
    state
        .prepare_frame(&mut terminal, &mut app)
        .expect("initial frame");
    terminal.backend_mut().inner = WritableTestBackend::new(40, 10);
    terminal.backend_mut().fail_clear = true;
    let err = state
        .prepare_frame(&mut terminal, &mut app)
        .expect_err("clear error");
    assert!(err.to_string().contains("clear boom"));
}

#[test]
fn repaint_after_progress_surfaces_backend_clear_failure() {
    let mut terminal = Terminal::new(FailBackend::new(80, 24)).expect("terminal");
    let _installed = crate::progress::install_with(Box::new(Nowhere), Duration::ZERO);
    let run = crate::progress::begin(Operation::Copy, &[]);
    crate::progress::entering(std::path::Path::new("big.img"));
    drop(run);
    terminal.backend_mut().fail_clear = true;
    let err = repaint_after_progress(&mut terminal).expect_err("clear error");
    assert!(err.to_string().contains("clear boom"));
}
