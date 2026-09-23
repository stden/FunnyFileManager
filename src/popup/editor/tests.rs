//! Editor keys use an injected clipboard and never reach panel marking.

use std::{cell::RefCell, rc::Rc};

use tempfile::TempDir;

use crate::clipboard::ClipboardBackend;
use crate::models::{EditorSelection, Overlay};
use crate::popup::{process_popup_input, tests::app_with_editor};

use super::*;

#[derive(Default)]
struct Record {
    text: String,
    reads: usize,
    writes: usize,
    fail: bool,
}

#[derive(Clone, Default)]
struct Clipboard(Rc<RefCell<Record>>);

impl ClipboardBackend for Clipboard {
    fn get_native(&mut self) -> Result<String, String> {
        let mut record = self.0.borrow_mut();
        record.reads += 1;
        if record.fail {
            Err("read failed".to_owned())
        } else {
            Ok(record.text.clone())
        }
    }

    fn set_native(&mut self, text: &str) -> Result<(), String> {
        let mut record = self.0.borrow_mut();
        record.writes += 1;
        if record.fail {
            return Err("write failed".to_owned());
        }
        record.text = text.to_owned();
        Ok(())
    }

    fn emit_osc52(&mut self, _payload: &str) -> Result<(), String> {
        Err("no terminal in test".to_owned())
    }
}

fn key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    assert!(process_popup_input(app, KeyEvent::new(code, modifiers)));
}

#[test]
fn ctrl_a_copies_the_entire_document_and_typing_replaces_it() {
    let temp = TempDir::new().expect("temporary directory");
    let mut app = app_with_editor("a界\r\nβ🙂\r\n", temp.path().join("text.txt"));
    let clipboard = Clipboard::default();
    app.clipboard.install(Box::new(clipboard.clone()));
    key(&mut app, KeyCode::Char('a'), KeyModifiers::CONTROL);
    key(&mut app, KeyCode::Char('c'), KeyModifiers::CONTROL);
    assert_eq!(clipboard.0.borrow().text, "a界\r\nβ🙂\r\n");
    assert_eq!(app.get_editor().expect("editor").state, EditState::Clean);
    key(&mut app, KeyCode::Char('x'), KeyModifiers::NONE);
    assert_eq!(app.get_editor().expect("editor").content(), "x");
}

#[test]
fn shift_arrows_copy_and_paste_round_trip_unicode_without_panel_side_effects() {
    let temp = TempDir::new().expect("temporary directory");
    let mut app = app_with_editor("a界\nβ🙂\nz!", temp.path().join("text.txt"));
    let clipboard = Clipboard::default();
    app.clipboard.install(Box::new(clipboard.clone()));
    let panel_selection = app.left_panel.selected();
    app.get_editor_mut().expect("editor").cursor_line = 2;
    app.get_editor_mut().expect("editor").cursor_col = 1;
    key(&mut app, KeyCode::Up, KeyModifiers::SHIFT);
    key(&mut app, KeyCode::Up, KeyModifiers::SHIFT);
    key(&mut app, KeyCode::Insert, KeyModifiers::CONTROL);
    assert_eq!(clipboard.0.borrow().text, "界\nβ🙂\nz");
    assert_eq!(clipboard.0.borrow().writes, 1);
    assert_eq!(app.get_editor().expect("editor").state, EditState::Clean);

    clipboard.0.borrow_mut().text = "Ж\r\nX".to_owned();
    key(&mut app, KeyCode::Insert, KeyModifiers::SHIFT);
    let editor = app.get_editor().expect("editor");
    assert_eq!(editor.lines, ["aЖ", "X!"]);
    assert_eq!(editor.selection, EditorSelection::None);
    assert_eq!(editor.state, EditState::Modified);
    assert_eq!(clipboard.0.borrow().reads, 1);
    assert_eq!(app.left_panel.selected(), panel_selection);
    assert!(app.left_panel.marked.is_empty());
}

#[test]
fn copy_without_selection_does_not_touch_the_clipboard() {
    let temp = TempDir::new().expect("temporary directory");
    let mut app = app_with_editor("text", temp.path().join("text.txt"));
    let clipboard = Clipboard::default();
    app.clipboard.install(Box::new(clipboard.clone()));
    key(&mut app, KeyCode::Insert, KeyModifiers::CONTROL);
    key(&mut app, KeyCode::Char('c'), KeyModifiers::CONTROL);
    assert_eq!(clipboard.0.borrow().writes, 0);
    assert_eq!(app.get_editor().expect("editor").lines, ["text"]);
}

#[test]
fn failed_paste_preserves_selection_and_buffer_and_reports_in_editor() {
    let temp = TempDir::new().expect("temporary directory");
    let mut app = app_with_editor("界🙂", temp.path().join("text.txt"));
    let clipboard = Clipboard::default();
    clipboard.0.borrow_mut().fail = true;
    app.clipboard.install(Box::new(clipboard));
    key(&mut app, KeyCode::Right, KeyModifiers::SHIFT);
    key(&mut app, KeyCode::Insert, KeyModifiers::SHIFT);
    let editor = app.get_editor().expect("editor");
    assert_eq!(editor.lines, ["界🙂"]);
    assert_eq!(editor.selected_text().as_deref(), Some("界"));
    assert_eq!(editor.state, EditState::Clean);
    assert!(editor
        .notice
        .as_ref()
        .expect("visible error")
        .contains("Paste failed"));
}

#[test]
fn clipboard_shortcuts_are_consumed_by_search_and_exit_dialogs() {
    let temp = TempDir::new().expect("temporary directory");
    let mut app = app_with_editor("text", temp.path().join("text.txt"));
    let clipboard = Clipboard::default();
    app.clipboard.install(Box::new(clipboard.clone()));
    key(&mut app, KeyCode::Right, KeyModifiers::SHIFT);
    key(&mut app, KeyCode::F(7), KeyModifiers::NONE);
    key(&mut app, KeyCode::Insert, KeyModifiers::CONTROL);
    key(&mut app, KeyCode::Insert, KeyModifiers::SHIFT);
    assert_eq!(clipboard.0.borrow().reads, 0);
    assert_eq!(clipboard.0.borrow().writes, 0);
    key(&mut app, KeyCode::Esc, KeyModifiers::NONE);
    app.get_editor_mut().expect("editor").state = EditState::ConfirmingExit(ExitChoice::Save);
    key(&mut app, KeyCode::Insert, KeyModifiers::CONTROL);
    key(&mut app, KeyCode::Insert, KeyModifiers::SHIFT);
    assert_eq!(clipboard.0.borrow().reads, 0);
    assert_eq!(clipboard.0.borrow().writes, 0);
    assert!(matches!(app.overlay, Overlay::Editor(_)));
}

#[test]
fn save_shortcut_cannot_escape_search_or_exit_confirmation() {
    let temp = TempDir::new().expect("temporary directory");
    let path = temp.path().join("text.txt");
    std::fs::write(&path, "original界").expect("original file");
    let mut app = app_with_editor("original界", path.clone());
    key(&mut app, KeyCode::End, KeyModifiers::NONE);
    key(&mut app, KeyCode::Char('!'), KeyModifiers::NONE);

    key(&mut app, KeyCode::F(7), KeyModifiers::NONE);
    key(&mut app, KeyCode::Char('界'), KeyModifiers::NONE);
    key(&mut app, KeyCode::F(2), KeyModifiers::NONE);
    assert_unsaved_search(&app, &path);

    key(&mut app, KeyCode::Esc, KeyModifiers::NONE); // Close search.
    key(&mut app, KeyCode::Esc, KeyModifiers::NONE); // Ask whether to save.
    key(&mut app, KeyCode::F(2), KeyModifiers::NONE);
    let editor = app.get_editor().expect("confirmation remains in editor");
    assert!(editor.search.prompt.is_none());
    assert_eq!(editor.state, EditState::ConfirmingExit(ExitChoice::Save));
    assert_eq!(editor.lines, ["original界!"]);
    assert_eq!(
        std::fs::read_to_string(path).expect("disk content"),
        "original界"
    );
}

#[test]
fn ignored_editor_command_keeps_selection_and_unsaved_file_unchanged() {
    let temp = TempDir::new().expect("temporary directory");
    let path = temp.path().join("text.txt");
    std::fs::write(&path, "界text").expect("original file");
    let mut app = app_with_editor("界text", path.clone());
    key(&mut app, KeyCode::End, KeyModifiers::NONE);
    key(&mut app, KeyCode::Char('!'), KeyModifiers::NONE);
    key(&mut app, KeyCode::Home, KeyModifiers::NONE);
    key(&mut app, KeyCode::Right, KeyModifiers::SHIFT);

    key(&mut app, KeyCode::F(8), KeyModifiers::NONE);
    let editor = app
        .get_editor()
        .expect("unsupported key stays inside editor");
    assert_eq!(editor.selected_text().as_deref(), Some("界"));
    assert_eq!(editor.lines, ["界text!"]);
    assert_eq!(editor.state, EditState::Modified);
    assert_eq!(
        std::fs::read_to_string(path).expect("file remains on disk"),
        "界text"
    );
}

#[cfg(feature = "keepass")]
#[test]
fn editor_copy_cancels_the_pending_secret_wipe() {
    let temp = TempDir::new().expect("temporary directory");
    let mut app = app_with_editor("text", temp.path().join("text.txt"));
    app.clipboard.install(Box::new(Clipboard::default()));
    app.clipboard.arm(std::time::Instant::now());
    key(&mut app, KeyCode::Right, KeyModifiers::SHIFT);
    key(&mut app, KeyCode::Insert, KeyModifiers::CONTROL);
    assert!(!app.clipboard.is_armed());
}

/// `request_save` is only ever called from the `Editor` overlay's own key
/// handling, so its "no editor open" branch cannot be reached through
/// `process_popup_input`. Called directly, like the guard it is.
#[test]
fn a_save_request_does_nothing_without_an_open_editor() {
    let mut app = App::new().expect("test app");
    let before = app.status_text();

    save::request_save(&mut app, AfterSave::Close);

    assert!(app.get_editor().is_none());
    assert_eq!(
        app.status_text(),
        before,
        "the guard must not report anything"
    );
}

#[test]
fn f8_opens_the_encoding_dialog_and_esc_closes_it() {
    let temp = TempDir::new().expect("temporary directory");
    let mut app = app_with_editor("text", temp.path().join("text.txt"));
    key(&mut app, KeyCode::F(8), KeyModifiers::NONE);
    assert_eq!(
        app.get_editor().expect("editor").encoding_choice,
        Some(TextEncoding::Utf8)
    );
    key(&mut app, KeyCode::Esc, KeyModifiers::NONE);
    assert_eq!(app.get_editor().expect("editor").encoding_choice, None);
}

#[test]
fn encoding_arrows_walk_the_list_and_enter_keeps_utf8() {
    let temp = TempDir::new().expect("temporary directory");
    let path = temp.path().join("text.txt");
    std::fs::write(&path, "text").expect("file");
    let mut app = app_with_editor("text", path);
    key(&mut app, KeyCode::F(8), KeyModifiers::NONE);
    key(&mut app, KeyCode::Left, KeyModifiers::NONE);
    key(&mut app, KeyCode::Right, KeyModifiers::NONE);
    key(&mut app, KeyCode::Tab, KeyModifiers::NONE);
    key(&mut app, KeyCode::Up, KeyModifiers::NONE);
    key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    let editor = app.get_editor().expect("editor");
    assert_eq!(editor.encoding_choice, None);
    assert_eq!(editor.format.encoding, TextEncoding::Utf8);
}

#[test]
fn encoding_helpers_do_nothing_without_an_open_editor() {
    let mut app = App::new().expect("test app");
    process_encoding(&mut app, KeyCode::Enter);
    crate::ops::change_editor_encoding(&mut app, TextEncoding::Utf8);
    crate::ops::reload_editor_encoding(&mut app, TextEncoding::Utf8);
    assert!(app.get_editor().is_none());
}

#[test]
fn process_encoding_returns_when_the_dialog_is_not_open() {
    let temp = TempDir::new().expect("temporary directory");
    let mut app = app_with_editor("text", temp.path().join("text.txt"));
    process_encoding(&mut app, KeyCode::Down);
    assert_eq!(app.get_editor().expect("editor").encoding_choice, None);
}

#[test]
fn encoding_reload_of_a_missing_file_leaves_a_notice() {
    let temp = TempDir::new().expect("temporary directory");
    let mut app = app_with_editor("text", temp.path().join("missing.txt"));
    key(&mut app, KeyCode::F(8), KeyModifiers::NONE);
    key(&mut app, KeyCode::Char('r'), KeyModifiers::NONE);
    let editor = app.get_editor().expect("editor");
    assert!(editor.notice.is_some(), "{:?}", editor.notice);
}

fn assert_unsaved_search(app: &App, path: &std::path::Path) {
    let editor = app.get_editor().expect("search remains in editor");
    assert_eq!(
        editor
            .search
            .prompt
            .as_ref()
            .expect("search stays open")
            .query,
        "界"
    );
    assert_eq!(editor.lines, ["original界!"]);
    assert_eq!(editor.state, EditState::Modified);
    assert_eq!(
        std::fs::read_to_string(path).expect("disk content"),
        "original界"
    );
}

#[test]
fn encoding_dialog_ignores_keys_that_are_not_choices() {
    let temp = TempDir::new().expect("temporary directory");
    let mut app = app_with_editor("text", temp.path().join("text.txt"));
    key(&mut app, KeyCode::F(8), KeyModifiers::NONE);
    key(&mut app, KeyCode::Char('x'), KeyModifiers::NONE);
    let editor = app.get_editor().expect("editor");
    assert_eq!(editor.encoding_choice, Some(TextEncoding::Utf8));
    assert_eq!(
        editor.lines,
        ["text"],
        "the key must not reach the document"
    );
}

#[test]
fn lsp_shortcuts_record_the_request_for_the_worker_without_editing() {
    use crate::models::{lsp::Intent, LspFeature};
    let temp = TempDir::new().expect("temporary directory");
    let mut app = app_with_editor("text", temp.path().join("main.rs"));
    for (code, modifiers, expected) in [
        (
            KeyCode::Char(' '),
            KeyModifiers::CONTROL,
            Intent::Query(LspFeature::Completion),
        ),
        (
            KeyCode::Char('k'),
            KeyModifiers::CONTROL,
            Intent::Query(LspFeature::Hover),
        ),
        (
            KeyCode::Char(']'),
            KeyModifiers::CONTROL,
            Intent::Query(LspFeature::Definition),
        ),
        (
            KeyCode::Char('r'),
            KeyModifiers::CONTROL | KeyModifiers::ALT,
            Intent::Restart,
        ),
    ] {
        key(&mut app, code, modifiers);
        let editor = app.get_editor().expect("editor");
        assert!(editor.lsp.intent == expected, "{code:?} {modifiers:?}");
        assert_eq!(editor.lines, ["text"]);
    }
}

#[test]
fn an_open_lsp_popup_takes_keys_before_the_document() {
    use crate::models::lsp::Dialog;
    let temp = TempDir::new().expect("temporary directory");
    let mut app = app_with_editor("text", temp.path().join("main.rs"));
    app.get_editor_mut().expect("editor").lsp.popup = Some(Dialog::Hover {
        text: "docs".into(),
        scroll: 0,
    });

    key(&mut app, KeyCode::Char('x'), KeyModifiers::NONE);
    let editor = app.get_editor().expect("editor");
    assert_eq!(editor.lines, ["text"]);
    assert!(editor.lsp.popup.is_some());

    key(&mut app, KeyCode::Esc, KeyModifiers::NONE);
    let editor = app
        .get_editor()
        .expect("Esc closes the popup, not the editor");
    assert!(editor.lsp.popup.is_none());
    assert_eq!(editor.state, EditState::Clean);
}

/// A backend that can only write, like the terminal-only OSC 52 route.
struct WriteOnly;

impl ClipboardBackend for WriteOnly {
    fn set_native(&mut self, _text: &str) -> Result<(), String> {
        Ok(())
    }

    fn emit_osc52(&mut self, _payload: &str) -> Result<(), String> {
        Ok(())
    }
}

#[test]
fn paste_from_a_write_only_clipboard_is_refused_without_editing() {
    assert_eq!(
        WriteOnly.get_native(),
        Err("clipboard text is unavailable".to_owned())
    );
    let temp = TempDir::new().expect("temporary directory");
    let mut app = app_with_editor("text", temp.path().join("text.txt"));
    app.clipboard.install(Box::new(WriteOnly));
    key(&mut app, KeyCode::Insert, KeyModifiers::SHIFT);
    let editor = app.get_editor().expect("editor");
    assert_eq!(editor.lines, ["text"]);
    assert_eq!(editor.state, EditState::Clean);
    assert_eq!(
        editor.notice.as_deref(),
        Some("Paste failed: clipboard text is unavailable")
    );
}

#[test]
fn failed_copy_reports_in_editor_and_writes_nothing() {
    let temp = TempDir::new().expect("temporary directory");
    let mut app = app_with_editor("text", temp.path().join("text.txt"));
    let clipboard = Clipboard::default();
    clipboard.0.borrow_mut().fail = true;
    app.clipboard.install(Box::new(clipboard.clone()));
    key(&mut app, KeyCode::Right, KeyModifiers::SHIFT);
    key(&mut app, KeyCode::Insert, KeyModifiers::CONTROL);
    let editor = app.get_editor().expect("editor");
    let notice = editor.notice.as_deref().expect("visible error");
    assert!(notice.starts_with("Copy failed: "), "{notice}");
    assert_eq!(clipboard.0.borrow().text, "");
    assert_eq!(editor.selected_text().as_deref(), Some("t"));
}
