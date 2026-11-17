//! Purpose: The LSP editor's detail pane stays blank with no server selected,
//! and a view drawn without a scroll position starts at the top of the list.

use std::collections::BTreeMap;

use ratatui::Terminal;

use super::*;
use crate::config::LspSettings;
use crate::models::lsp_editor::Access;
use crate::test_support::{buffer_text, WritableTestBackend};

fn empty_editor() -> LspEditor {
    LspEditor::new(
        Access::Writable {
            path: "/srv/x/settings.yaml".into(),
        },
        LspSettings(BTreeMap::new()),
    )
}

#[test]
fn the_detail_pane_stays_blank_when_no_server_is_selected() {
    let editor = empty_editor();
    let mut terminal = Terminal::new(WritableTestBackend::new(30, 4)).expect("terminal");
    terminal
        .draw(|f| render_detail(f, f.area(), &editor))
        .expect("draw");
    assert_eq!(buffer_text(&terminal).trim(), "");
}

#[test]
fn a_view_drawn_without_a_scroll_position_starts_at_the_top() {
    let mut editor = empty_editor();
    let mut terminal = Terminal::new(WritableTestBackend::new(60, 12)).expect("terminal");
    terminal
        .draw(|f| render_view(f, f.area(), &mut editor))
        .expect("draw");
    assert!(buffer_text(&terminal).contains("Language servers: all"));
}
