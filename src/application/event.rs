//! Purpose: read the next event — a lost terminal ends the session, not the program — and hand one event to paste, resize, mouse or key dispatch.

use std::io::{self, Write};
use std::time::Duration;

use crossterm::event::{Event, KeyEventKind};
use ratatui::{backend::Backend, Terminal};

use super::dispatch::dispatch_mouse;
use super::dispatch_key;
use super::input::{BufferedInput, EventSource};
use crate::diagnostics::{key_label, log_action};
use crate::models::{App, Lifecycle};

/// How long one event may keep the screen from answering before the keys
/// typed meanwhile are thrown away instead of run.
///
/// Past this the user is typing at a frozen screen: a copy that takes half a
/// minute shows nothing until it is over, and an F8 Enter typed into it used
/// to delete whatever the cursor stood on once it was. Far and mc put up a
/// progress window that swallows every key but Esc and runs none of them
/// afterwards; this does the same after the fact. Short enough that a
/// noticeable freeze counts, long enough that a key handled in the usual few
/// milliseconds never does.
pub(super) const UNANSWERED: Duration = Duration::from_millis(300);

/// The next event, `None` for an idle tick, and `None` again once the session
/// has no terminal left to run in.
pub(super) fn next_event<F>(
    read_event: &mut BufferedInput<F>,
    app: &mut App,
) -> io::Result<Option<Event>>
where
    F: EventSource,
{
    unless_terminal_lost(read_event.read(app), app)
}

/// Throw away the keys typed while the event that just ran kept the screen
/// from answering; see [`UNANSWERED`].
pub(super) fn discard_typeahead<F>(
    read_event: &mut BufferedInput<F>,
    app: &mut App,
) -> io::Result<()>
where
    F: EventSource,
{
    if unless_terminal_lost(read_event.discard_queued(), app)? {
        log_action("Esc typed during a long operation: it had already finished");
    }
    Ok(())
}

/// A lost terminal is the end of a session, not a failure of one: nothing can
/// be drawn for it and no key can ever arrive, so the loop is asked to quit
/// the way F10 asks — the caller still writes the histories and restores what
/// is left to restore. Every other read failure stays fatal, which is what
/// keeps the replay tests' end-of-script sentinel meaning what it says.
fn unless_terminal_lost<T: Default>(read: io::Result<T>, app: &mut App) -> io::Result<T> {
    match read {
        Ok(value) => Ok(value),
        Err(error) if crate::recording::TerminalLost::caused(&error) => {
            log_action("Controlling terminal disconnected; leaving the event loop");
            app.lifecycle = Lifecycle::Quitting;
            Ok(T::default())
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
