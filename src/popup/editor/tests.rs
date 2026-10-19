//! Editor keys use an injected clipboard and never reach panel marking.

use std::{cell::RefCell, rc::Rc};

use tempfile::TempDir;

use crate::clipboard::ClipboardBackend;
use crate::encoding::TextEncoding;
use crate::models::{AfterSave, EditState, EditorSelection, ExitChoice, Overlay};
use crate::popup::{process_popup_input, tests::app_with_editor};

use super::*;

mod dialog_precedence;
mod encoding;
mod file_name;
mod lsp_keys;

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
    assert_eq!(app.get_editor().expect("editor").content(), "x\r\n");
    key(&mut app, KeyCode::Char('ф'), KeyModifiers::CONTROL);
    key(&mut app, KeyCode::Char('с'), KeyModifiers::CONTROL);
    assert_eq!(clipboard.0.borrow().text, "x\r\n");
    key(&mut app, KeyCode::End, KeyModifiers::CONTROL);
    key(&mut app, KeyCode::Char('м'), KeyModifiers::CONTROL);
    assert_eq!(app.get_editor().expect("editor").content(), "xx\r\n\r\n");
    key(&mut app, KeyCode::Home, KeyModifiers::CONTROL);
    assert_eq!(app.get_editor().expect("editor").cursor_col, 0);
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
