//! Purpose: Assert the editor's visible language-server information: hover,
//! completion and definition lists, the header's status, and workspace
//! trust. Diagnostic underlines are in `tests_diagnostics`.

use super::*;
use crate::lsp::positions::{Position, Range};
use ratatui::{backend::TestBackend, Terminal};
use tempfile::TempDir;

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
        .draw(|frame| assert!(popup(frame, &editor, (5, 5))))
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
fn hover_and_completion_docs_sanitize_control_characters() {
    let (_, hover_text, _) = contents(&Dialog::Hover {
        text: "safe\x1b]52;c;ZXY=\x07\nline2\x00".into(),
        scroll: 0,
    });
    assert!(!hover_text.contains('\x1b'));
    assert!(!hover_text.contains('\x07'));
    assert!(!hover_text.contains('\0'));
    assert!(hover_text.contains("safe") && hover_text.contains("line2"));
}

#[test]
fn completion_and_definition_lists_include_selection_details_and_locations() {
    use crate::lsp::{
        features::{self, Reply},
        types::{CompletionTrigger, Feature, Query},
    };
    let root = TempDir::new().unwrap();
    let editor = Editor::from_content("source".into(), root.path().join("source.rs"), "source");
    let query = Query {
        stamp: editor.lsp_stamp(),
        position: Position::default(),
        feature: Feature::Completion(CompletionTrigger::Manual),
    };
    let Reply::Completions(items) = features::parse(
        Feature::Completion(CompletionTrigger::Manual),
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
    assert_eq!(status(&editor).len(), 0);
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

/// One `marker label [kind]` line per item, then a blank line and the
/// selected item's detail and documentation.
#[test]
fn completion_text_lists_the_items_then_the_selected_ones_details() {
    use crate::lsp::{
        features::{self, Reply},
        types::{CompletionTrigger, Feature},
    };
    let Reply::Completions(items) = features::parse(
        Feature::Completion(CompletionTrigger::Manual),
        &serde_json::json!([
            {"label":"alpha","kind":3,"detail":"fn alpha()","documentation":"first"},
            {"label":"beta","detail":"fn beta()","documentation":"second"}
        ]),
    )
    .unwrap() else {
        panic!()
    };
    assert_eq!(
        completions(&items, 1),
        "  alpha [3]\n> beta []\n\nfn beta()\nsecond"
    );
}

/// One `marker path:line:column` line per definition, counted from one.
#[test]
fn definition_text_is_one_location_per_line_counted_from_one() {
    let path = std::path::PathBuf::from("lib.rs");
    let at = Position {
        line: 4,
        character: 2,
    };
    let items = [
        crate::lsp::features::Definition {
            path: path.clone(),
            range: Range { start: at, end: at },
        },
        crate::lsp::features::Definition {
            path: path.clone(),
            range: Range::default(),
        },
    ];
    let label = crate::path_display::label(&path);
    assert_eq!(
        definitions(&items, 0),
        format!("> {label}:5:3\n  {label}:1:1\n")
    );
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
