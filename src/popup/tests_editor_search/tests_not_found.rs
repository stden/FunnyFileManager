//! Purpose: the "not found" box is modal — the key or paste that dismisses it never reaches the document.

use super::*;

// ─── the "not found" box is modal ───────────────────────────────────────────

/// Open `content`, search for something it does not hold, and return the app
/// with the "not found" box on screen.
fn after_a_failed_search(content: &str) -> App {
    let mut app = editing(content);
    press(&mut app, KeyCode::F(7));
    type_text(&mut app, "zzz");
    press(&mut app, KeyCode::Enter);
    assert!(editor(&app).search.not_found.is_some(), "test setup");
    app
}

/// The box is Far's modal warning: the key that dismisses it is spent on it.
/// Enter used to split the line, Esc closed the editor and a letter was typed
/// into the file, because the key fell through to the editor keymap.
#[test]
fn the_key_that_dismisses_the_not_found_box_never_reaches_the_document() {
    for code in [
        KeyCode::Enter,
        KeyCode::Esc,
        KeyCode::F(10),
        KeyCode::Char('k'),
    ] {
        let mut app = after_a_failed_search("alpha\nbeta");

        press(&mut app, code);

        let editor = app
            .get_editor()
            .unwrap_or_else(|| panic!("{code:?} closed the editor"));
        assert!(
            editor.search.not_found.is_none(),
            "{code:?} dismissed the box"
        );
        assert_eq!(
            editor.lines,
            ["alpha", "beta"],
            "{code:?} left the text alone"
        );
        assert!(
            !editor.state.is_modified(),
            "{code:?} left the buffer clean"
        );
        assert_eq!(caret(&app), (0, 0), "{code:?} left the caret alone");
    }
}

/// Text arriving while the box is up is no more the document's than a key is:
/// neither a terminal paste nor a batched burst of typing may slip past it.
#[test]
fn a_paste_while_the_not_found_box_is_up_does_not_reach_the_document() {
    let mut app = after_a_failed_search("alpha");

    assert!(
        !editor(&app).accepts_text(),
        "the box owns the input, so typing is not batched into the buffer"
    );
    editor::paste_text(&mut app, "pasted");

    assert_eq!(editor(&app).lines, ["alpha"]);
    assert!(!editor(&app).state.is_modified());
}
