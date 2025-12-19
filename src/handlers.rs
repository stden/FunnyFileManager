use crossterm::event::KeyCode;
#[cfg(not(test))]
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::Backend;
use ratatui::Terminal;
use std::{
    fs,
    io::{self, Write},
    process::Command,
};

use crate::models::{App, Viewer};
use crate::utils::{copy_dir_all, refresh_panels, unique_destination};

pub fn handle_overlay_keys(app: &mut App, key: KeyCode) -> bool {
    if let Some(viewer) = app.viewer.as_mut() {
        match key {
            KeyCode::Esc | KeyCode::F(3) | KeyCode::Enter => {
                app.viewer = None;
            }
            KeyCode::Down => {
                let max_scroll = viewer
                    .lines
                    .len()
                    .saturating_sub(1)
                    .saturating_sub(viewer.scroll as usize);
                if max_scroll > 0 {
                    viewer.scroll = viewer.scroll.saturating_add(1);
                }
            }
            KeyCode::Up => {
                if viewer.scroll > 0 {
                    viewer.scroll -= 1;
                }
            }
            _ => {}
        }
        return true;
    }

    if app.show_help {
        if matches!(key, KeyCode::Esc | KeyCode::F(1)) {
            app.show_help = false;
        }
        return true;
    }

    false
}

pub fn handle_view(app: &mut App) -> io::Result<()> {
    let Some(selected) = app.get_active_panel().selected_path() else {
        return Ok(());
    };

    if selected.is_dir() {
        app.status_message = "Cannot view a directory".to_string();
        return Ok(());
    }

    let content = fs::read_to_string(&selected)?;
    let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
    if lines.is_empty() {
        lines.push(String::from("<empty file>"));
    }

    app.viewer = Some(Viewer {
        title: selected
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
        lines,
        scroll: 0,
    });
    app.status_message = format!("Viewing {}", selected.display());
    Ok(())
}

pub fn handle_edit<B: Backend + Write>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    #[cfg(test)]
    if std::env::var("TEST_MODE").is_ok() {
        return handle_edit_test_mode(app);
    }

    handle_edit_impl(terminal, app, |editor, path| {
        Command::new(editor).arg(path).status()
    })
}

fn handle_edit_impl<B, F>(
    #[allow(unused_variables)] terminal: &mut Terminal<B>,
    app: &mut App,
    run_editor: F,
) -> io::Result<()>
where
    B: Backend + Write,
    F: FnOnce(&str, &std::path::Path) -> io::Result<std::process::ExitStatus>,
{
    let Some(selected) = app.get_active_panel().selected_path() else {
        return Ok(());
    };

    if selected.is_dir() {
        app.status_message = "Editing directories is not supported".to_string();
        return Ok(());
    }

    // Skip terminal manipulation in test mode
    #[cfg(not(test))]
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

    #[cfg(not(test))]
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

#[cfg(test)]
fn handle_edit_test_mode(app: &mut App) -> io::Result<()> {
    let Some(selected) = app.get_active_panel().selected_path() else {
        return Ok(());
    };

    if selected.is_dir() {
        app.status_message = "Editing directories is not supported".to_string();
        return Ok(());
    }

    let test_editor = std::env::var("TEST_EDITOR").unwrap_or_else(|_| "success".to_string());
    match test_editor.as_str() {
        "fail_launch" => {
            app.status_message =
                "Failed to launch editor: No such file or directory (os error 2)".to_string();
        }
        "exit_error" => {
            app.status_message = "Editor exited with status exit status: 1".to_string();
        }
        _ => {
            app.status_message = format!("Edited {}", selected.display());
        }
    }

    app.left_panel.load_entries()?;
    app.left_panel.ensure_selection();
    app.right_panel.load_entries()?;
    app.right_panel.ensure_selection();
    Ok(())
}

pub fn handle_copy(app: &mut App) -> io::Result<()> {
    let Some(source_path) = app.get_active_panel().selected_path() else {
        return Ok(());
    };
    let target_dir = app.get_inactive_panel().path.clone();

    let dest = unique_destination(&target_dir, source_path.file_name().unwrap_or_default());
    if source_path.is_dir() {
        copy_dir_all(&source_path, &dest)?;
    } else {
        fs::copy(&source_path, &dest)?;
    }

    app.status_message = format!("Copied to {}", dest.display());
    refresh_panels(app)?;
    Ok(())
}

pub fn handle_move(app: &mut App) -> io::Result<()> {
    let Some(source_path) = app.get_active_panel().selected_path() else {
        return Ok(());
    };
    let target_dir = app.get_inactive_panel().path.clone();

    let dest = unique_destination(&target_dir, source_path.file_name().unwrap_or_default());

    match fs::rename(&source_path, &dest) {
        Ok(_) => {}
        Err(_) => {
            if source_path.is_dir() {
                copy_dir_all(&source_path, &dest)?;
                fs::remove_dir_all(&source_path)?;
            } else {
                fs::copy(&source_path, &dest)?;
                fs::remove_file(&source_path)?;
            }
        }
    }

    app.status_message = format!("Moved to {}", dest.display());
    refresh_panels(app)?;
    Ok(())
}

pub fn handle_mkdir(app: &mut App) -> io::Result<()> {
    let base_dir = app.get_active_panel().path.clone();
    let mut counter = 0;
    let mut candidate = base_dir.join("new_folder");
    while candidate.exists() {
        counter += 1;
        candidate = base_dir.join(format!("new_folder_{}", counter));
    }

    fs::create_dir_all(&candidate)?;
    app.status_message = format!("Created directory {}", candidate.display());
    refresh_panels(app)?;
    Ok(())
}

pub fn handle_delete(app: &mut App) -> io::Result<()> {
    let Some(target) = app.get_active_panel().selected_path() else {
        return Ok(());
    };

    if target.is_dir() {
        fs::remove_dir_all(&target)?;
    } else {
        fs::remove_file(&target)?;
    }

    app.status_message = format!("Deleted {}", target.display());
    refresh_panels(app)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{App, Panel, Viewer};
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_handle_overlay_keys_viewer() -> io::Result<()> {
        let mut app = App::new()?;
        
        // Create a viewer with multiple lines
        app.viewer = Some(Viewer {
            title: "test.txt".to_string(),
            lines: vec![
                "Line 1".to_string(),
                "Line 2".to_string(),
                "Line 3".to_string(),
                "Line 4".to_string(),
                "Line 5".to_string(),
            ],
            scroll: 0,
        });
        
        // Test scrolling down
        assert!(handle_overlay_keys(&mut app, KeyCode::Down));
        assert_eq!(app.viewer.as_ref().unwrap().scroll, 1);
        
        // Test scrolling up
        assert!(handle_overlay_keys(&mut app, KeyCode::Up));
        assert_eq!(app.viewer.as_ref().unwrap().scroll, 0);
        
        // Test scrolling up at top (should stay at 0)
        assert!(handle_overlay_keys(&mut app, KeyCode::Up));
        assert_eq!(app.viewer.as_ref().unwrap().scroll, 0);
        
        // Test closing viewer with Esc
        assert!(handle_overlay_keys(&mut app, KeyCode::Esc));
        assert!(app.viewer.is_none());
        
        Ok(())
    }

    #[test]
    fn test_handle_overlay_keys_viewer_close_variations() -> io::Result<()> {
        let mut app = App::new()?;
        
        // Test closing with F3
        app.viewer = Some(Viewer {
            title: "test.txt".to_string(),
            lines: vec!["Line 1".to_string()],
            scroll: 0,
        });
        assert!(handle_overlay_keys(&mut app, KeyCode::F(3)));
        assert!(app.viewer.is_none());
        
        // Test closing with Enter
        app.viewer = Some(Viewer {
            title: "test.txt".to_string(),
            lines: vec!["Line 1".to_string()],
            scroll: 0,
        });
        assert!(handle_overlay_keys(&mut app, KeyCode::Enter));
        assert!(app.viewer.is_none());
        
        Ok(())
    }

    #[test]
    fn test_handle_overlay_keys_viewer_max_scroll() -> io::Result<()> {
        let mut app = App::new()?;
        
        // Create a viewer with limited lines
        app.viewer = Some(Viewer {
            title: "test.txt".to_string(),
            lines: vec!["Line 1".to_string(), "Line 2".to_string()],
            scroll: 0,
        });
        
        // Try to scroll beyond available lines
        for _ in 0..10 {
            handle_overlay_keys(&mut app, KeyCode::Down);
        }
        // Should stop at maximum scroll position
        assert!(app.viewer.as_ref().unwrap().scroll <= 1);
        
        Ok(())
    }

    #[test]
    fn test_handle_overlay_keys_help() -> io::Result<()> {
        let mut app = App::new()?;
        app.show_help = true;
        
        // Test closing help with Esc
        assert!(handle_overlay_keys(&mut app, KeyCode::Esc));
        assert!(!app.show_help);
        
        app.show_help = true;
        // Test closing help with F1
        assert!(handle_overlay_keys(&mut app, KeyCode::F(1)));
        assert!(!app.show_help);
        
        Ok(())
    }

    #[test]
    fn test_handle_overlay_keys_no_overlay() -> io::Result<()> {
        let mut app = App::new()?;
        
        // No viewer or help shown - should return false
        assert!(!handle_overlay_keys(&mut app, KeyCode::Down));
        assert!(!handle_overlay_keys(&mut app, KeyCode::Up));
        assert!(!handle_overlay_keys(&mut app, KeyCode::Esc));
        
        Ok(())
    }

    #[test]
    fn test_handle_view_file() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.path().join("test.txt");
        File::create(&file_path)?.write_all(b"Line 1\nLine 2\nLine 3")?;
        
        let mut app = App::new()?;
        app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
        
        // Select the file
        for (i, entry) in app.left_panel.entries.iter().enumerate() {
            if entry.name == "test.txt" {
                app.left_panel.state.select(Some(i));
                break;
            }
        }
        
        handle_view(&mut app)?;
        
        assert!(app.viewer.is_some());
        let viewer = app.viewer.as_ref().unwrap();
        assert_eq!(viewer.title, "test.txt");
        assert_eq!(viewer.lines.len(), 3);
        assert_eq!(viewer.lines[0], "Line 1");
        assert!(app.status_message.contains("Viewing"));
        
        Ok(())
    }

    #[test]
    fn test_handle_view_empty_file() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.path().join("empty.txt");
        File::create(&file_path)?; // Empty file
        
        let mut app = App::new()?;
        app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
        
        // Select the file
        for (i, entry) in app.left_panel.entries.iter().enumerate() {
            if entry.name == "empty.txt" {
                app.left_panel.state.select(Some(i));
                break;
            }
        }
        
        handle_view(&mut app)?;
        
        assert!(app.viewer.is_some());
        let viewer = app.viewer.as_ref().unwrap();
        assert_eq!(viewer.lines.len(), 1);
        assert_eq!(viewer.lines[0], "<empty file>");
        
        Ok(())
    }

    #[test]
    fn test_handle_view_directory() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        fs::create_dir(temp_dir.path().join("subdir"))?;
        
        let mut app = App::new()?;
        app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
        
        // Select the directory
        for (i, entry) in app.left_panel.entries.iter().enumerate() {
            if entry.name == "subdir" {
                app.left_panel.state.select(Some(i));
                break;
            }
        }
        
        handle_view(&mut app)?;
        
        assert!(app.viewer.is_none());
        assert_eq!(app.status_message, "Cannot view a directory");
        
        Ok(())
    }

    #[test]
    fn test_handle_view_no_selection() -> io::Result<()> {
        let mut app = App::new()?;
        app.left_panel.state.select(None);
        
        // Should not crash
        handle_view(&mut app)?;
        assert!(app.viewer.is_none());
        
        Ok(())
    }

    #[test]
    fn test_handle_edit_success() -> io::Result<()> {
        // Clear any existing env vars
        std::env::remove_var("TEST_EDITOR");
        // Set test mode
        std::env::set_var("TEST_MODE", "1");
        
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.path().join("test.txt");
        File::create(&file_path)?;
        
        let mut app = App::new()?;
        app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
        
        // Select the file
        for (i, entry) in app.left_panel.entries.iter().enumerate() {
            if entry.name == "test.txt" {
                app.left_panel.state.select(Some(i));
                break;
            }
        }
        
        // Since handle_edit requires a terminal in non-test mode,
        // we use the test mode handler directly
        handle_edit_test_mode(&mut app)?;
        assert!(app.status_message.contains("Edited"));
        
        std::env::remove_var("TEST_MODE");
        Ok(())
    }

    #[test]
    fn test_handle_edit_fail_launch() -> io::Result<()> {
        // Clear any existing env vars
        std::env::remove_var("TEST_EDITOR");
        // Set test mode
        std::env::set_var("TEST_MODE", "1");
        std::env::set_var("TEST_EDITOR", "fail_launch");
        
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.path().join("test.txt");
        File::create(&file_path)?;
        
        let mut app = App::new()?;
        app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
        
        // Select the file
        for (i, entry) in app.left_panel.entries.iter().enumerate() {
            if entry.name == "test.txt" {
                app.left_panel.state.select(Some(i));
                break;
            }
        }
        
        // Since handle_edit requires a terminal in non-test mode,
        // we use the test mode handler directly
        handle_edit_test_mode(&mut app)?;
        assert!(app.status_message.contains("Failed to launch editor"));
        
        std::env::remove_var("TEST_MODE");
        std::env::remove_var("TEST_EDITOR");
        Ok(())
    }

    #[test]
    fn test_handle_edit_exit_error() -> io::Result<()> {
        // Clear any existing env vars
        std::env::remove_var("TEST_EDITOR");
        // Set test mode
        std::env::set_var("TEST_MODE", "1");
        std::env::set_var("TEST_EDITOR", "exit_error");
        
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.path().join("test.txt");
        File::create(&file_path)?;
        
        let mut app = App::new()?;
        app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
        
        // Select the file
        for (i, entry) in app.left_panel.entries.iter().enumerate() {
            if entry.name == "test.txt" {
                app.left_panel.state.select(Some(i));
                break;
            }
        }
        
        // Since handle_edit requires a terminal in non-test mode,
        // we use the test mode handler directly
        handle_edit_test_mode(&mut app)?;
        assert!(app.status_message.contains("Editor exited with status"));
        
        std::env::remove_var("TEST_MODE");
        std::env::remove_var("TEST_EDITOR");
        Ok(())
    }

    #[test]
    fn test_handle_edit_directory() -> io::Result<()> {
        // Clear any existing env vars
        std::env::remove_var("TEST_EDITOR");
        // Set test mode
        std::env::set_var("TEST_MODE", "1");
        
        let temp_dir = TempDir::new()?;
        fs::create_dir(temp_dir.path().join("subdir"))?;
        
        let mut app = App::new()?;
        app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
        
        // Select the directory
        for (i, entry) in app.left_panel.entries.iter().enumerate() {
            if entry.name == "subdir" {
                app.left_panel.state.select(Some(i));
                break;
            }
        }
        
        // Since handle_edit requires a terminal in non-test mode,
        // we use the test mode handler directly
        handle_edit_test_mode(&mut app)?;
        assert_eq!(app.status_message, "Editing directories is not supported");
        
        std::env::remove_var("TEST_MODE");
        Ok(())
    }

    #[test]
    fn test_handle_copy_directory() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let left_dir = temp_dir.path().join("left");
        let right_dir = temp_dir.path().join("right");
        fs::create_dir(&left_dir)?;
        fs::create_dir(&right_dir)?;
        
        // Create a directory with contents
        let source_dir = left_dir.join("source_dir");
        fs::create_dir(&source_dir)?;
        File::create(source_dir.join("file.txt"))?.write_all(b"content")?;
        
        let mut app = App::new()?;
        app.left_panel = Panel::new(left_dir)?;
        app.right_panel = Panel::new(right_dir.clone())?;
        
        // Select the directory
        for (i, entry) in app.left_panel.entries.iter().enumerate() {
            if entry.name == "source_dir" {
                app.left_panel.state.select(Some(i));
                break;
            }
        }
        
        handle_copy(&mut app)?;
        
        assert!(right_dir.join("source_dir").exists());
        assert!(right_dir.join("source_dir").join("file.txt").exists());
        assert!(app.status_message.contains("Copied to"));
        
        Ok(())
    }

    #[test]
    fn test_handle_move_cross_device() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let left_dir = temp_dir.path().join("left");
        let right_dir = temp_dir.path().join("right");
        fs::create_dir(&left_dir)?;
        fs::create_dir(&right_dir)?;
        
        // Create a directory to move
        let source_dir = left_dir.join("move_dir");
        fs::create_dir(&source_dir)?;
        File::create(source_dir.join("file.txt"))?.write_all(b"content")?;
        
        let mut app = App::new()?;
        app.left_panel = Panel::new(left_dir.clone())?;
        app.right_panel = Panel::new(right_dir.clone())?;
        
        // Select the directory
        for (i, entry) in app.left_panel.entries.iter().enumerate() {
            if entry.name == "move_dir" {
                app.left_panel.state.select(Some(i));
                break;
            }
        }
        
        handle_move(&mut app)?;
        
        assert!(!source_dir.exists());
        assert!(right_dir.join("move_dir").exists());
        assert!(right_dir.join("move_dir").join("file.txt").exists());
        assert!(app.status_message.contains("Moved to"));
        
        Ok(())
    }

    #[test]
    fn test_handle_mkdir_multiple() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        
        let mut app = App::new()?;
        app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
        
        // Create first directory
        handle_mkdir(&mut app)?;
        assert!(temp_dir.path().join("new_folder").exists());
        
        // Create second directory (should be numbered)
        handle_mkdir(&mut app)?;
        assert!(temp_dir.path().join("new_folder_1").exists());
        
        // Create third directory
        handle_mkdir(&mut app)?;
        assert!(temp_dir.path().join("new_folder_2").exists());
        
        Ok(())
    }

    #[test]
    fn test_handle_delete_no_selection() -> io::Result<()> {
        let mut app = App::new()?;
        app.left_panel.state.select(None);
        
        // Should not crash
        handle_delete(&mut app)?;
        
        Ok(())
    }

    #[test]
    fn test_handle_copy_no_selection() -> io::Result<()> {
        let mut app = App::new()?;
        app.left_panel.state.select(None);
        
        // Should not crash
        handle_copy(&mut app)?;
        
        Ok(())
    }

    #[test]
    fn test_handle_move_no_selection() -> io::Result<()> {
        let mut app = App::new()?;
        app.left_panel.state.select(None);
        
        // Should not crash
        handle_move(&mut app)?;
        
        Ok(())
    }

    #[test]
    fn test_handle_overlay_keys_viewer_other_key() -> io::Result<()> {
        let mut app = App::new()?;
        app.viewer = Some(Viewer {
            title: "test.txt".to_string(),
            lines: vec!["Line 1".to_string()],
            scroll: 0,
        });
        
        // Test with a key that doesn't do anything in viewer
        assert!(handle_overlay_keys(&mut app, KeyCode::Char('a')));
        // Viewer should still be active
        assert!(app.viewer.is_some());
        
        Ok(())
    }

    #[test]
    fn test_handle_overlay_keys_help_other_key() -> io::Result<()> {
        let mut app = App::new()?;
        app.show_help = true;
        
        // Test with a key that doesn't close help
        assert!(handle_overlay_keys(&mut app, KeyCode::Down));
        // Help should still be shown
        assert!(app.show_help);
        
        Ok(())
    }

    #[test]
    fn test_handle_edit_no_selection() -> io::Result<()> {
        // Clear any existing env vars
        std::env::remove_var("TEST_EDITOR");
        // Set test mode
        std::env::set_var("TEST_MODE", "1");
        
        let mut app = App::new()?;
        app.left_panel.state.select(None);
        
        // Should not crash
        handle_edit_test_mode(&mut app)?;
        
        std::env::remove_var("TEST_MODE");
        Ok(())
    }

    // Create a wrapper for TestBackend that implements Write
    struct WritableTestBackend {
        inner: ratatui::backend::TestBackend,
    }

    impl WritableTestBackend {
        fn new(width: u16, height: u16) -> Self {
            Self {
                inner: ratatui::backend::TestBackend::new(width, height),
            }
        }
    }

    impl Write for WritableTestBackend {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl ratatui::backend::Backend for WritableTestBackend {
        fn draw<'a, I>(&mut self, content: I) -> io::Result<()>
        where
            I: Iterator<Item = (u16, u16, &'a ratatui::buffer::Cell)>,
        {
            self.inner.draw(content)
        }

        fn hide_cursor(&mut self) -> io::Result<()> {
            self.inner.hide_cursor()
        }

        fn show_cursor(&mut self) -> io::Result<()> {
            self.inner.show_cursor()
        }

        fn get_cursor(&mut self) -> io::Result<(u16, u16)> {
            self.inner.get_cursor()
        }

        fn set_cursor(&mut self, x: u16, y: u16) -> io::Result<()> {
            self.inner.set_cursor(x, y)
        }

        fn clear(&mut self) -> io::Result<()> {
            self.inner.clear()
        }

        fn size(&self) -> io::Result<ratatui::layout::Rect> {
            self.inner.size()
        }

        fn flush(&mut self) -> io::Result<()> {
            self.inner.flush()
        }

        fn window_size(&mut self) -> io::Result<ratatui::backend::WindowSize> {
            let size = self.size()?;
            Ok(ratatui::backend::WindowSize {
                columns_rows: ratatui::layout::Size {
                    width: size.width,
                    height: size.height,
                },
                pixels: ratatui::layout::Size {
                    width: size.width * 8,
                    height: size.height * 16,
                },
            })
        }
    }

    #[test]
    fn test_handle_edit_impl_success() -> io::Result<()> {
        use ratatui::Terminal;
        use std::process::Command;
        
        let backend = WritableTestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend)?;
        
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.path().join("test.txt");
        File::create(&file_path)?;
        
        let mut app = App::new()?;
        app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
        
        // Select the file
        for (i, entry) in app.left_panel.entries.iter().enumerate() {
            if entry.name == "test.txt" {
                app.left_panel.state.select(Some(i));
                break;
            }
        }
        
        // Mock successful editor run
        let result = handle_edit_impl(&mut terminal, &mut app, |_editor, _path| {
            // Simulate successful editor exit
            Command::new("true").status()
        });
        
        if let Err(e) = &result {
            panic!("handle_edit_impl failed with error: {:?}", e);
        }
        assert!(result.is_ok());
        assert!(app.status_message.contains("Edited"));
        
        Ok(())
    }

    #[test]
    fn test_handle_edit_impl_editor_error_exit() -> io::Result<()> {
        use ratatui::Terminal;
        use std::process::Command;
        
        let backend = WritableTestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend)?;
        
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.path().join("test.txt");
        File::create(&file_path)?;
        
        let mut app = App::new()?;
        app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
        
        // Select the file
        for (i, entry) in app.left_panel.entries.iter().enumerate() {
            if entry.name == "test.txt" {
                app.left_panel.state.select(Some(i));
                break;
            }
        }
        
        // Mock editor with error exit code
        let result = handle_edit_impl(&mut terminal, &mut app, |_editor, _path| {
            // Simulate editor exiting with error
            Command::new("false").status()
        });
        
        assert!(result.is_ok());
        assert!(app.status_message.contains("Editor exited with status"));
        
        Ok(())
    }

    #[test]
    fn test_handle_edit_impl_launch_failure() -> io::Result<()> {
        use ratatui::Terminal;
        
        let backend = WritableTestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend)?;
        
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.path().join("test.txt");
        File::create(&file_path)?;
        
        let mut app = App::new()?;
        app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
        
        // Select the file
        for (i, entry) in app.left_panel.entries.iter().enumerate() {
            if entry.name == "test.txt" {
                app.left_panel.state.select(Some(i));
                break;
            }
        }
        
        // Mock editor launch failure
        let result = handle_edit_impl(&mut terminal, &mut app, |_editor, _path| {
            Err(io::Error::new(io::ErrorKind::NotFound, "Editor not found"))
        });
        
        assert!(result.is_ok());
        assert!(app.status_message.contains("Failed to launch editor"));
        
        Ok(())
    }

    #[test]
    fn test_handle_edit_impl_no_selection() -> io::Result<()> {
        use ratatui::Terminal;
        
        let backend = WritableTestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend)?;
        
        let mut app = App::new()?;
        app.left_panel.state.select(None);
        
        // Should return Ok without doing anything
        let result = handle_edit_impl(&mut terminal, &mut app, |_editor, _path| {
            panic!("Editor should not be called when no file is selected");
        });
        
        assert!(result.is_ok());
        
        Ok(())
    }

    #[test]
    fn test_handle_edit_impl_directory() -> io::Result<()> {
        use ratatui::Terminal;
        
        let backend = WritableTestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend)?;
        
        let temp_dir = TempDir::new()?;
        fs::create_dir(temp_dir.path().join("subdir"))?;
        
        let mut app = App::new()?;
        app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
        
        // Select the directory
        for (i, entry) in app.left_panel.entries.iter().enumerate() {
            if entry.name == "subdir" {
                app.left_panel.state.select(Some(i));
                break;
            }
        }
        
        // Should return Ok without launching editor
        let result = handle_edit_impl(&mut terminal, &mut app, |_editor, _path| {
            panic!("Editor should not be called for directories");
        });
        
        assert!(result.is_ok());
        assert_eq!(app.status_message, "Editing directories is not supported");
        
        Ok(())
    }
}
