//! Purpose: Holds feature-gated editor unit tests that need test-helper APIs.
//!

#[cfg(all(test, feature = "test-helpers"))]
mod tests_launch_outcomes;
#[cfg(all(test, feature = "test-helpers"))]
mod tests_panel_paths;

#[cfg(all(test, feature = "test-helpers"))]
mod editor_tests {
    use crate::editor::handle_edit_impl;

    use crate::models::EntryKind;
    use crate::models::FileSize;
    use crate::models::Modified;

    use crate::models::{App, DirEntry, Panel};

    use crate::test_support::EditorEnvGuard;
    use ratatui::Terminal;
    use std::fs::File;
    use std::io;
    use std::path::Path;
    use tempfile::TempDir;

    pub(super) fn create_test_app(temp_dir: &TempDir) -> io::Result<App> {
        let mut app = App::with_panels(
            Panel::new(temp_dir.path().to_path_buf())?,
            Panel::new(temp_dir.path().to_path_buf())?,
        );
        app.status = crate::status::Status::Cleared;
        Ok(app)
    }

    #[test]
    fn test_handle_edit_impl_no_selection() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let mut app = create_test_app(&temp_dir)?;
        app.left_panel.select(None);

        let backend = crate::WritableTestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend)?;

        let result = handle_edit_impl(&mut terminal, &mut app, |editor, args, path| {
            let _ = editor;
            let _ = args;
            let _ = path;
            panic!("Editor should not be called when no selection");
        });

        assert!(result.is_ok());
        assert_eq!(app.status_text(), "");
        Ok(())
    }

    #[test]
    fn test_handle_edit_impl_directory_selected() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let mut app = create_test_app(&temp_dir)?;

        // Ensure there's a parent directory entry
        if app.left_panel.entries.is_empty() || app.left_panel.entries[0].name != ".." {
            // Manually add a directory entry
            app.left_panel.entries.insert(
                0,
                DirEntry {
                    name: "test_dir".to_owned(),
                    kind: EntryKind::Directory,
                    size: FileSize::from_bytes(0),
                    modified: Modified::UNKNOWN,
                    executable: false,
                    ..DirEntry::default()
                },
            );
        }
        app.left_panel.select(Some(0));

        let backend = crate::WritableTestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend)?;

        let result = handle_edit_impl(&mut terminal, &mut app, |editor, args, path| {
            let _ = editor;
            let _ = args;
            let _ = path;
            panic!("Editor should not be called for directories");
        });

        assert!(result.is_ok());
        assert_eq!(app.status_text(), "Editing directories is not supported");
        Ok(())
    }

    #[test]
    fn test_handle_edit_impl_success() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.path().join("test.txt");
        File::create(&file_path)?;

        // Pin $EDITOR rather than reading it live inside the closure: comparing
        // the resolved editor against whatever the ambient environment happens
        // to hold makes the test depend on the machine it runs on, and on
        // whichever sibling test last touched the variable.
        let _guard = EditorEnvGuard::set(Path::new("ffm-test-editor"));
        let mut app = create_test_app(&temp_dir)?;

        // Find and select the file
        for (i, entry) in app.left_panel.entries.iter().enumerate() {
            if entry.name == "test.txt" {
                app.left_panel.select(Some(i));
                break;
            }
        }

        let backend = crate::WritableTestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend)?;

        let result = handle_edit_impl(&mut terminal, &mut app, |editor, args, path| {
            let _ = args;
            assert_eq!(editor, "ffm-test-editor");
            assert_eq!(path, file_path.as_path());
            Ok(std::process::ExitStatus::default())
        });

        assert!(result.is_ok());
        assert!(app.status_text().contains("Edited"));
        assert!(app.status_text().contains("test.txt"));
        Ok(())
    }

    #[test]
    fn test_handle_edit_impl_with_editor_env() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.path().join("test.txt");
        File::create(&file_path)?;

        let _guard = EditorEnvGuard::set(Path::new("vim"));

        let mut app = create_test_app(&temp_dir)?;

        // Find and select the file
        for (i, entry) in app.left_panel.entries.iter().enumerate() {
            if entry.name == "test.txt" {
                app.left_panel.select(Some(i));
                break;
            }
        }

        let backend = crate::WritableTestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend)?;

        let result = handle_edit_impl(&mut terminal, &mut app, |editor, args, path| {
            let _ = args;
            let _ = path;
            assert_eq!(editor, "vim");
            Ok(std::process::ExitStatus::default())
        });

        assert!(result.is_ok());
        Ok(())
    }

    #[test]
    fn test_handle_edit_impl_no_editor_env() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.path().join("test.txt");
        File::create(&file_path)?;

        let _guard = EditorEnvGuard::clear();

        let mut app = create_test_app(&temp_dir)?;

        // Find and select the file
        for (i, entry) in app.left_panel.entries.iter().enumerate() {
            if entry.name == "test.txt" {
                app.left_panel.select(Some(i));
                break;
            }
        }

        let backend = crate::WritableTestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend)?;

        let result = handle_edit_impl(&mut terminal, &mut app, |editor, args, path| {
            let _ = args;
            let _ = path;
            assert_eq!(editor, "nano"); // Should use "nano" as default
            Ok(std::process::ExitStatus::default())
        });

        assert!(result.is_ok());
        assert!(app.status_text().contains("Edited"));
        Ok(())
    }
}
