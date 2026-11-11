//! Purpose: Tests that really spawn the `$EDITOR` script and report its exit status.
use super::*;

/// Launch the resolved editor for real — the production runner, minus the
/// terminal juggling. Tests that use it must point `$EDITOR` at a script that
/// exits immediately.
fn run_editor_process(
    editor: &str,
    args: &[String],
    path: &Path,
) -> io::Result<std::process::ExitStatus> {
    std::process::Command::new(editor)
        .args(args)
        .arg(path)
        .status()
}

/// `$EDITOR` really is spawned, and exit code 0 reports success.
#[test]
fn test_handle_edit_impl_spawns_editor_from_env() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    let file_path = temp_dir.path().join("file.txt");
    fs::write(&file_path, b"content").expect("write file");

    let editor_path = fake_editor_path(temp_dir.path(), 0);
    let _guard = EditorEnvGuard::set(&editor_path);

    let mut app = build_app(&temp_dir);
    assert!(select_entry(&mut app.left_panel, "file.txt"));

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");

    handle_edit_impl(&mut terminal, &mut app, run_editor_process)?;

    assert!(
        app.status_text().contains("Edited"),
        "{}",
        app.status_text()
    );
    Ok(())
}

/// A non-zero exit from the spawned editor is reported, not swallowed.
#[test]
fn test_handle_edit_impl_spawned_editor_exit_error() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    let file_path = temp_dir.path().join("file.txt");
    fs::write(&file_path, b"content").expect("write file");

    let editor_path = fake_editor_path(temp_dir.path(), 1);
    let _guard = EditorEnvGuard::set(&editor_path);

    let mut app = build_app(&temp_dir);
    assert!(select_entry(&mut app.left_panel, "file.txt"));

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");

    handle_edit_impl(&mut terminal, &mut app, run_editor_process)?;

    assert!(
        app.status_text().contains("Editor exited with status"),
        "{}",
        app.status_text()
    );
    Ok(())
}

/// An editor path containing a space must still launch: the `$EDITOR`
/// whitespace split may not chop a real program path in half.
#[test]
fn test_handle_edit_impl_editor_path_with_space() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    let file_path = temp_dir.path().join("file.txt");
    fs::write(&file_path, b"content").expect("write file");

    let editor_dir = temp_dir.path().join("Program Files");
    fs::create_dir(&editor_dir).expect("create editor dir");
    let editor_path = fake_editor_path(&editor_dir, 0);
    let _guard = EditorEnvGuard::set(&editor_path);

    let mut app = build_app(&temp_dir);
    assert!(select_entry(&mut app.left_panel, "file.txt"));

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");

    handle_edit_impl(&mut terminal, &mut app, run_editor_process)?;

    assert!(
        app.status_text().contains("Edited"),
        "{}",
        app.status_text()
    );
    Ok(())
}
