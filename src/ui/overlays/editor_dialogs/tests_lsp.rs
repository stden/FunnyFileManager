//! Purpose: An editor with a language-server popup open draws that popup
//! through the editor dialog layer.

use ratatui::Terminal;

use super::*;
use crate::lsp::features::Reply;
use crate::lsp::types::{Feature, Query, ServerState};
use crate::models::Editor;
use crate::mouse::DialogMouse;
use crate::test_support::{buffer_text, WritableTestBackend};

#[test]
fn an_open_hover_popup_is_drawn_as_the_editor_dialog() {
    let root = tempfile::TempDir::new().expect("test dir");
    let mut editor = Editor::from_content(
        "source".into(),
        root.path().join("source.rs"),
        "fn main() {}\n",
    );
    editor.lsp.state = ServerState::Ready;
    editor.cursor_col = 2;
    let query = Query {
        stamp: editor.lsp_stamp(),
        position: editor.lsp_position().expect("a position"),
        feature: Feature::Hover,
    };
    editor.show_lsp_reply(query, Reply::Hover("docs for main".to_owned()));
    let mut tracker = DialogMouse::default();
    let mut terminal = Terminal::new(WritableTestBackend::new(60, 12)).expect("terminal");
    let mut drawn = false;
    terminal
        .draw(|f| drawn = render_editor_dialog(f, &mut tracker, &editor, (0, 0)))
        .expect("draw");
    assert!(drawn);
    assert!(buffer_text(&terminal).contains("docs for main"));
}
