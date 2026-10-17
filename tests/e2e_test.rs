//! End-to-end tests: keystrokes in, filesystem and screen out.
//!
//! Every test here drives the real event loop through `run_app_with_event_reader`
//! — the same function `main` calls — and then asserts on what a user could
//! actually observe: the files on disk, and the cells on the screen.
//!
//! That is the difference from the rest of the suite. A test that calls
//! `handle_copy` and checks a struct field proves the handler works; it cannot
//! prove that F5 reaches it, that the key is not swallowed by an overlay, that
//! the dispatcher routes it, or that the result is drawn. Those are exactly the
//! joints that broke while this program was being made to look like Far, and
//! they are only visible from the outside.
//!
//! The event loop consumes the `App`, which is a feature here: after the loop
//! there is no state left to peek at, so an assertion has to be about something
//! real.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use funny_file_manager::models::{App, Panel};
use funny_file_manager::{run_app_with_event_reader, WritableTestBackend};
use ratatui::Terminal;
use std::fs;
use std::io;
use std::path::Path;
use tempfile::TempDir;

#[path = "e2e_test/copy_dialog.rs"]
mod copy_dialog;
#[path = "e2e_test/dialog_buttons.rs"]
mod dialog_buttons;
#[path = "e2e_test/dialog_position.rs"]
mod dialog_position;
#[path = "e2e_test/editor.rs"]
mod editor;
#[path = "e2e_test/file_operations.rs"]
mod file_operations;
#[path = "e2e_test/folders_and_rename.rs"]
mod folders_and_rename;
#[path = "e2e_test/help_quit_view.rs"]
mod help_quit_view;
#[path = "e2e_test/key_bar_mouse.rs"]
mod key_bar_mouse;
#[path = "e2e_test/menu.rs"]
mod menu;
#[path = "e2e_test/mouse.rs"]
mod mouse;
#[path = "e2e_test/panel_mouse.rs"]
mod panel_mouse;
#[path = "e2e_test/scrolling.rs"]
mod scrolling;
#[path = "e2e_test/session.rs"]
mod session;
use mouse::{click, click_with, drag, find, scroll_down};

const SCREEN_W: u16 = 100;
const SCREEN_H: u16 = 24;

// ─── harness ────────────────────────────────────────────────────────────────

const fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent {
        code,
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    })
}

const fn key_with(code: KeyCode, modifiers: KeyModifiers) -> Event {
    Event::Key(KeyEvent {
        code,
        modifiers,
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    })
}

/// The keys that end a session: F10 asks, Enter answers Yes.
fn quit() -> Vec<Event> {
    vec![key(KeyCode::F(10)), key(KeyCode::Enter)]
}

fn typed(text: &str) -> Vec<Event> {
    text.chars().map(|ch| key(KeyCode::Char(ch))).collect()
}

/// Whether the key sequence is expected to end the session by itself.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Ends {
    /// The sequence quits; the loop must return of its own accord.
    Yes,
    /// The sequence stops with something still on screen, so it can be
    /// asserted against. The loop then runs out of events, which is how the
    /// test says "stop here" and not a failure.
    No,
}

/// Run the real event loop over `events` and return the screen it left behind.
#[expect(
    clippy::expect_used,
    reason = "test fixture: a failed setup, or a session that does not end when told to, fails the \
              calling test; clippy.toml allows that in #[test] fns and #[cfg(test)] items, not in \
              integration-test helpers"
)]
fn drive(left: &Path, right: &Path, events: Vec<Event>, ends: Ends) -> String {
    let mut app = App::new().expect("app");
    app.left_panel = Panel::new(left.to_path_buf()).expect("left panel");
    app.right_panel = Panel::new(right.to_path_buf()).expect("right panel");

    let mut terminal =
        Terminal::new(WritableTestBackend::new(SCREEN_W, SCREEN_H)).expect("terminal");
    let mut stream = events.into_iter();
    let outcome = run_app_with_event_reader(&mut terminal, app, || {
        stream
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "out of events"))
    });
    if ends == Ends::Yes {
        // An error here means the session did not end when it was told to,
        // which is a bug in the program and not in the test.
        outcome.expect("the session must end on F10 + Enter");
    }

    let buffer = terminal.backend().buffer();
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The common case: a sequence that quits at the end.
fn run(left: &Path, right: &Path, events: Vec<Event>) -> String {
    drive(left, right, events, Ends::Yes)
}

/// A sequence that stops with a dialog or overlay still up.
fn run_until(left: &Path, right: &Path, events: Vec<Event>) -> String {
    drive(left, right, events, Ends::No)
}

/// A pair of directories, the left one seeded with `files`.
#[expect(
    clippy::expect_used,
    reason = "test fixture: a failed setup fails the calling test; clippy.toml allows that in \
              #[test] fns and #[cfg(test)] items, not in integration-test helpers"
)]
fn fixture(files: &[(&str, &str)]) -> (TempDir, TempDir) {
    let left = TempDir::new().expect("left dir");
    let right = TempDir::new().expect("right dir");
    for (name, body) in files {
        fs::write(left.path().join(name), body).expect("seed file");
    }
    (left, right)
}
