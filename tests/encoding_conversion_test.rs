//! File-level conversion preserves bytes and refuses lossy saves.
use funny_file_manager::{
    editor::handle_edit_builtin,
    encoding::TextEncoding,
    models::{App, EditState, Panel},
};
use std::fs;
use tempfile::TempDir;

#[test]
fn encoding_dialog_converts_clean_unicode_text_without_reinterpreting_it() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use funny_file_manager::popup::process_popup_input;
    let root = TempDir::new().expect("directory");
    let mut app = open_bytes(&root, "Привет\r\n".as_bytes());
    assert!(process_popup_input(
        &mut app,
        KeyEvent::new(KeyCode::F(8), KeyModifiers::SHIFT)
    ));
    for code in [
        KeyCode::Down,
        KeyCode::Down,
        KeyCode::Down,
        KeyCode::Down,
        KeyCode::Enter,
    ] {
        assert!(process_popup_input(
            &mut app,
            KeyEvent::new(code, KeyModifiers::NONE)
        ));
    }
    let editor = app.get_editor_mut().expect("editor");
    assert_eq!(editor.content(), "Привет\r\n");
    assert_eq!(editor.format.encoding, TextEncoding::Windows1251);
    assert_eq!(editor.state, EditState::Modified);
    editor.save().expect("convert");
    assert_eq!(
        fs::read(root.path().join("text.txt")).expect("saved"),
        [0xcf, 0xf0, 0xe8, 0xe2, 0xe5, 0xf2, 13, 10]
    );
}

#[expect(
    clippy::expect_used,
    reason = "test fixture: a failed setup fails the calling test; clippy.toml allows that in \
              #[test] fns and #[cfg(test)] items, not in integration-test helpers"
)]
fn open_bytes(root: &TempDir, bytes: &[u8]) -> App {
    fs::write(root.path().join("text.txt"), bytes).expect("source");
    let mut app = App::new().expect("app");
    app.left_panel = Panel::new(root.path().to_path_buf()).expect("panel");
    app.right_panel = Panel::new(root.path().to_path_buf()).expect("panel");
    let index = app
        .left_panel
        .entries
        .iter()
        .position(|entry| entry.name == "text.txt");
    app.left_panel.select(index);
    handle_edit_builtin(&mut app).expect("open editor");
    app
}

#[test]
fn windows1251_converts_to_utf8_bom_without_changing_line_endings() {
    let root = TempDir::new().expect("directory");
    let bytes = [0xcf, 0xf0, 0xe8, 0xe2, 0xe5, 0xf2, 13, 10];
    let mut app = open_bytes(&root, &bytes);
    let editor = app.get_editor_mut().expect("editor");
    assert_eq!(editor.format.encoding, TextEncoding::Windows1251);
    assert_eq!(editor.content(), "Привет\r\n");
    editor.format.encoding = TextEncoding::Utf8Bom;
    editor.save().expect("convert");
    assert_eq!(
        fs::read(root.path().join("text.txt")).expect("saved"),
        b"\xef\xbb\xbf\xd0\x9f\xd1\x80\xd0\xb8\xd0\xb2\xd0\xb5\xd1\x82\r\n"
    );
}

#[test]
fn unicode_files_save_in_their_original_encoding() {
    let root = TempDir::new().expect("directory");
    for encoding in [
        TextEncoding::Utf8Bom,
        TextEncoding::Utf16Le,
        TextEncoding::Utf16Be,
    ] {
        let bytes = encoding.encode("Привет🙂\r\n").expect("fixture");
        let mut app = open_bytes(&root, &bytes);
        let editor = app.get_editor_mut().expect("editor");
        assert_eq!(editor.format.encoding, encoding);
        editor.save().expect("save");
        assert_eq!(
            fs::read(root.path().join("text.txt")).expect("saved"),
            bytes
        );
    }
}

#[test]
fn lossy_conversion_leaves_original_file_and_unsaved_buffer_intact() {
    let root = TempDir::new().expect("directory");
    let mut app = open_bytes(&root, b"original\n");
    let editor = app.get_editor_mut().expect("editor");
    editor.insert_char('🙂');
    editor.format.encoding = TextEncoding::Windows1251;
    assert!(editor.save().is_err());
    assert_eq!(
        fs::read(root.path().join("text.txt")).expect("original"),
        b"original\n"
    );
    assert_eq!(editor.state, EditState::Modified);
    assert!(editor.content().contains('🙂'));
}

#[test]
fn encoding_dialog_can_explicitly_reload_an_oem866_file() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use funny_file_manager::popup::process_popup_input;
    let root = TempDir::new().expect("directory");
    let mut app = open_bytes(&root, &[0x8f, 0xe0, 0xa8, 0xa2, 0xa5, 0xe2]);
    assert!(process_popup_input(
        &mut app,
        KeyEvent::new(KeyCode::F(8), KeyModifiers::SHIFT)
    ));
    for code in [KeyCode::Down, KeyCode::Char('r')] {
        assert!(process_popup_input(
            &mut app,
            KeyEvent::new(code, KeyModifiers::NONE)
        ));
    }
    let editor = app.get_editor().expect("editor");
    assert_eq!(editor.content(), "Привет");
    assert_eq!(editor.format.encoding, TextEncoding::Oem866);
    assert_eq!(editor.state, EditState::Clean);
}

#[test]
fn encoding_reload_refuses_to_discard_unsaved_edits() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let root = TempDir::new().expect("directory");
    let mut app = open_bytes(&root, b"original\n");
    app.get_editor_mut().expect("editor").insert_char('!');
    funny_file_manager::popup::process_popup_input(
        &mut app,
        KeyEvent::new(KeyCode::F(8), KeyModifiers::SHIFT),
    );
    for code in [KeyCode::Down, KeyCode::Char('r')] {
        funny_file_manager::popup::process_popup_input(
            &mut app,
            KeyEvent::new(code, KeyModifiers::NONE),
        );
    }
    let editor = app.get_editor().expect("editor");
    assert_eq!(editor.content(), "!original\n");
    assert_eq!(editor.state, EditState::Modified);
    assert_eq!(editor.format.encoding, TextEncoding::Utf8);
    assert_eq!(
        fs::read(root.path().join("text.txt")).expect("original"),
        b"original\n"
    );
}
