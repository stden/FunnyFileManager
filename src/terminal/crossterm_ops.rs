//! Purpose: `CrosstermOps`, the real `TerminalOps`: each terminal step as the escape sequence crossterm writes.

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::Backend, Terminal};
use std::io::{self, Write};

use super::{keyboard, KeyboardProtocol, TerminalOps};

pub struct CrosstermOps;

impl<B> TerminalOps<B> for CrosstermOps
where
    B: Backend + Write,
{
    fn enable_raw(&mut self) -> io::Result<()> {
        enable_raw_mode()
    }

    fn disable_raw(&mut self) -> io::Result<()> {
        disable_raw_mode()
    }

    fn enter_alternate(&mut self, terminal: &mut Terminal<B>) -> io::Result<()> {
        execute!(terminal.backend_mut(), EnterAlternateScreen)
    }

    fn leave_alternate(&mut self, terminal: &mut Terminal<B>) -> io::Result<()> {
        execute!(terminal.backend_mut(), LeaveAlternateScreen)
    }

    fn enable_mouse(&mut self, terminal: &mut Terminal<B>) -> io::Result<()> {
        execute!(terminal.backend_mut(), EnableMouseCapture)
    }

    fn disable_mouse(&mut self, terminal: &mut Terminal<B>) -> io::Result<()> {
        execute!(terminal.backend_mut(), DisableMouseCapture)
    }

    fn clear(&mut self, terminal: &mut Terminal<B>) -> io::Result<()> {
        terminal
            .clear()
            .map_err(|error| io::Error::other(error.to_string()))
    }

    fn show_cursor(&mut self, terminal: &mut Terminal<B>) -> io::Result<()> {
        terminal
            .show_cursor()
            .map_err(|error| io::Error::other(error.to_string()))
    }

    fn enable_keyboard(&mut self, terminal: &mut Terminal<B>) -> io::Result<KeyboardProtocol> {
        keyboard::push(terminal.backend_mut(), keyboard::host_protocol())
    }

    fn disable_keyboard(&mut self, terminal: &mut Terminal<B>) -> io::Result<KeyboardProtocol> {
        keyboard::pop(terminal.backend_mut(), keyboard::known_protocol())
    }

    fn wait_for_key(&mut self, terminal: &mut Terminal<B>, prompt: &str) -> io::Result<()> {
        let screen = terminal.backend_mut();
        // Raw mode: a bare `\n` would not return the cursor to column 0.
        write!(screen, "\r\n{prompt}")?;
        Write::flush(screen)?;
        // Read the way the event loop reads, which survives a hung-up
        // terminal; live and unrecorded, since the key only ends the pause.
        let mut input = crate::recording::Input::Live(None);
        while !input.read()?.as_ref().is_some_and(is_key_press) {}
        // End the prompt's line, so whatever runs next starts on its own.
        write!(screen, "\r\n")?;
        Write::flush(screen)
    }
}

/// A key going down: its release, mouse moves and resizes do not end a pause.
pub(super) fn is_key_press(event: &Event) -> bool {
    matches!(event, Event::Key(key) if key.kind == KeyEventKind::Press)
}
