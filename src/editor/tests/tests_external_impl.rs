//! Purpose: Tests of `handle_edit_impl` driven by an injected editor runner: selection
//! guards, `$EDITOR` resolution and the reported launch outcomes.
use super::*;

#[test]
fn test_handle_edit_impl_no_selection() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    let mut app = build_app(&temp_dir);
    app.left_panel.select(None);

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");

    let result = handle_edit_impl(&mut terminal, &mut app, |_, _, _| {
        run_editor_success("", Path::new(""))
    });

    assert!(result.is_ok());
    assert_eq!(app.status_text().len(), 0);
    Ok(())
}

#[test]
fn test_handle_edit_impl_directory_selected() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    let subdir = temp_dir.path().join("subdir");
    fs::create_dir(&subdir).expect("create subdir");

    let mut app = build_app(&temp_dir);
    assert!(select_entry(&mut app.left_panel, "subdir"));

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");

    let result = handle_edit_impl(&mut terminal, &mut app, |_, _, _| {
        run_editor_success("", Path::new(""))
    });

    assert!(result.is_ok());
    assert_eq!(app.status_text(), "Editing directories is not supported");
    Ok(())
}

#[test]
fn test_handle_edit_impl_selected_path_none() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    let mut app = build_app(&temp_dir);
    *app.left_panel.entries = vec![DirEntry {
        name: "..".to_owned(),
        kind: EntryKind::File,
        size: FileSize::from_bytes(0),
        modified: Modified::UNKNOWN,
        executable: false,
        ..DirEntry::default()
    }];
    app.left_panel.select(Some(0));

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");

    let result = handle_edit_impl(&mut terminal, &mut app, |_, _, _| {
        panic!("editor should not be launched when selected_path is None")
    });

    assert!(result.is_ok());
    assert_eq!(app.status_text().len(), 0);
    Ok(())
}

#[test]
fn test_handle_edit_impl_success() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    let file_path = temp_dir.path().join("file.txt");
    fs::write(&file_path, b"content").expect("write file");

    let mut app = build_app(&temp_dir);
    assert!(select_entry(&mut app.left_panel, "file.txt"));
    let editor_path = Path::new("nano");
    let _guard = EditorEnvGuard::set(editor_path);

    let expected_editor = "nano";

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");

    let result = handle_edit_impl(&mut terminal, &mut app, |editor, _args, path| {
        assert_eq!(editor, expected_editor);

        assert_eq!(path, file_path.as_path());
        run_editor_success(editor, path)
    });

    assert!(result.is_ok());
    assert!(app.status_text().contains("Edited"));
    Ok(())
}

#[test]
fn test_handle_edit_impl_exit_error() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    let file_path = temp_dir.path().join("file.txt");
    fs::write(&file_path, b"content").expect("write file");

    let mut app = build_app(&temp_dir);
    assert!(select_entry(&mut app.left_panel, "file.txt"));

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");

    let result = handle_edit_impl(&mut terminal, &mut app, |_, _, _| Ok(failure_exit_status()));

    assert!(result.is_ok());
    assert!(app.status_text().contains("Editor exited with status"));
    Ok(())
}

#[test]
fn test_handle_edit_impl_launch_error() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    let file_path = temp_dir.path().join("file.txt");
    fs::write(&file_path, b"content").expect("write file");

    let mut app = build_app(&temp_dir);
    assert!(select_entry(&mut app.left_panel, "file.txt"));

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");

    let result = handle_edit_impl(&mut terminal, &mut app, |_, _, _| {
        Err(io::Error::new(io::ErrorKind::NotFound, "missing editor"))
    });

    assert!(result.is_ok());
    assert!(app.status_text().contains("Failed to launch editor"));
    Ok(())
}

#[test]
fn test_handle_edit_impl_propagates_reload_error() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    let file_path = temp_dir.path().join("file.txt");
    fs::write(&file_path, b"content").expect("write file");

    let _guard = EditorEnvGuard::clear();

    let mut app = build_app(&temp_dir);
    app.active_panel = PanelSide::Right;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("create right panel");
    assert!(select_entry(&mut app.right_panel, "file.txt"));
    app.left_panel.path = unreachable_folder();

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");

    let err = handle_edit_impl(&mut terminal, &mut app, |editor, _args, path| {
        assert_eq!(editor, "nano");
        assert_eq!(path, file_path.as_path());
        run_editor_success(editor, path)
    })
    .expect_err("a left panel with nowhere to go should fail reload");

    assert_eq!(err.kind(), io::ErrorKind::NotFound);
    assert!(app.status_text().contains("Edited"));
    Ok(())
}

#[test]
fn test_handle_edit_impl_default_editor() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    let file_path = temp_dir.path().join("file.txt");
    fs::write(&file_path, b"content").expect("write file");

    let mut app = build_app(&temp_dir);
    assert!(select_entry(&mut app.left_panel, "file.txt"));

    // Remove EDITOR env var to test default
    let _guard = EditorEnvGuard::clear();

    let expected_editor = "nano";

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");

    let result = handle_edit_impl(&mut terminal, &mut app, |editor, _args, path| {
        assert_eq!(editor, expected_editor);

        assert_eq!(path, file_path.as_path());
        run_editor_success(editor, path)
    });

    assert!(result.is_ok());
    assert!(app.status_text().contains("Edited"));

    Ok(())
}
