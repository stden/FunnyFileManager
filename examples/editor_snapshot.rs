//! Export the real editor rendering as an SVG for documentation and review.
//! Run: `cargo run --example editor_snapshot -- docs/screenshots/editor-dotenv-selection.svg`

use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use funny_file_manager::{
    models::{App, Editor, Overlay},
    popup::process_popup_input,
    ui,
};
use ratatui::{backend::TestBackend, Terminal};

#[path = "support/snapshot.rs"]
mod snapshot;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args_os().nth(1).map_or_else(
        || PathBuf::from("editor-dotenv-selection.svg"),
        PathBuf::from,
    );
    let mut app = App::new()?;
    app.set_overlay(Overlay::Editor(Editor::from_content(
        ".env".to_owned(), PathBuf::from(".env"),
        "# Local development configuration\nAPP_NAME=\"Funny File Manager\"\nHOST=localhost\nPORT=8080\nexport API_URL=\"https://${HOST}:${PORT}/api\"\nLOG_LEVEL=debug # application logs\n\n# Shift+arrows: select | Ctrl+Insert: copy | Shift+Insert: paste\n",
    )));
    process_popup_input(&mut app, KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    for _ in 0..9 {
        process_popup_input(&mut app, KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
    }
    process_popup_input(&mut app, KeyEvent::new(KeyCode::End, KeyModifiers::SHIFT));

    let mut terminal = Terminal::new(TestBackend::new(100, 14)).expect("test backend");
    terminal
        .draw(|frame| ui::ui(frame, &mut app))
        .expect("render editor");
    snapshot::save(
        terminal.backend().buffer(),
        "FFM editor: dotenv highlighting and Shift selection",
        &output,
    )
}
