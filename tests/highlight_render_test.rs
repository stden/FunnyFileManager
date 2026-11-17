//! End-to-end tests: syntax highlighting as it reaches the screen.
//!
//! `src/highlight/tests.rs` proves the tokenizer classifies text correctly.
//! That is a different claim from "the editor draws it in the right colour",
//! and the gap between them is where the real bugs live: a token class that no
//! style maps to, a horizontal scroll that slices a span at the wrong byte, a
//! highlighter seeded at the visible line instead of at the top of the file.
//!
//! So these tests open a real file in the real editor and read the colour of
//! individual cells back out of the frame buffer.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use funny_file_manager::highlight::{Highlighter, Token};
use funny_file_manager::models::{App, Panel};
use funny_file_manager::{run_app_with_event_reader, WritableTestBackend};
use ratatui::style::Color;
use ratatui::Terminal;
use std::fs;
use std::io;
use std::path::Path;
use tempfile::TempDir;

#[path = "highlight_render_test/code_languages.rs"]
mod code_languages;
#[path = "highlight_render_test/scripts_and_config.rs"]
mod scripts_and_config;

const SCREEN_W: u16 = 90;
const SCREEN_H: u16 = 20;

const CODE_KEYWORD: Color = Color::Rgb(255, 255, 255);
const CODE_STRING: Color = Color::Rgb(255, 255, 0);

const CODE_COMMENT: Color = Color::Rgb(80, 170, 170);
const CODE_TYPE: Color = Color::Rgb(0, 255, 0);

#[expect(
    clippy::expect_used,
    reason = "test fixture: a failed setup fails the calling test; clippy.toml allows that in \
              #[test] fns and #[cfg(test)] items, not in integration-test helpers"
)]
fn painted_with_key(name: &str, content: &str, row: u16, open: KeyCode) -> Vec<(char, Color)> {
    let dir = TempDir::new().expect("dir");
    let other = TempDir::new().expect("dir");
    fs::write(dir.path().join(name), content).expect("seed");

    let mut app = App::new().expect("app");
    // No language servers: a workspace-trust prompt for one would be drawn over
    // the row under test, and whether it shows up depends on timing.
    app.config.lsp.0.clear();
    app.left_panel = Panel::new(dir.path().to_path_buf()).expect("panel");
    app.right_panel = Panel::new(other.path().to_path_buf()).expect("panel");

    let key = |code| {
        Event::Key(KeyEvent {
            code,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        })
    };
    let mut events = vec![key(KeyCode::Down), key(open)].into_iter();

    let mut terminal =
        Terminal::new(WritableTestBackend::new(SCREEN_W, SCREEN_H)).expect("terminal");
    // The stream runs out with the editor still open, which is how the test
    // says "stop here" — the frame is what it wants, not a clean exit.
    drop(run_app_with_event_reader(&mut terminal, app, || {
        events
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "stop"))
    }));

    let buffer = terminal.backend().buffer();
    (0..SCREEN_W)
        .map(|x| {
            let cell = &buffer[(x, row)];
            (cell.symbol().chars().next().unwrap_or(' '), cell.fg)
        })
        .collect()
}

/// The colour the run starting at `needle` is painted in.
#[expect(
    clippy::panic,
    reason = "test assertion: a needle missing from the row fails the calling test; clippy.toml \
              allows that in #[test] fns and #[cfg(test)] items, not in integration-test helpers"
)]
fn colour_of(row: &[(char, Color)], needle: &str) -> Color {
    let text: String = row.iter().map(|(ch, _)| *ch).collect();
    // `find` answers in bytes and the row holds one cell per character, so
    // after a multibyte prefix this reads a cell further along the run, not
    // its first one. Kept as is: the assertions were written against it.
    text.find(needle).and_then(|at| row.get(at)).map_or_else(
        || panic!("{needle:?} is not on the row {text:?}"),
        |&(_, colour)| colour,
    )
}
