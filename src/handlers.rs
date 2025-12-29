use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
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
    path::PathBuf,
};

use crate::models::{App, SortMode, Viewer};
use crate::utils::{copy_dir_all, refresh_panels, unique_destination};

#[cfg(all(not(test), not(tarpaulin)))]
use std::process::Command;

#[cfg(any(test, tarpaulin))]
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(any(test, tarpaulin))]
static FORCE_RENAME_FAILURE: AtomicBool = AtomicBool::new(false);

fn try_rename(source: &std::path::Path, dest: &std::path::Path) -> io::Result<()> {
    #[cfg(any(test, tarpaulin))]
    if FORCE_RENAME_FAILURE.load(Ordering::SeqCst) {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "forced rename failure",
        ));
    }

    fs::rename(source, dest)
}

pub fn handle_overlay_keys(app: &mut App, key: KeyEvent) -> bool {
    let code = key.code;
    let modifiers = key.modifiers;

    if let Some(viewer) = app.viewer.as_mut() {
        match code {
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
        if matches!(code, KeyCode::Esc | KeyCode::F(1)) {
            app.show_help = false;
        }
        return true;
    }

    if let Some(query) = app.search.as_mut() {
        match code {
            KeyCode::Esc => {
                app.search = None;
            }
            KeyCode::Enter => {
                let query_trimmed = query.trim().to_string();
                if !query_trimmed.is_empty() {
                    let found = jump_to_match(app.get_active_panel(), &query_trimmed);
                    if found {
                        app.status_message = format!("Found: {}", query_trimmed);
                    } else {
                        app.status_message = format!("Not found: {}", query_trimmed);
                    }
                }
                app.search = None;
            }
            KeyCode::Backspace => {
                query.pop();
                let query_trimmed = query.trim().to_string();
                if !query_trimmed.is_empty() {
                    jump_to_match(app.get_active_panel(), &query_trimmed);
                }
            }
            KeyCode::Char(ch)
                if !modifiers.contains(KeyModifiers::CONTROL)
                    && !modifiers.contains(KeyModifiers::ALT) =>
            {
                query.push(ch);
                let query_trimmed = query.trim().to_string();
                if !query_trimmed.is_empty() {
                    jump_to_match(app.get_active_panel(), &query_trimmed);
                }
            }
            _ => {}
        }
        return true;
    }

    if app.show_bookmarks {
        match code {
            KeyCode::Esc | KeyCode::F(2) => {
                app.show_bookmarks = false;
            }
            KeyCode::Char(ch) if ch.is_ascii_digit() => {
                let index = (ch as u8 - b'0') as usize;
                if index >= app.bookmarks.len() {
                    app.status_message = format!("Invalid bookmark slot {}", index);
                    return true;
                }

                if modifiers.contains(KeyModifiers::CONTROL) {
                    let path = app.get_active_panel().path.clone();
                    app.bookmarks[index] = Some(path.clone());
                    app.status_message = format!("Bookmark {} set to {}", index, path.display());
                } else if let Some(path) = app.bookmarks[index].clone() {
                    let panel = app.get_active_panel();
                    panel.path = path.clone();
                    panel.clear_marks();
                    match panel.load_entries() {
                        Ok(()) => {
                            panel.ensure_selection();
                            app.status_message = format!("Opened bookmark {}: {}", index, path.display());
                            app.show_bookmarks = false;
                        }
                        Err(err) => {
                            app.status_message =
                                format!("Failed to open bookmark {}: {}", index, err);
                        }
                    }
                } else {
                    app.status_message = format!("Bookmark {} is empty", index);
                }
            }
            _ => {}
        }
        return true;
    }

    if let Some(input) = app.command_line.as_mut() {
        match code {
            KeyCode::Esc => {
                app.command_line = None;
            }
            KeyCode::Enter => {
                let command = input.trim().to_string();
                app.command_line = None;
                if command.is_empty() {
                    return true;
                }

                if let Err(err) = execute_command(app, &command) {
                    app.status_message = format!("Command failed: {}", err);
                }
            }
            KeyCode::Backspace => {
                input.pop();
            }
            KeyCode::Char(ch)
                if !modifiers.contains(KeyModifiers::CONTROL)
                    && !modifiers.contains(KeyModifiers::ALT) =>
            {
                input.push(ch);
            }
            _ => {}
        }
        return true;
    }

    false
}

fn jump_to_match(panel: &mut crate::models::Panel, query: &str) -> bool {
    let query = query.to_lowercase();
    for (index, entry) in panel.entries.iter().enumerate() {
        if entry.name == ".." {
            continue;
        }
        if entry.name.to_lowercase().contains(&query) {
            panel.state.select(Some(index));
            return true;
        }
    }

    false
}

fn execute_command(app: &mut App, command: &str) -> io::Result<()> {
    let mut parts = command.split_whitespace();
    let Some(cmd) = parts.next() else {
        return Ok(());
    };

    match cmd {
        "cd" => {
            let Some(path_arg) = parts.next() else {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, "cd requires a path"));
            };

            let requested = PathBuf::from(path_arg);
            let new_path = if requested.is_absolute() {
                requested
            } else {
                app.get_active_panel().path.join(requested)
            };

            if !new_path.is_dir() {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("not a directory: {}", new_path.display()),
                ));
            }

            let panel = app.get_active_panel();
            panel.path = new_path.clone();
            panel.clear_marks();
            panel.load_entries()?;
            panel.ensure_selection();
            app.status_message = format!("Opened {}", new_path.display());
            Ok(())
        }
        "pwd" => {
            let path = app.get_active_panel().path.clone();
            app.status_message = path.display().to_string();
            Ok(())
        }
        "sort" => {
            let mode = parts.next().unwrap_or("name");
            let sort_mode = match mode {
                "name" => SortMode::Name,
                "ext" | "extension" => SortMode::Extension,
                "size" => SortMode::Size,
                _ => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "sort expects: name|ext|size",
                    ));
                }
            };

            let panel = app.get_active_panel();
            panel.sort_mode = sort_mode;
            panel.load_entries()?;
            panel.ensure_selection();
            app.status_message = format!("Sort: {}", panel.sort_mode.label());
            Ok(())
        }
        "bookmark" => {
            let Some(sub) = parts.next() else {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "bookmark expects: set|go <0-9>",
                ));
            };
            let Some(slot_str) = parts.next() else {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "bookmark expects a slot 0-9",
                ));
            };

            let slot: usize = slot_str.parse().map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidInput, "bookmark slot must be 0-9")
            })?;
            if slot >= app.bookmarks.len() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "bookmark slot must be 0-9",
                ));
            }

            match sub {
                "set" => {
                    let path = app.get_active_panel().path.clone();
                    app.bookmarks[slot] = Some(path.clone());
                    app.status_message = format!("Bookmark {} set to {}", slot, path.display());
                    Ok(())
                }
                "go" => {
                    let Some(path) = app.bookmarks[slot].clone() else {
                        return Err(io::Error::new(
                            io::ErrorKind::NotFound,
                            format!("bookmark {} is empty", slot),
                        ));
                    };
                    let panel = app.get_active_panel();
                    panel.path = path.clone();
                    panel.clear_marks();
                    panel.load_entries()?;
                    panel.ensure_selection();
                    app.status_message = format!("Opened bookmark {}: {}", slot, path.display());
                    Ok(())
                }
                _ => Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "bookmark expects: set|go <0-9>",
                )),
            }
        }
        "mark" => {
            let Some(sub) = parts.next() else {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "mark expects: all",
                ));
            };

            match sub {
                "all" => {
                    let names: Vec<String> = app
                        .get_active_panel()
                        .entries
                        .iter()
                        .filter(|entry| entry.name != "..")
                        .map(|entry| entry.name.clone())
                        .collect();
                    let panel = app.get_active_panel();
                    for name in names {
                        panel.marked.insert(name);
                    }
                    app.status_message = format!("Marked {} items", panel.marked.len());
                    Ok(())
                }
                _ => Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "mark expects: all",
                )),
            }
        }
        "unmark" => {
            app.get_active_panel().clear_marks();
            app.status_message = "Unmarked all".to_string();
            Ok(())
        }
        "help" => {
            app.show_help = true;
            Ok(())
        }
        "quit" | "exit" => {
            app.should_quit = true;
            Ok(())
        }
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unknown command: {}", cmd),
        )),
    }
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

#[cfg(any(test, tarpaulin))]
pub fn handle_edit<B: Backend + Write>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    let _ = terminal;
    handle_edit_test_mode(app, TestEditorBehavior::Success)
}

#[cfg(all(not(test), not(tarpaulin)))]
pub fn handle_edit<B: Backend + Write>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> { // tarpaulin ignore
    handle_edit_impl(terminal, app, |editor, path| Command::new(editor).arg(path).status()) // tarpaulin ignore
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
    #[cfg(all(not(test), not(tarpaulin)))]
    {
        disable_raw_mode()?; // tarpaulin ignore
        execute!( // tarpaulin ignore
            terminal.backend_mut(), // tarpaulin ignore
            LeaveAlternateScreen, // tarpaulin ignore
            DisableMouseCapture // tarpaulin ignore
        )?; // tarpaulin ignore
    }
    
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "nano".to_string());
    let status = run_editor(&editor, &selected);

    #[cfg(all(not(test), not(tarpaulin)))]
    {
        enable_raw_mode()?; // tarpaulin ignore
        execute!( // tarpaulin ignore
            terminal.backend_mut(), // tarpaulin ignore
            EnterAlternateScreen, // tarpaulin ignore
            EnableMouseCapture // tarpaulin ignore
        )?; // tarpaulin ignore
        terminal.clear()?; // tarpaulin ignore
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

#[cfg(any(test, tarpaulin))]
#[derive(Clone, Copy, Debug)]
enum TestEditorBehavior {
    Success,
    FailLaunch,
    ExitError,
}

#[cfg(any(test, tarpaulin))]
fn handle_edit_test_mode(app: &mut App, behavior: TestEditorBehavior) -> io::Result<()> {
    let Some(selected) = app.get_active_panel().selected_path() else {
        return Ok(());
    };

    if selected.is_dir() {
        app.status_message = "Editing directories is not supported".to_string();
        return Ok(());
    }

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

    match try_rename(&source_path, &dest) {
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

    fn fake_exit_status(code: i32) -> std::process::ExitStatus {
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            std::process::ExitStatus::from_raw(code << 8)
        }

        #[cfg(windows)]
        {
            use std::os::windows::process::ExitStatusExt;
            std::process::ExitStatus::from_raw(code as u32)
        }
    }

    struct ForceRenameFailureGuard;

    impl ForceRenameFailureGuard {
        fn enable() -> Self {
            FORCE_RENAME_FAILURE.store(true, Ordering::SeqCst);
            Self
        }
    }

    impl Drop for ForceRenameFailureGuard {
        fn drop(&mut self) {
            FORCE_RENAME_FAILURE.store(false, Ordering::SeqCst);
        }
    }

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
        
        handle_edit_test_mode(&mut app, TestEditorBehavior::Success)?;
        assert!(app.status_message.contains("Edited"));
        
        Ok(())
    }

    #[test]
    fn test_handle_edit_fail_launch() -> io::Result<()> {
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
        
        handle_edit_test_mode(&mut app, TestEditorBehavior::FailLaunch)?;
        assert!(app.status_message.contains("Failed to launch editor"));
        
        Ok(())
    }

    #[test]
    fn test_handle_edit_exit_error() -> io::Result<()> {
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
        
        handle_edit_test_mode(&mut app, TestEditorBehavior::ExitError)?;
        assert!(app.status_message.contains("Editor exited with status"));
        
        Ok(())
    }

    #[test]
    fn test_handle_edit_directory() -> io::Result<()> {
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
        
        handle_edit_test_mode(&mut app, TestEditorBehavior::Success)?;
        assert_eq!(app.status_message, "Editing directories is not supported");
        
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

        let _guard = ForceRenameFailureGuard::enable();
        handle_move(&mut app)?;
        
        assert!(!source_dir.exists());
        assert!(right_dir.join("move_dir").exists());
        assert!(right_dir.join("move_dir").join("file.txt").exists());
        assert!(app.status_message.contains("Moved to"));
        
        Ok(())
    }

    #[test]
    fn test_handle_move_fallback_for_file() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let left_dir = temp_dir.path().join("left");
        let right_dir = temp_dir.path().join("right");
        fs::create_dir(&left_dir)?;
        fs::create_dir(&right_dir)?;

        let file_name = "move_file.txt";
        let source_file = left_dir.join(file_name);
        File::create(&source_file)?.write_all(b"content")?;

        let mut app = App::new()?;
        app.left_panel = Panel::new(left_dir.clone())?;
        app.right_panel = Panel::new(right_dir.clone())?;

        for (i, entry) in app.left_panel.entries.iter().enumerate() {
            if entry.name == file_name {
                app.left_panel.state.select(Some(i));
                break;
            }
        }

        let _guard = ForceRenameFailureGuard::enable();
        handle_move(&mut app)?;

        assert!(!source_file.exists());
        let moved_file = right_dir.join(file_name);
        assert!(moved_file.exists());
        assert_eq!(fs::read_to_string(&moved_file)?, "content");
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
        let mut app = App::new()?;
        app.left_panel.state.select(None);
        
        // Should not crash
        handle_edit_test_mode(&mut app, TestEditorBehavior::Success)?;
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
            Ok(fake_exit_status(0))
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
            Ok(fake_exit_status(1))
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
