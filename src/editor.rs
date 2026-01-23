#[cfg(all(not(feature = "test-helpers"), not(any(test, coverage))))]
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::Backend;
use ratatui::Terminal;
use std::io::{self, Write};
#[cfg(not(feature = "test-helpers"))]
use std::process::Command;

use crate::models::App;

#[cfg(feature = "test-helpers")]
pub fn handle_edit<B: Backend + Write>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> io::Result<()> {
    let _ = terminal;
    handle_edit_test_mode(app, TestEditorBehavior::Success)
}

#[cfg(not(feature = "test-helpers"))]
pub fn handle_edit<B: Backend + Write>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> io::Result<()> {
    handle_edit_impl(terminal, app, |editor, path| {
        Command::new(editor).arg(path).status()
    })
}

#[allow(dead_code)]
pub(crate) fn handle_edit_impl<B, F>(
    #[allow(unused_variables)] terminal: &mut Terminal<B>,
    app: &mut App,
    run_editor: F,
) -> io::Result<()>
where
    B: Backend + Write,
    F: FnOnce(&str, &std::path::Path) -> io::Result<std::process::ExitStatus>,
{
    let Some(selected_entry) = app.get_active_panel().get_selected_entry() else {
        return Ok(());
    };

    if selected_entry.is_dir {
        app.status_message = "Editing directories is not supported".to_string();
        return Ok(());
    }

    let selected = app.get_active_panel().selected_path().unwrap();

    // Skip terminal manipulation in test/coverage mode.
    #[cfg(all(not(feature = "test-helpers"), not(any(test, coverage))))]
    {
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
    }

    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "nano".to_string());
    let status = run_editor(&editor, &selected);

    #[cfg(all(not(feature = "test-helpers"), not(any(test, coverage))))]
    {
        enable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            EnterAlternateScreen,
            EnableMouseCapture
        )?;
        terminal.clear()?;
    }

    match status {
        Ok(code) => {
            if code.success() {
                app.status_message = format!("Edited {}", selected.display());
            } else {
                app.status_message = format!("Editor exited with status {}", code);
            }
        }
        Err(err) => {
            app.status_message = format!("Failed to launch editor: {}", err);
        }
    }

    app.left_panel.load_entries()?;
    app.left_panel.ensure_selection();
    app.right_panel.load_entries()?;
    app.right_panel.ensure_selection();
    Ok(())
}

#[cfg(feature = "test-helpers")]
#[derive(Clone, Copy, Debug)]
pub enum TestEditorBehavior {
    Success,
    FailLaunch,
    ExitError,
}

#[cfg(feature = "test-helpers")]
pub fn handle_edit_test_mode(app: &mut App, behavior: TestEditorBehavior) -> io::Result<()> {
    let Some(selected_entry) = app.get_active_panel().get_selected_entry() else {
        return Ok(());
    };

    if selected_entry.is_dir {
        app.status_message = "Editing directories is not supported".to_string();
        return Ok(());
    }

    let selected = app.get_active_panel().selected_path().unwrap();

    match behavior {
        TestEditorBehavior::FailLaunch => {
            app.status_message =
                "Failed to launch editor: No such file or directory (os error 2)".to_string();
        }
        TestEditorBehavior::ExitError => {
            app.status_message = "Editor exited with status exit status: 1".to_string();
        }
        TestEditorBehavior::Success => {
            app.status_message = format!("Edited {}", selected.display());
        }
    }

    app.left_panel.load_entries()?;
    app.left_panel.ensure_selection();
    app.right_panel.load_entries()?;
    app.right_panel.ensure_selection();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{handle_edit, handle_edit_impl};
    use crate::models::{App, Panel, PanelSide};
    use crate::test_support::{editor_env_lock, EditorEnvGuard, restore_editor_env};
    use crate::WritableTestBackend;
    use ratatui::Terminal;
    use std::ffi::OsString;
    use std::fs;
    use std::io;
    use std::path::{Path, PathBuf};
    use tempfile::TempDir;

    fn build_app(temp_dir: &TempDir) -> App {
        App {
            left_panel: Panel::new(temp_dir.path().to_path_buf()).expect("create left panel"),
            right_panel: Panel::new(temp_dir.path().to_path_buf()).expect("create right panel"),
            active_panel: PanelSide::Left,
            show_help: false,
            viewer: None,
            search: None,
            command_line: None,
            show_bookmarks: false,
            bookmarks: vec![None; 10],
            should_quit: false,
            status_message: String::new(),
            quick_search: None,
        }
    }

    fn select_entry(panel: &mut Panel, name: &str) -> bool {
        if let Some(index) = panel.entries.iter().position(|entry| entry.name == name) {
            panel.state.select(Some(index));
            true
        } else {
            false
        }
    }

    fn run_editor_success(_editor: &str, _path: &Path) -> io::Result<std::process::ExitStatus> {
        Ok(std::process::ExitStatus::default())
    }


    #[cfg(unix)]
    fn fake_editor_path(dir: &Path, exit_code: i32) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let path = dir.join("fake_editor.sh");
        let script = format!("#!/bin/sh\nexit {}\n", exit_code);
        fs::write(&path, script).expect("write fake editor script");
        let mut perms = fs::metadata(&path)
            .expect("read fake editor metadata")
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&path, perms).expect("set fake editor permissions");
        path
    }

    #[cfg(windows)]
    fn fake_editor_path(dir: &Path, exit_code: i32) -> PathBuf {
        let path = dir.join("fake_editor.cmd");
        let script = format!("@echo off\r\nexit /b {}\r\n", exit_code);
        fs::write(&path, script).expect("write fake editor script");
        path
    }

    #[cfg(unix)]
    fn failure_exit_status() -> std::process::ExitStatus {
        use std::os::unix::process::ExitStatusExt;
        std::process::ExitStatus::from_raw(1 << 8)
    }

    #[cfg(windows)]
    fn failure_exit_status() -> std::process::ExitStatus {
        use std::os::windows::process::ExitStatusExt;
        std::process::ExitStatus::from_raw(1)
    }

    #[test]
    fn test_handle_edit_impl_no_selection() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("temp dir");
        let mut app = build_app(&temp_dir);
        app.left_panel.state.select(None);

        let backend = WritableTestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("terminal");

        let result = handle_edit_impl(&mut terminal, &mut app, run_editor_success);

        assert!(result.is_ok());
        assert!(app.status_message.is_empty());
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

        let result = handle_edit_impl(&mut terminal, &mut app, run_editor_success);

        assert!(result.is_ok());
        assert_eq!(app.status_message, "Editing directories is not supported");
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
        let expected_editor = editor_path.to_string_lossy().to_string();

        let backend = WritableTestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("terminal");

        let result = handle_edit_impl(&mut terminal, &mut app, |editor, path| {
            assert_eq!(editor, expected_editor);
            assert_eq!(path, file_path.as_path());
            run_editor_success(editor, path)
        });

        assert!(result.is_ok());
        assert!(app.status_message.contains("Edited"));
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

        let result = handle_edit_impl(&mut terminal, &mut app, |_, _| Ok(failure_exit_status()));

        assert!(result.is_ok());
        assert!(app.status_message.contains("Editor exited with status"));
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

        let result = handle_edit_impl(&mut terminal, &mut app, |_, _| {
            Err(io::Error::new(io::ErrorKind::NotFound, "missing editor"))
        });

        assert!(result.is_ok());
        assert!(app.status_message.contains("Failed to launch editor"));
        Ok(())
    }

    #[test]
    fn test_handle_edit_runs_command() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("temp dir");
        let file_path = temp_dir.path().join("file.txt");
        fs::write(&file_path, b"content").expect("write file");

        let editor_path = fake_editor_path(temp_dir.path(), 0);
        let _guard = EditorEnvGuard::set(&editor_path);

        let mut app = build_app(&temp_dir);
        assert!(select_entry(&mut app.left_panel, "file.txt"));

        let backend = WritableTestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("terminal");

        let result = handle_edit(&mut terminal, &mut app);

        assert!(result.is_ok());
        assert!(app.status_message.contains("Edited"));
        Ok(())
    }

    #[test]
    fn test_select_entry_returns_false() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("temp dir");
        let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("create panel");

        assert!(!select_entry(&mut panel, "missing.txt"));
        Ok(())
    }

    #[test]
    fn test_editor_env_guard_restores_existing_value() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("temp dir");
        let editor_path = fake_editor_path(temp_dir.path(), 0);

        let lock = editor_env_lock();
        let original = std::env::var_os("EDITOR");
        std::env::set_var("EDITOR", "keep");

        let guard = EditorEnvGuard::set_with_lock(lock, &editor_path);
        let current = std::env::var("EDITOR").unwrap();
        let expected = editor_path.to_string_lossy().to_string();
        assert!(current == expected);
        drop(guard);

        let lock = editor_env_lock();
        assert_eq!(std::env::var("EDITOR").unwrap(), "keep");
        restore_editor_env(Some(OsString::from("ffm_temp")));
        restore_editor_env(None);
        restore_editor_env(original);
        drop(lock);

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

        let backend = WritableTestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("terminal");

        let result = handle_edit_impl(&mut terminal, &mut app, |editor, path| {
            assert_eq!(editor, "nano"); // Should use nano as default
            assert_eq!(path, file_path.as_path());
            run_editor_success(editor, path)
        });

        assert!(result.is_ok());
        assert!(app.status_message.contains("Edited"));
        
        Ok(())
    }
}
