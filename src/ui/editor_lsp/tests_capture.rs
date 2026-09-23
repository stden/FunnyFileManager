//! Purpose: Capture the real TUI buffer for the LSP review screenshot using synthetic source text.

use crate::{
    models::{lsp::Dialog, App, Editor, Overlay, Panel, ServerState},
    ui::ui,
};
use ratatui::{backend::TestBackend, Terminal};
use serde_json::json;
use std::path::PathBuf;

fn fixture() -> App {
    let panel = || Panel::from_entries(PathBuf::from("."), Vec::new());
    let mut app = App::with_panels(panel(), panel());
    let mut editor = Editor::from_content("sample.rs".into(), PathBuf::from("sample.rs"), "fn greet(name: &str) -> String {\n    format!(\"Hello, {name}\")\n}\n\nfn main() {\n    let message = greet(\"world\");\n    println!(\"{message}\");\n}\n");
    editor.lsp.state = ServerState::Ready;
    editor.cursor_line = 5;
    editor.cursor_col = 22;
    editor.lsp.popup = Some(Dialog::Hover {text: "```rust\nfn greet(name: &str) -> String\n```\n\nReturns a greeting for the supplied name.".into(), scroll: 0});
    app.set_overlay(Overlay::Editor(editor));
    app
}

#[test]
#[ignore = "explicit review artifact capture"]
fn capture_lsp_hover_review_screen() {
    let mut app = fixture();
    let mut terminal = Terminal::new(TestBackend::new(100, 24)).unwrap();
    terminal.draw(|frame| ui(frame, &mut app)).unwrap();
    let cells: Vec<_> = terminal.backend().buffer().content.iter().map(|cell| {
        json!({"text":cell.symbol(), "fg":cell.fg.to_string(), "bg":cell.bg.to_string()})
    }).collect();
    assert!(cells.iter().any(|cell| cell["text"] == "g"));
    println!(
        "FFM_SCREEN:{}",
        json!({"width":100,"height":24,"cells":cells})
    );
}
