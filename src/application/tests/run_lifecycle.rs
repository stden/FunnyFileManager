//! Event-loop lifecycle: normal exit and terminal failures.
#![allow(unused_imports)]

use super::*;
use crate::models::Lifecycle;

use crate::models::Panel;

use ratatui::backend::{ClearType, WindowSize};
use ratatui::layout::{Position, Size};

use tempfile::TempDir;

/// Message carried by [`FailingDrawBackend`]'s error, so the assertion can
/// prove the loop surfaced *that* failure rather than some other one.
const DRAW_FAILURE: &str = "backend refused to draw";

/// A backend whose `draw` always fails. Everything else delegates to the real
/// test backend, so `Terminal::new` and the resize probe still work and the
/// only failure the loop can see is the one under test.
struct FailingDrawBackend(WritableTestBackend);

impl Write for FailingDrawBackend {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Write::flush(&mut self.0)
    }
}

impl Backend for FailingDrawBackend {
    type Error = io::Error;

    fn draw<'a, I>(&mut self, _content: I) -> io::Result<()>
    where
        I: Iterator<Item = (u16, u16, &'a ratatui::buffer::Cell)>,
    {
        Err(io::Error::new(io::ErrorKind::BrokenPipe, DRAW_FAILURE))
    }

    fn hide_cursor(&mut self) -> io::Result<()> {
        self.0.hide_cursor()
    }

    fn show_cursor(&mut self) -> io::Result<()> {
        self.0.show_cursor()
    }

    fn get_cursor_position(&mut self) -> io::Result<Position> {
        self.0.get_cursor_position()
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

    fn flush(&mut self) -> io::Result<()> {
        Backend::flush(&mut self.0)
    }

    fn window_size(&mut self) -> io::Result<WindowSize> {
        self.0.window_size()
    }
}

/// `run_app` binds the loop to crossterm's real `event::read`, which would
/// block forever. A backend that cannot draw stops it on the first frame — the
/// error can only come from inside the loop, so it proves `run_app` really
/// entered it, without ever reaching the blocking read.
#[test]
fn run_app_enters_the_loop_and_surfaces_a_failed_draw() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp.path().to_path_buf())?;
    app.right_panel = Panel::new(temp.path().to_path_buf())?;

    let mut terminal = Terminal::new(FailingDrawBackend(WritableTestBackend::new(40, 8)))?;

    let error = run_app(&mut terminal, app).expect_err("a failed draw must be fatal");

    assert!(
        error.to_string().contains(DRAW_FAILURE),
        "the backend's message was lost: {error}"
    );
    Ok(())
}

/// An app that already wants to quit must leave the loop before the first
/// draw: nothing is painted and no event is consumed.
#[test]
fn event_loop_exits_before_drawing_when_the_app_already_quit() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp.path().to_path_buf())?;
    app.right_panel = Panel::new(temp.path().to_path_buf())?;
    app.lifecycle = Lifecycle::Quitting;

    let mut terminal = Terminal::new(WritableTestBackend::new(40, 8))?;

    run_app_with_event_reader(&mut terminal, app, || {
        panic!("a quitting app must not read another event")
    })?;

    assert!(
        buffer_text(&terminal).trim().is_empty(),
        "the loop painted a frame it should never have entered"
    );
    Ok(())
}

/// A failed terminal draw is the one fatal error in the loop: it aborts
/// immediately, carrying the backend's message, without reading an event.
#[test]
fn event_loop_aborts_with_the_backend_message_when_the_draw_fails() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp.path().to_path_buf())?;
    app.right_panel = Panel::new(temp.path().to_path_buf())?;

    let mut terminal = Terminal::new(FailingDrawBackend(WritableTestBackend::new(40, 8)))?;

    let error = run_app_with_event_reader(&mut terminal, app, || {
        panic!("a failed draw must abort before the next event is read")
    })
    .expect_err("a failed draw must be fatal");

    assert_eq!(error.kind(), io::ErrorKind::Other);
    assert!(
        error.to_string().contains(DRAW_FAILURE),
        "the backend's message was lost: {error}"
    );
    Ok(())
}

/// Losing the terminal is the end of a session, not a failure of one: the loop
/// leaves the way F10 leaves, so the caller still gets to write the histories.
/// Before this, crossterm's reader trapped the process in a loop that read
/// zero bytes forever and burned a core until something killed it.
#[test]
fn a_lost_terminal_ends_the_loop_cleanly() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp.path().to_path_buf())?;
    app.right_panel = Panel::new(temp.path().to_path_buf())?;

    let mut terminal = Terminal::new(WritableTestBackend::new(40, 8))?;
    let reads = std::cell::Cell::new(0_u32);

    run_loop(
        &mut terminal,
        &mut app,
        || {
            reads.set(reads.get() + 1);
            match reads.get() {
                1 => Ok(Some(Event::Key(key_event(KeyCode::Down)))),
                2 => Err(crate::recording::TerminalLost::error()),
                _ => panic!("the loop must not read past a lost terminal"),
            }
        },
        std::sync::Arc::new(crate::wsl::SystemWslBackend),
    )?;

    assert_eq!(reads.get(), 2, "the loop read on after the hang-up");
    assert_eq!(
        app.lifecycle,
        Lifecycle::Quitting,
        "the session ended, so it must not still look running"
    );
    Ok(())
}

/// Only a lost terminal leaves quietly. Every other read failure still aborts,
/// so the sentinel the replay tests end their scripts with keeps its meaning.
#[test]
fn other_read_errors_are_still_fatal() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp.path().to_path_buf())?;
    app.right_panel = Panel::new(temp.path().to_path_buf())?;

    let mut terminal = Terminal::new(WritableTestBackend::new(40, 8))?;

    let error = run_loop(
        &mut terminal,
        &mut app,
        || Err(io::Error::new(io::ErrorKind::UnexpectedEof, "done")),
        std::sync::Arc::new(crate::wsl::SystemWslBackend),
    )
    .expect_err("a read failure that is not a hang-up must stay fatal");

    assert_eq!(error.kind(), io::ErrorKind::UnexpectedEof);
    assert!(
        app.lifecycle.is_running(),
        "a failure is not a quit: the app must not be marked as having quit"
    );
    Ok(())
}
