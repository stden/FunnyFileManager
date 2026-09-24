//! Purpose: read the next event — a lost terminal ends the session, not the program — and hand one event to paste, resize, mouse or key dispatch.

use std::io::{self, Write};

use crossterm::event::{Event, KeyEventKind};
use ratatui::{backend::Backend, Terminal};

use super::dispatch::dispatch_mouse;
use super::dispatch_key;
use super::input::{BufferedInput, EventSource};
use crate::diagnostics::{key_label, log_action};
use crate::models::{App, Lifecycle};

/// The next event, `None` for an idle tick, and `None` again once the session
/// has no terminal left to run in.
///
/// A lost terminal is the end of a session, not a failure of one: nothing can
/// be drawn for it and no key can ever arrive, so the loop is asked to quit
/// the way F10 asks — the caller still writes the histories and restores what
/// is left to restore. Every other read failure stays fatal, which is what
/// keeps the replay tests' end-of-script sentinel meaning what it says.
pub(super) fn next_event<F>(
    read_event: &mut BufferedInput<F>,
    app: &mut App,
) -> io::Result<Option<Event>>
where
    F: EventSource,
{
    match read_event.read(app) {
        Ok(event) => Ok(event),
        Err(error) if crate::recording::TerminalLost::caused(&error) => {
            log_action("Controlling terminal disconnected; leaving the event loop");
            app.lifecycle = Lifecycle::Quitting;
            Ok(None)
        }
        Err(error) => Err(error),
    }
}

pub(super) fn handle_event<B: Backend + Write>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    event: Event,
    panel_clicks: &mut crate::mouse::PanelClicks,
) -> io::Result<()> {
    if let Event::Paste(text) = event {
        panel_clicks.reset();
        crate::popup::paste_text(app, &text);
        return Ok(());
    }
    if let Event::Resize(width, height) = event {
        return terminal
            .resize(ratatui::layout::Rect::new(0, 0, width, height))
            .map_err(|error| io::Error::other(error.to_string()));
    }
    if let Event::Mouse(mouse) = event {
        let area = terminal.get_frame().area();
        dispatch_mouse(terminal, app, mouse, area, panel_clicks);
        return Ok(());
    }
    if let Event::Key(key) = event {
        panel_clicks.reset();
        // Only process Press events, ignore Release/Repeat to avoid double navigation on Windows
        if key.kind != KeyEventKind::Press {
            return Ok(());
        }

        log_action(&format!(
            "Key event: {} (modifiers: {:?})",
            key_label(key.code),
            key.modifiers
        ));

        dispatch_key(terminal, app, key);
    }
    Ok(())
}
