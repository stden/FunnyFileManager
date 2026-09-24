//! Unit tests: the real [`CrosstermOps`] implementation of [`TerminalOps`], the
//! inertness of the [`NoopTerminalOps`] double every other suite drives, and
//! the terminal the session lends back to its caller.
//!
//! Every operation except `enable_raw` reaches the terminal through the
//! injected backend, so a backend that keeps the bytes written and the
//! [`Backend`] calls made pins each one to the escape sequence it really emits
//! — no pty required.

use super::*;

use ratatui::backend::{ClearType, WindowSize};
use ratatui::layout::{Position, Size};

use crate::WritableTestBackend;

mod tests_escape_sequences;

/// Message carried by [`SpyBackend`]'s error, so a failure assertion can prove
/// the backend's own text survived the conversion into [`io::Error`].
const SPY_FAILURE: &str = "spy backend refused";

/// Backend that keeps every byte written through [`Write`] and the name of
/// every [`Backend`] method called.
///
/// `WritableTestBackend` throws written bytes away, so the escape sequences the
/// `execute!` calls emit need a backend of their own to land in. `size` is left
/// unrecorded: `Terminal::new` probes it before any test starts.
struct SpyBackend {
    inner: WritableTestBackend,
    written: Vec<u8>,
    calls: Vec<&'static str>,
    fail: bool,
}

impl SpyBackend {
    fn record(&mut self, operation: &'static str) -> io::Result<()> {
        self.calls.push(operation);
        if self.fail {
            Err(io::Error::other(SPY_FAILURE))
        } else {
            Ok(())
        }
    }
}

impl Write for SpyBackend {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.written.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Backend for SpyBackend {
    type Error = io::Error;

    fn draw<'a, I>(&mut self, content: I) -> io::Result<()>
    where
        I: Iterator<Item = (u16, u16, &'a ratatui::buffer::Cell)>,
    {
        self.record("draw")?;
        self.inner.draw(content)
    }

    fn hide_cursor(&mut self) -> io::Result<()> {
        self.record("hide_cursor")?;
        self.inner.hide_cursor()
    }

    fn show_cursor(&mut self) -> io::Result<()> {
        self.record("show_cursor")?;
        self.inner.show_cursor()
    }

    fn get_cursor_position(&mut self) -> io::Result<Position> {
        self.record("get_cursor_position")?;
        self.inner.get_cursor_position()
    }

    fn set_cursor_position<P: Into<Position>>(&mut self, position: P) -> io::Result<()> {
        self.record("set_cursor_position")?;
        self.inner.set_cursor_position(position)
    }

    fn clear(&mut self) -> io::Result<()> {
        self.record("clear")?;
        self.inner.clear()
    }

    fn clear_region(&mut self, clear_type: ClearType) -> io::Result<()> {
        self.record("clear_region")?;
        self.inner.clear_region(clear_type)
    }

    fn size(&self) -> io::Result<Size> {
        self.inner.size()
    }

    fn window_size(&mut self) -> io::Result<WindowSize> {
        self.record("window_size")?;
        self.inner.window_size()
    }

    fn flush(&mut self) -> io::Result<()> {
        self.record("flush")?;
        Backend::flush(&mut self.inner)
    }
}

fn spy_terminal() -> Terminal<SpyBackend> {
    Terminal::new(SpyBackend {
        inner: WritableTestBackend::new(20, 5),
        written: Vec::new(),
        calls: Vec::new(),
        fail: false,
    })
    .expect("terminal over the spy backend")
}

/// Bytes the terminal has been handed since the last [`take_written`] call.
fn take_written(terminal: &mut Terminal<SpyBackend>) -> String {
    let bytes = std::mem::take(&mut terminal.backend_mut().written);
    String::from_utf8_lossy(&bytes).into_owned()
}

/// The double the session and suspension suites run on must be inert: if it
/// reached the terminal, every one of those tests would be writing escape
/// sequences into the test runner's own output.
#[test]
fn noop_terminal_ops_touch_neither_the_backend_nor_the_bytes() {
    let mut terminal = spy_terminal();
    let mut ops = NoopTerminalOps;

    TerminalOps::<SpyBackend>::enable_raw(&mut ops).expect("enable raw");
    TerminalOps::<SpyBackend>::disable_raw(&mut ops).expect("disable raw");
    ops.enter_alternate(&mut terminal).expect("enter alternate");
    ops.leave_alternate(&mut terminal).expect("leave alternate");
    ops.enable_mouse(&mut terminal).expect("enable mouse");
    ops.disable_mouse(&mut terminal).expect("disable mouse");
    ops.clear(&mut terminal).expect("clear");
    ops.show_cursor(&mut terminal).expect("show cursor");
    let pushed = ops.enable_keyboard(&mut terminal).expect("enable keyboard");
    let popped = ops
        .disable_keyboard(&mut terminal)
        .expect("disable keyboard");

    assert_eq!(
        (pushed, popped),
        (KeyboardProtocol::Legacy, KeyboardProtocol::Legacy)
    );
    assert!(
        terminal.backend().calls.is_empty(),
        "the no-op ops reached the backend: {:?}",
        terminal.backend().calls
    );
    assert_eq!(take_written(&mut terminal), "");
}

/// `terminal_mut` is how `main_impl` reaches the terminal it lent the session.
/// It must hand back *that* terminal, so a frame drawn through the session
/// lands in the caller's own backend rather than somewhere the caller cannot
/// see.
#[test]
fn session_terminal_mut_hands_back_the_borrowed_terminal() {
    let mut terminal = spy_terminal();
    let mut ops = NoopTerminalOps;

    {
        let mut session =
            TerminalSession::enter(&mut terminal, &mut ops).expect("enter the session");
        session
            .terminal_mut()
            .backend_mut()
            .write_all(b"probe")
            .expect("write through the session");
        session.finish().expect("finish the session");
    }

    assert_eq!(take_written(&mut terminal), "probe");
}

/// Panic-hook cleanup must not panic even when the process has no real TTY —
/// it is best-effort restoration after an abort.
#[test]
fn restore_after_abort_does_not_panic() {
    restore_after_abort();
}
