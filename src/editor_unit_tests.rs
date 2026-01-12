#[cfg(test)]
mod editor_tests {
    use crate::editor::handle_edit_impl;
    use crate::models::{App, DirEntry, Panel, PanelSide};
    use ratatui::Terminal;
    use std::fs::File;
    use std::io;
    use tempfile::TempDir;

    fn create_test_app(temp_dir: &TempDir) -> io::Result<App> {
        Ok(App {
            left_panel: Panel::new(temp_dir.path().to_path_buf())?,
            right_panel: Panel::new(temp_dir.path().to_path_buf())?,
            active_panel: PanelSide::Left,
            status_message: String::new(),
            show_help: false,
            viewer: None,
            search: None,
            command_line: None,
            show_bookmarks: false,
            bookmarks: vec![None; 10],
            should_quit: false,
        })
    }

    #[test]
    fn test_handle_edit_impl_no_selection() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let mut app = create_test_app(&temp_dir)?;
        app.left_panel.state.select(None);

        let backend = crate::WritableTestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend)?;

        let result = handle_edit_impl(&mut terminal, &mut app, |_, _| {
            panic!("Editor should not be called when no selection");
        });

        assert!(result.is_ok());
        assert_eq!(app.status_message, "");
        Ok(())
    }

    #[test]
    fn test_handle_edit_impl_directory_selected() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let mut app = create_test_app(&temp_dir)?;
        
        // Ensure there's a parent directory entry
        if !app.left_panel.entries.is_empty() && app.left_panel.entries[0].name == ".." {
            app.left_panel.state.select(Some(0));
        } else {
            // Manually add a directory entry
            app.left_panel.entries.insert(0, DirEntry {
                name: "test_dir".to_string(),
                is_dir: true,
                size: 0,
            });
            app.left_panel.state.select(Some(0));
        }

        let backend = crate::WritableTestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend)?;

        let result = handle_edit_impl(&mut terminal, &mut app, |_, _| {
            panic!("Editor should not be called for directories");
        });

        assert!(result.is_ok());
        assert_eq!(app.status_message, "Editing directories is not supported");
        Ok(())
    }

    #[test]
    fn test_handle_edit_impl_success() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.path().join("test.txt");
        File::create(&file_path)?;

        let mut app = create_test_app(&temp_dir)?;
        
        // Find and select the file
        for (i, entry) in app.left_panel.entries.iter().enumerate() {
            if entry.name == "test.txt" {
                app.left_panel.state.select(Some(i));
                break;
            }
        }

        let backend = crate::WritableTestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend)?;

        let result = handle_edit_impl(&mut terminal, &mut app, |editor, path| {
            assert!(editor.contains("nano") || editor == "nano");
            assert_eq!(path, file_path.as_path());
            Ok(std::process::ExitStatus::default())
        });

        assert!(result.is_ok());
        assert!(app.status_message.contains("Edited"));
        assert!(app.status_message.contains("test.txt"));
        Ok(())
    }

    #[test]
    fn test_handle_edit_impl_editor_error() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.path().join("test.txt");
        File::create(&file_path)?;

        let mut app = create_test_app(&temp_dir)?;
        
        // Find and select the file
        for (i, entry) in app.left_panel.entries.iter().enumerate() {
            if entry.name == "test.txt" {
                app.left_panel.state.select(Some(i));
                break;
            }
        }

        let backend = crate::WritableTestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend)?;

        let result = handle_edit_impl(&mut terminal, &mut app, |_, _| {
            Err(io::Error::new(io::ErrorKind::NotFound, "Editor not found"))
        });

        assert!(result.is_ok());
        assert!(app.status_message.contains("Failed to launch editor"));
        assert!(app.status_message.contains("Editor not found"));
        Ok(())
    }

    #[test]
    fn test_handle_edit_impl_editor_exit_code() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.path().join("test.txt");
        File::create(&file_path)?;

        let mut app = create_test_app(&temp_dir)?;
        
        // Find and select the file
        for (i, entry) in app.left_panel.entries.iter().enumerate() {
            if entry.name == "test.txt" {
                app.left_panel.state.select(Some(i));
                break;
            }
        }

        let backend = crate::WritableTestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend)?;

        // Create a mock ExitStatus with error code
        #[cfg(unix)]
        let exit_status = std::os::unix::process::ExitStatusExt::from_raw(256); // Exit code 1
        #[cfg(windows)]
        let exit_status = std::process::ExitStatus::default(); // Windows doesn't have from_raw

        let result = handle_edit_impl(&mut terminal, &mut app, |_, _| {
            Ok(exit_status)
        });

        assert!(result.is_ok());
        #[cfg(unix)]
        assert!(app.status_message.contains("Editor exited with status"));
        Ok(())
    }

    #[test]
    fn test_handle_edit_impl_with_editor_env() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.path().join("test.txt");
        File::create(&file_path)?;

        // Set custom editor
        std::env::set_var("EDITOR", "vim");

        let mut app = create_test_app(&temp_dir)?;
        
        // Find and select the file
        for (i, entry) in app.left_panel.entries.iter().enumerate() {
            if entry.name == "test.txt" {
                app.left_panel.state.select(Some(i));
                break;
            }
        }

        let backend = crate::WritableTestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend)?;

        let result = handle_edit_impl(&mut terminal, &mut app, |editor, _| {
            assert_eq!(editor, "vim");
            Ok(std::process::ExitStatus::default())
        });

        assert!(result.is_ok());
        
        // Clean up
        std::env::remove_var("EDITOR");
        Ok(())
    }

    #[test]
    fn test_handle_edit_impl_right_panel() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.path().join("right.txt");
        File::create(&file_path)?;

        let mut app = create_test_app(&temp_dir)?;
        app.active_panel = PanelSide::Right;
        
        // Find and select the file in right panel
        for (i, entry) in app.right_panel.entries.iter().enumerate() {
            if entry.name == "right.txt" {
                app.right_panel.state.select(Some(i));
                break;
            }
        }

        let backend = crate::WritableTestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend)?;

        let result = handle_edit_impl(&mut terminal, &mut app, |_, path| {
            assert_eq!(path.file_name().unwrap().to_str().unwrap(), "right.txt");
            Ok(std::process::ExitStatus::default())
        });

        assert!(result.is_ok());
        assert!(app.status_message.contains("Edited"));
        Ok(())
    }
}