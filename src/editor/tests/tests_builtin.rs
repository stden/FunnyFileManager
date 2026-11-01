//! Purpose: Tests of `handle_edit_builtin` (F4): opening the overlay, the code-page
//! setting it honours, and the view history both editors record.
use super::*;

#[test]
fn test_handle_edit_builtin_opens_editor_overlay() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    let file_path = temp_dir.path().join("file.txt");
    fs::write(&file_path, "line one\nline two\n").expect("write file");

    let mut app = build_app(&temp_dir);
    assert!(select_entry(&mut app.left_panel, "file.txt"));

    handle_edit_builtin(&mut app)?;

    let editor = app.get_editor().expect("editor overlay should be open");
    assert_eq!(editor.title, "file.txt");
    assert_eq!(editor.path, file_path);
    assert_eq!(editor.lines, vec!["line one", "line two"]);
    assert!(!editor.state.is_modified());
    assert!(app.status_text().contains("Editing"));
    Ok(())
}

#[test]
fn test_handle_edit_builtin_no_selection() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    let mut app = build_app(&temp_dir);
    app.left_panel.select(None);

    handle_edit_builtin(&mut app)?;

    assert!(app.get_editor().is_none());
    assert_eq!(app.status_text().len(), 0);
    Ok(())
}

#[test]
fn test_handle_edit_builtin_directory_selected() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    fs::create_dir(temp_dir.path().join("subdir")).expect("create subdir");

    let mut app = build_app(&temp_dir);
    assert!(select_entry(&mut app.left_panel, "subdir"));

    handle_edit_builtin(&mut app)?;

    assert!(app.get_editor().is_none());
    assert_eq!(app.status_text(), "Editing directories is not supported");
    Ok(())
}

#[test]
fn test_handle_edit_builtin_rejects_malformed_utf16() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    fs::write(temp_dir.path().join("bin.dat"), [0xFF, 0xFE, 0x00, 0xD8]).expect("write file");

    let mut app = build_app(&temp_dir);
    assert!(select_entry(&mut app.left_panel, "bin.dat"));

    handle_edit_builtin(&mut app)?;

    assert!(app.get_editor().is_none());
    assert!(app.status_text().contains("Cannot edit"));
    Ok(())
}

#[test]
fn editing_records_the_file_in_the_view_history() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    let file_path = temp_dir.path().join("edited.txt");
    fs::write(&file_path, "text\n").expect("write file");
    let mut app = build_app(&temp_dir);
    assert!(select_entry(&mut app.left_panel, "edited.txt"));
    handle_edit_builtin(&mut app)?;
    assert_eq!(app.view_history.last(), Some(file_path.as_path()));

    let mut app = build_app(&temp_dir);
    assert!(select_entry(&mut app.left_panel, "edited.txt"));
    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend)?;
    handle_edit_impl(&mut terminal, &mut app, |_, _, _| {
        Err(io::Error::other("no editor"))
    })?;
    assert_eq!(
        app.view_history.last(),
        Some(file_path.as_path()),
        "remembered on launch even when the editor fails"
    );
    Ok(())
}

#[test]
fn zz_probe_utf16le_setting_selects_utf16le() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    fs::write(temp_dir.path().join("empty.txt"), b"").expect("write file");

    let mut app = build_app(&temp_dir);
    app.settings
        .set(crate::menu::SettingsPage::CodePages, "UTF-16LE")
        .expect("valid code page");
    assert!(select_entry(&mut app.left_panel, "empty.txt"));

    handle_edit_builtin(&mut app)?;

    let editor = app.get_editor().expect("editor overlay should be open");
    assert_eq!(
        editor.format.encoding,
        crate::encoding::TextEncoding::Utf16Le
    );
    Ok(())
}

#[test]
fn zz_probe_windows1252_setting_selects_windows1252() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    fs::write(temp_dir.path().join("empty.txt"), b"").expect("write file");

    let mut app = build_app(&temp_dir);
    app.settings
        .set(crate::menu::SettingsPage::CodePages, "Windows-1252")
        .expect("valid code page");
    assert!(select_entry(&mut app.left_panel, "empty.txt"));

    handle_edit_builtin(&mut app)?;

    let editor = app.get_editor().expect("editor overlay should be open");
    assert_eq!(
        editor.format.encoding,
        crate::encoding::TextEncoding::Windows1252
    );
    Ok(())
}
