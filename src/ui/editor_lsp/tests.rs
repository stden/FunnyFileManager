//! Purpose: Assert visible language-server information and Unicode diagnostic underline cells.

use super::*;
use crate::lsp::positions::{Position, Range};
use ratatui::{backend::TestBackend, Terminal};
use tempfile::TempDir;

#[test]
fn diagnostics_underlines_the_emoji_range_and_shows_the_message() {
    let root = TempDir::new().unwrap();
    let mut editor = Editor::from_content("main.rs".into(), root.path().join("main.rs"), "a😀z");
    editor.cursor_col = 1;
    editor.lsp.state = ServerState::Ready;
    editor.lsp.diagnostics.push(emoji_diagnostic());
    let mut terminal = Terminal::new(TestBackend::new(20, 5)).unwrap();
    terminal
        .draw(|frame| diagnostics(frame, &editor, frame.area(), 0, Cells::new(0)))
        .unwrap();
    for (column, underlined) in [(1, true), (2, true), (3, false)] {
        assert_eq!(
            terminal.backend().buffer()[(column, 0)]
                .modifier
                .contains(Modifier::UNDERLINED),
            underlined
        );
    }
    assert!(status(&editor).contains("Error: bad type"));
}

fn emoji_diagnostic() -> Diagnostic {
    Diagnostic {
        range: Range {
            start: Position {
                line: 0,
                character: 1,
            },
            end: Position {
                line: 0,
                character: 3,
            },
        },
        message: "bad type".into(),
        severity: Some(1),
    }
}

#[test]
fn hover_is_rendered_as_readable_text_in_a_bounded_popup() {
    let root = TempDir::new().unwrap();
    let mut editor = Editor::from_content("main.rs".into(), root.path().join("main.rs"), "");
    editor.lsp.popup = Some(Dialog::Hover {
        text: "```rust\nfn example()\n```".into(),
        scroll: 0,
    });
    let mut terminal = Terminal::new(TestBackend::new(60, 12)).unwrap();
    terminal
        .draw(|frame| assert!(popup(frame, &editor)))
        .unwrap();
    let text: String = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect();
    assert!(text.contains("fn example()"));
    assert!(text.contains("Hover"));
}

#[test]
fn completion_and_definition_lists_include_selection_details_and_locations() {
    use crate::lsp::{
        features::{self, Reply},
        types::{Feature, Query},
    };
    let root = TempDir::new().unwrap();
    let editor = Editor::from_content("source".into(), root.path().join("source.rs"), "source");
    let query = Query {
        stamp: editor.lsp_stamp(),
        position: Position::default(),
        feature: Feature::Completion,
    };
    let Reply::Completions(items) = features::parse(
        Feature::Completion,
        &serde_json::json!([{"label":"item","kind":3,"detail":"signature","documentation":"docs"}]),
    )
    .unwrap() else {
        panic!()
    };
    let (_, text, _) = contents(&Dialog::Completion {
        query,
        items,
        selected: 0,
    });
    assert!(text.contains("> item [3]") && text.contains("signature") && text.contains("docs"));
    check_definition_list(&editor);
    assert_eq!(marker(0, 1), " ");
    assert!(status(&editor).is_empty());
}

fn check_definition_list(editor: &Editor) {
    let item = crate::lsp::features::Definition {
        path: editor.path.clone(),
        range: Range::default(),
    };
    let (_, text, _) = contents(&Dialog::Definitions {
        items: vec![item],
        selected: 0,
    });
    assert!(text.contains("source.rs:1:1"));
}

/// `text` as an editor line with the default tab stops.
fn line(text: &str) -> crate::ui::EditorLine<'_> {
    crate::ui::EditorLine {
        text,
        tab: crate::models::TabSize::DEFAULT,
    }
}

#[test]
fn invalid_and_offscreen_diagnostics_do_not_paint_other_text() {
    let mut diagnostic = emoji_diagnostic();
    assert_eq!(
        diagnostic_columns(&diagnostic, 1, line("plain"), Cells::new(0), 8),
        None
    );
    assert_eq!(
        diagnostic_columns(&diagnostic, 0, line("a😀z"), Cells::new(8), 8),
        None
    );
    diagnostic.range.start.character = 2;
    assert_eq!(
        diagnostic_columns(&diagnostic, 0, line("a😀z"), Cells::new(0), 8),
        None
    );
    diagnostic.range.start.character = 5;
    diagnostic.range.end.character = 5;
    assert_eq!(
        diagnostic_columns(&diagnostic, 0, line("a😀z"), Cells::new(0), 8),
        None
    );
    diagnostic.range.start = Position::default();
    diagnostic.range.end = Position {
        line: 2,
        character: 0,
    };
    assert_eq!(
        diagnostic_columns(&diagnostic, 1, line("middle"), Cells::new(0), 3),
        Some(0..3)
    );
}

#[test]
fn diagnostic_messages_stop_at_the_exclusive_end_but_include_point_ranges() {
    let root = TempDir::new().unwrap();
    let mut editor = Editor::from_content("source".into(), root.path().join("source.rs"), "a😀z");
    editor.lsp.state = ServerState::Ready;
    editor.lsp.diagnostics.push(emoji_diagnostic());
    editor.cursor_col = 2;
    assert!(!status(&editor).contains("bad type"));
    editor.lsp.diagnostics[0].range.start = editor.lsp.diagnostics[0].range.end;
    assert!(status(&editor).contains("bad type"));
}

#[test]
fn diagnostic_severity_selects_warning_and_hint_colors() {
    let mut terminal = Terminal::new(TestBackend::new(4, 1)).unwrap();
    terminal
        .draw(|frame| {
            paint(frame, 0, 0, 0..1, Some(2));
            paint(frame, 0, 0, 1..2, Some(4));
        })
        .unwrap();
    assert_eq!(terminal.backend().buffer()[(0, 0)].fg, Color::Yellow);
    assert_eq!(terminal.backend().buffer()[(1, 0)].fg, Color::Cyan);
}
#[test]
fn language_server_status_precedes_the_path_on_narrow_editor_headers() {
    let root = TempDir::new().unwrap();
    let mut editor = Editor::from_content("source".into(), root.path().join("source.rs"), "");
    assert_eq!(
        label(&editor, " *"),
        format!(" {} *", crate::path_display::label(&editor.path))
    );
    for state in [
        ServerState::Starting,
        ServerState::Initializing,
        ServerState::Ready,
        ServerState::Unavailable,
        ServerState::Untrusted,
        ServerState::Failed,
    ] {
        editor.lsp.state = state;
        assert!(label(&editor, "").starts_with(&format!(" {}", editor.lsp.state.label())));
    }
}

#[test]
fn workspace_trust_explains_execution_and_defaults_to_disabled() {
    let root = TempDir::new().unwrap();
    let (_, text, _) = contents(&Dialog::Trust {
        root: root.path().to_path_buf(),
        allow: false,
    });
    assert!(text.contains("execute this project's build scripts"));
    assert!(text.contains(&format!("{} Keep LSP disabled", marker(0, 0))));
    assert!(text.contains(crate::path_display::label(root.path()).as_ref()));
}
