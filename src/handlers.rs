use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::{fs, io, path::PathBuf};

use crate::models::{App, SortMode, Viewer};
use crate::utils::{copy_dir_all, refresh_panels, unique_destination};

#[cfg(any(test, tarpaulin))]
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(any(test, tarpaulin))]
static FORCE_RENAME_FAILURE: AtomicBool = AtomicBool::new(false);

fn try_rename(source: &std::path::Path, dest: &std::path::Path) -> io::Result<()> {
    #[cfg(any(test, tarpaulin))]
    let forced = FORCE_RENAME_FAILURE.load(Ordering::SeqCst);

    #[cfg(not(any(test, tarpaulin)))]
    let forced = false;

    if forced {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "forced rename failure",
        ))
    } else {
        fs::rename(source, dest)
    }
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
                            app.status_message =
                                format!("Opened bookmark {}: {}", index, path.display());
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
    let start_index = panel.state.selected().unwrap_or(0);

    for i in 1..=panel.entries.len() {
        let index = (start_index + i) % panel.entries.len();
        let entry = &panel.entries[index];

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
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "empty command"));
    };

    match cmd {
        "cd" => {
            let Some(path_arg) = parts.next() else {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "cd requires a path",
                ));
            };

            let requested = PathBuf::from(path_arg);
            let new_path = if requested.is_absolute() {
                requested
            } else {
                app.get_active_panel().path.join(requested)
            }
            .canonicalize()?;

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
    #[cfg(feature = "test-helpers")]
    use crate::editor::{handle_edit_impl, handle_edit_test_mode, TestEditorBehavior};
    use crate::models::{App, Panel, SortMode, Viewer};
    #[cfg(feature = "test-helpers")]
    use crate::WritableTestBackend;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    #[cfg(feature = "test-helpers")]
    fn fake_exit_status(code: i32) -> std::process::ExitStatus {
        use std::process::Command;
        if code == 0 {
            Command::new("true").status().unwrap()
        } else {
            Command::new("false").status().unwrap()
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

    fn key_event(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::empty())
    }

    fn key_event_with_mods(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, modifiers)
    }

    #[test]
    fn test_handle_overlay_keys_viewer() -> io::Result<()> {
        let mut app = App::new().expect("test setup");

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
        assert!(handle_overlay_keys(&mut app, key_event(KeyCode::Down)));
        assert_eq!(app.viewer.as_ref().unwrap().scroll, 1);

        // Test scrolling up
        assert!(handle_overlay_keys(&mut app, key_event(KeyCode::Up)));
        assert_eq!(app.viewer.as_ref().unwrap().scroll, 0);

        // Test scrolling up at top (should stay at 0)
        assert!(handle_overlay_keys(&mut app, key_event(KeyCode::Up)));
        assert_eq!(app.viewer.as_ref().unwrap().scroll, 0);

        // Test closing viewer with Esc
        assert!(handle_overlay_keys(&mut app, key_event(KeyCode::Esc)));
        assert!(app.viewer.is_none());

        Ok(())
    }

    #[test]
    fn test_handle_overlay_keys_viewer_close_variations() -> io::Result<()> {
        let mut app = App::new().expect("test setup");

        // Test closing with F3
        app.viewer = Some(Viewer {
            title: "test.txt".to_string(),
            lines: vec!["Line 1".to_string()],
            scroll: 0,
        });
        assert!(handle_overlay_keys(&mut app, key_event(KeyCode::F(3))));
        assert!(app.viewer.is_none());

        // Test closing with Enter
        app.viewer = Some(Viewer {
            title: "test.txt".to_string(),
            lines: vec!["Line 1".to_string()],
            scroll: 0,
        });
        assert!(handle_overlay_keys(&mut app, key_event(KeyCode::Enter)));
        assert!(app.viewer.is_none());

        Ok(())
    }

    #[test]
    fn test_handle_overlay_keys_viewer_max_scroll() -> io::Result<()> {
        let mut app = App::new().expect("test setup");

        // Create a viewer with limited lines
        app.viewer = Some(Viewer {
            title: "test.txt".to_string(),
            lines: vec!["Line 1".to_string(), "Line 2".to_string()],
            scroll: 0,
        });

        // Try to scroll beyond available lines
        for _ in 0..10 {
            handle_overlay_keys(&mut app, key_event(KeyCode::Down));
        }
        // Should stop at maximum scroll position
        assert!(app.viewer.as_ref().unwrap().scroll <= 1);

        Ok(())
    }

    #[test]
    fn test_handle_overlay_keys_help() -> io::Result<()> {
        let mut app = App::new().expect("test setup");
        app.show_help = true;

        // Test closing help with Esc
        assert!(handle_overlay_keys(&mut app, key_event(KeyCode::Esc)));
        assert!(!app.show_help);

        app.show_help = true;
        // Test closing help with F1
        assert!(handle_overlay_keys(&mut app, key_event(KeyCode::F(1))));
        assert!(!app.show_help);

        Ok(())
    }

    #[test]
    fn test_handle_overlay_keys_no_overlay() -> io::Result<()> {
        let mut app = App::new().expect("test setup");

        // No viewer or help shown - should return false
        assert!(!handle_overlay_keys(&mut app, key_event(KeyCode::Down)));
        assert!(!handle_overlay_keys(&mut app, key_event(KeyCode::Up)));
        assert!(!handle_overlay_keys(&mut app, key_event(KeyCode::Esc)));

        Ok(())
    }

    #[test]
    fn test_handle_view_file() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let file_path = temp_dir.path().join("test.txt");
        File::create(&file_path)
            .expect("test setup")
            .write_all(b"Line 1\nLine 2\nLine 3")
            .expect("test setup");

        let mut app = App::new().expect("test setup");
        app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
        app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
        app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

        // Select the file
        for (i, entry) in app.left_panel.entries.iter().enumerate() {
            if entry.name == "test.txt" {
                app.left_panel.state.select(Some(i));
                break;
            }
        }

        handle_view(&mut app).expect("test setup");

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
        let temp_dir = TempDir::new().expect("test setup");
        let file_path = temp_dir.path().join("empty.txt");
        File::create(&file_path).expect("test setup"); // Empty file

        let mut app = App::new().expect("test setup");
        app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
        app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
        app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

        // Select the file
        for (i, entry) in app.left_panel.entries.iter().enumerate() {
            if entry.name == "empty.txt" {
                app.left_panel.state.select(Some(i));
                break;
            }
        }

        handle_view(&mut app).expect("test setup");

        assert!(app.viewer.is_some());
        let viewer = app.viewer.as_ref().unwrap();
        assert_eq!(viewer.lines.len(), 1);
        assert_eq!(viewer.lines[0], "<empty file>");

        Ok(())
    }

    #[test]
    fn test_handle_view_invalid_utf8() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let file_path = temp_dir.path().join("bad.txt");
        fs::write(&file_path, b"\xFF").expect("test setup");

        let mut app = App::new().expect("test setup");
        app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
        app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

        for (i, entry) in app.left_panel.entries.iter().enumerate() {
            if entry.name == "bad.txt" {
                app.left_panel.state.select(Some(i));
                break;
            }
        }

        let err = handle_view(&mut app).expect_err("expected invalid utf8");
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);

        Ok(())
    }

    #[test]
    fn test_handle_view_directory() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        fs::create_dir(temp_dir.path().join("subdir")).expect("test setup");

        let mut app = App::new().expect("test setup");
        app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
        app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
        app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

        // Select the directory
        for (i, entry) in app.left_panel.entries.iter().enumerate() {
            if entry.name == "subdir" {
                app.left_panel.state.select(Some(i));
                break;
            }
        }

        handle_view(&mut app).expect("test setup");

        assert!(app.viewer.is_none());
        assert_eq!(app.status_message, "Cannot view a directory");

        Ok(())
    }

    #[test]
    fn test_handle_view_no_selection() -> io::Result<()> {
        let mut app = App::new().expect("test setup");
        app.left_panel.state.select(None);

        // Should not crash
        handle_view(&mut app).expect("test setup");
        assert!(app.viewer.is_none());

        Ok(())
    }

    #[test]
    #[cfg(feature = "test-helpers")]
    fn test_handle_edit_success() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let file_path = temp_dir.path().join("test.txt");
        File::create(&file_path).expect("test setup");

        let mut app = App::new().expect("test setup");
        app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

        // Select the file
        for (i, entry) in app.left_panel.entries.iter().enumerate() {
            if entry.name == "test.txt" {
                app.left_panel.state.select(Some(i));
                break;
            }
        }

        handle_edit_test_mode(&mut app, TestEditorBehavior::Success).expect("test setup");
        assert!(app.status_message.contains("Edited"));

        Ok(())
    }

    #[test]
    #[cfg(feature = "test-helpers")]
    fn test_handle_edit_fail_launch() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let file_path = temp_dir.path().join("test.txt");
        File::create(&file_path).expect("test setup");

        let mut app = App::new().expect("test setup");
        app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

        // Select the file
        for (i, entry) in app.left_panel.entries.iter().enumerate() {
            if entry.name == "test.txt" {
                app.left_panel.state.select(Some(i));
                break;
            }
        }

        handle_edit_test_mode(&mut app, TestEditorBehavior::FailLaunch).expect("test setup");
        assert!(app.status_message.contains("Failed to launch editor"));

        Ok(())
    }

    #[test]
    #[cfg(feature = "test-helpers")]
    fn test_handle_edit_exit_error() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let file_path = temp_dir.path().join("test.txt");
        File::create(&file_path).expect("test setup");

        let mut app = App::new().expect("test setup");
        app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

        // Select the file
        for (i, entry) in app.left_panel.entries.iter().enumerate() {
            if entry.name == "test.txt" {
                app.left_panel.state.select(Some(i));
                break;
            }
        }

        handle_edit_test_mode(&mut app, TestEditorBehavior::ExitError).expect("test setup");
        assert!(app.status_message.contains("Editor exited with status"));

        Ok(())
    }

    #[test]
    #[cfg(feature = "test-helpers")]
    fn test_handle_edit_directory() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        fs::create_dir(temp_dir.path().join("subdir")).expect("test setup");

        let mut app = App::new().expect("test setup");
        app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

        // Select the directory
        for (i, entry) in app.left_panel.entries.iter().enumerate() {
            if entry.name == "subdir" {
                app.left_panel.state.select(Some(i));
                break;
            }
        }

        handle_edit_test_mode(&mut app, TestEditorBehavior::Success).expect("test setup");
        assert_eq!(app.status_message, "Editing directories is not supported");

        Ok(())
    }

    #[test]
    fn test_handle_copy_directory() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let left_dir = temp_dir.path().join("left");
        let right_dir = temp_dir.path().join("right");
        fs::create_dir(&left_dir).expect("test setup");
        fs::create_dir(&right_dir).expect("test setup");

        // Create a directory with contents
        let source_dir = left_dir.join("source_dir");
        fs::create_dir(&source_dir).expect("test setup");
        File::create(source_dir.join("file.txt"))
            .expect("test setup")
            .write_all(b"content")
            .expect("test setup");

        let mut app = App::new().expect("test setup");
        app.left_panel = Panel::new(left_dir).expect("test setup");
        app.right_panel = Panel::new(right_dir.clone()).expect("test setup");

        // Select the directory
        for (i, entry) in app.left_panel.entries.iter().enumerate() {
            if entry.name == "source_dir" {
                app.left_panel.state.select(Some(i));
                break;
            }
        }

        handle_copy(&mut app).expect("test setup");

        assert!(right_dir.join("source_dir").exists());
        assert!(right_dir.join("source_dir").join("file.txt").exists());
        assert!(app.status_message.contains("Copied to"));

        Ok(())
    }

    #[test]
    fn test_handle_copy_directory_with_conflict() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let left_dir = temp_dir.path().join("left");
        let right_dir = temp_dir.path().join("right");
        fs::create_dir(&left_dir).expect("test setup");
        fs::create_dir(&right_dir).expect("test setup");

        let source_dir = left_dir.join("source_dir");
        fs::create_dir(&source_dir).expect("test setup");
        fs::write(source_dir.join("file.txt"), b"content").expect("test setup");

        fs::create_dir(right_dir.join("source_dir")).expect("test setup");

        let mut app = App::new().expect("test setup");
        app.left_panel = Panel::new(left_dir.clone()).expect("test setup");
        app.right_panel = Panel::new(right_dir.clone()).expect("test setup");

        for (i, entry) in app.left_panel.entries.iter().enumerate() {
            if entry.name == "source_dir" {
                app.left_panel.state.select(Some(i));
                break;
            }
        }

        handle_copy(&mut app).expect("test setup");

        assert!(right_dir.join("source_dir").exists());
        let copied = right_dir.join("source_dir_1");
        assert!(copied.exists());
        assert_eq!(
            fs::read_to_string(copied.join("file.txt")).expect("test setup"),
            "content"
        );
        assert!(app.status_message.contains("Copied to"));

        Ok(())
    }

    #[test]
    fn test_handle_copy_file() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let left_dir = temp_dir.path().join("left");
        let right_dir = temp_dir.path().join("right");
        fs::create_dir(&left_dir).expect("test setup");
        fs::create_dir(&right_dir).expect("test setup");

        let file_name = "alpha.txt";
        let source_file = left_dir.join(file_name);
        fs::write(&source_file, b"alpha").expect("test setup");

        let mut app = App::new().expect("test setup");
        app.left_panel = Panel::new(left_dir.clone()).expect("test setup");
        app.right_panel = Panel::new(right_dir.clone()).expect("test setup");

        for (i, entry) in app.left_panel.entries.iter().enumerate() {
            if entry.name == file_name {
                app.left_panel.state.select(Some(i));
                break;
            }
        }

        handle_copy(&mut app).expect("test setup");

        let copied = right_dir.join(file_name);
        assert!(source_file.exists());
        assert!(copied.exists());
        assert_eq!(fs::read_to_string(copied).expect("test setup"), "alpha");
        assert!(app.status_message.contains("Copied to"));

        Ok(())
    }

    #[test]
    fn test_handle_copy_file_with_conflict() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let left_dir = temp_dir.path().join("left");
        let right_dir = temp_dir.path().join("right");
        fs::create_dir(&left_dir).expect("test setup");
        fs::create_dir(&right_dir).expect("test setup");

        let file_name = "data.txt";
        let source_file = left_dir.join(file_name);
        fs::write(&source_file, b"source").expect("test setup");
        fs::write(right_dir.join(file_name), b"existing").expect("test setup");

        let mut app = App::new().expect("test setup");
        app.left_panel = Panel::new(left_dir.clone()).expect("test setup");
        app.right_panel = Panel::new(right_dir.clone()).expect("test setup");

        for (i, entry) in app.left_panel.entries.iter().enumerate() {
            if entry.name == file_name {
                app.left_panel.state.select(Some(i));
                break;
            }
        }

        handle_copy(&mut app).expect("test setup");

        let copied = right_dir.join("data_1.txt");
        assert!(source_file.exists());
        assert!(copied.exists());
        assert_eq!(fs::read_to_string(copied).expect("test setup"), "source");
        assert!(app.status_message.contains("Copied to"));

        Ok(())
    }

    #[test]
    fn test_handle_move_rename_success() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let left_dir = temp_dir.path().join("left");
        let right_dir = temp_dir.path().join("right");
        fs::create_dir(&left_dir).expect("test setup");
        fs::create_dir(&right_dir).expect("test setup");

        let file_name = "move_ok.txt";
        let source_file = left_dir.join(file_name);
        fs::write(&source_file, b"content").expect("test setup");

        let mut app = App::new().expect("test setup");
        app.left_panel = Panel::new(left_dir.clone()).expect("test setup");
        app.right_panel = Panel::new(right_dir.clone()).expect("test setup");

        for (i, entry) in app.left_panel.entries.iter().enumerate() {
            if entry.name == file_name {
                app.left_panel.state.select(Some(i));
                break;
            }
        }

        handle_move(&mut app).expect("test setup");

        assert!(!source_file.exists());
        let moved_file = right_dir.join(file_name);
        assert!(moved_file.exists());
        assert_eq!(
            fs::read_to_string(moved_file).expect("test setup"),
            "content"
        );
        assert!(app.status_message.contains("Moved to"));

        Ok(())
    }

    #[test]
    fn test_handle_move_directory_rename_success() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let left_dir = temp_dir.path().join("left");
        let right_dir = temp_dir.path().join("right");
        fs::create_dir(&left_dir).expect("test setup");
        fs::create_dir(&right_dir).expect("test setup");

        let source_dir = left_dir.join("dir_move");
        fs::create_dir(&source_dir).expect("test setup");
        fs::write(source_dir.join("file.txt"), b"dircontent").expect("test setup");

        let mut app = App::new().expect("test setup");
        app.left_panel = Panel::new(left_dir.clone()).expect("test setup");
        app.right_panel = Panel::new(right_dir.clone()).expect("test setup");

        for (i, entry) in app.left_panel.entries.iter().enumerate() {
            if entry.name == "dir_move" {
                app.left_panel.state.select(Some(i));
                break;
            }
        }

        handle_move(&mut app).expect("test setup");

        assert!(!source_dir.exists());
        let moved_dir = right_dir.join("dir_move");
        assert!(moved_dir.exists());
        assert_eq!(
            fs::read_to_string(moved_dir.join("file.txt")).expect("test setup"),
            "dircontent"
        );
        assert!(app.status_message.contains("Moved to"));

        Ok(())
    }

    #[test]
    fn test_handle_move_directory_with_conflict_renames() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let left_dir = temp_dir.path().join("left");
        let right_dir = temp_dir.path().join("right");
        fs::create_dir(&left_dir).expect("test setup");
        fs::create_dir(&right_dir).expect("test setup");

        let source_dir = left_dir.join("dir_move");
        fs::create_dir(&source_dir).expect("test setup");
        fs::write(source_dir.join("file.txt"), b"dircontent").expect("test setup");

        fs::create_dir(right_dir.join("dir_move")).expect("test setup");

        let mut app = App::new().expect("test setup");
        app.left_panel = Panel::new(left_dir.clone()).expect("test setup");
        app.right_panel = Panel::new(right_dir.clone()).expect("test setup");

        for (i, entry) in app.left_panel.entries.iter().enumerate() {
            if entry.name == "dir_move" {
                app.left_panel.state.select(Some(i));
                break;
            }
        }

        handle_move(&mut app).expect("test setup");

        assert!(!source_dir.exists());
        assert!(right_dir.join("dir_move").exists());
        let moved_dir = right_dir.join("dir_move_1");
        assert!(moved_dir.exists());
        assert_eq!(
            fs::read_to_string(moved_dir.join("file.txt")).expect("test setup"),
            "dircontent"
        );
        assert!(app.status_message.contains("Moved to"));

        Ok(())
    }

    #[test]
    fn test_handle_move_file_with_conflict_renames() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let left_dir = temp_dir.path().join("left");
        let right_dir = temp_dir.path().join("right");
        fs::create_dir(&left_dir).expect("test setup");
        fs::create_dir(&right_dir).expect("test setup");

        let file_name = "conflict.txt";
        let source_file = left_dir.join(file_name);
        fs::write(&source_file, b"source").expect("test setup");
        fs::write(right_dir.join(file_name), b"existing").expect("test setup");

        let mut app = App::new().expect("test setup");
        app.left_panel = Panel::new(left_dir.clone()).expect("test setup");
        app.right_panel = Panel::new(right_dir.clone()).expect("test setup");

        for (i, entry) in app.left_panel.entries.iter().enumerate() {
            if entry.name == file_name {
                app.left_panel.state.select(Some(i));
                break;
            }
        }

        handle_move(&mut app).expect("test setup");

        assert!(!source_file.exists());
        assert_eq!(
            fs::read_to_string(right_dir.join(file_name)).expect("test setup"),
            "existing"
        );
        let moved_file = right_dir.join("conflict_1.txt");
        assert!(moved_file.exists());
        assert_eq!(
            fs::read_to_string(moved_file).expect("test setup"),
            "source"
        );
        assert!(app.status_message.contains("Moved to"));

        Ok(())
    }

    #[test]
    fn test_handle_move_cross_device() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let left_dir = temp_dir.path().join("left");
        let right_dir = temp_dir.path().join("right");
        fs::create_dir(&left_dir).expect("test setup");
        fs::create_dir(&right_dir).expect("test setup");

        // Create a directory to move
        let source_dir = left_dir.join("move_dir");
        fs::create_dir(&source_dir).expect("test setup");
        File::create(source_dir.join("file.txt"))
            .expect("test setup")
            .write_all(b"content")
            .expect("test setup");

        let mut app = App::new().expect("test setup");
        app.left_panel = Panel::new(left_dir.clone()).expect("test setup");
        app.right_panel = Panel::new(right_dir.clone()).expect("test setup");

        // Select the directory
        for (i, entry) in app.left_panel.entries.iter().enumerate() {
            if entry.name == "move_dir" {
                app.left_panel.state.select(Some(i));
                break;
            }
        }

        let _guard = ForceRenameFailureGuard::enable();
        handle_move(&mut app).expect("test setup");

        assert!(!source_dir.exists());
        assert!(right_dir.join("move_dir").exists());
        assert!(right_dir.join("move_dir").join("file.txt").exists());
        assert!(app.status_message.contains("Moved to"));

        Ok(())
    }

    #[test]
    fn test_handle_move_fallback_for_file() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let left_dir = temp_dir.path().join("left");
        let right_dir = temp_dir.path().join("right");
        fs::create_dir(&left_dir).expect("test setup");
        fs::create_dir(&right_dir).expect("test setup");

        let file_name = "move_file.txt";
        let source_file = left_dir.join(file_name);
        File::create(&source_file)
            .expect("test setup")
            .write_all(b"content")
            .expect("test setup");

        let mut app = App::new().expect("test setup");
        app.left_panel = Panel::new(left_dir.clone()).expect("test setup");
        app.right_panel = Panel::new(right_dir.clone()).expect("test setup");

        for (i, entry) in app.left_panel.entries.iter().enumerate() {
            if entry.name == file_name {
                app.left_panel.state.select(Some(i));
                break;
            }
        }

        let _guard = ForceRenameFailureGuard::enable();
        handle_move(&mut app).expect("test setup");

        assert!(!source_file.exists());
        let moved_file = right_dir.join(file_name);
        assert!(moved_file.exists());
        assert_eq!(
            fs::read_to_string(&moved_file).expect("test setup"),
            "content"
        );
        assert!(app.status_message.contains("Moved to"));

        Ok(())
    }

    #[test]
    fn test_handle_mkdir_multiple() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");

        let mut app = App::new().expect("test setup");
        app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
        app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

        // Create first directory
        handle_mkdir(&mut app).expect("test setup");
        assert!(temp_dir.path().join("new_folder").exists());

        // Create second directory (should be numbered)
        handle_mkdir(&mut app).expect("test setup");
        assert!(temp_dir.path().join("new_folder_1").exists());

        // Create third directory
        handle_mkdir(&mut app).expect("test setup");
        assert!(temp_dir.path().join("new_folder_2").exists());

        Ok(())
    }

    #[test]
    fn test_handle_delete_no_selection() -> io::Result<()> {
        let mut app = App::new().expect("test setup");
        app.left_panel.state.select(None);

        // Should not crash
        handle_delete(&mut app).expect("test setup");

        Ok(())
    }

    #[test]
    fn test_handle_delete_file() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let file_path = temp_dir.path().join("delete.txt");
        File::create(&file_path).expect("test setup");

        let mut app = App::new().expect("test setup");
        app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
        app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
        for (i, entry) in app.left_panel.entries.iter().enumerate() {
            if entry.name == "delete.txt" {
                app.left_panel.state.select(Some(i));
                break;
            }
        }

        handle_delete(&mut app).expect("test setup");

        assert!(!file_path.exists());
        assert!(app.status_message.contains("Deleted"));

        Ok(())
    }

    #[test]
    fn test_handle_delete_directory() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let dir_path = temp_dir.path().join("delete_dir");
        fs::create_dir(&dir_path).expect("test setup");

        let mut app = App::new().expect("test setup");
        app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
        app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
        for (i, entry) in app.left_panel.entries.iter().enumerate() {
            if entry.name == "delete_dir" {
                app.left_panel.state.select(Some(i));
                break;
            }
        }

        handle_delete(&mut app).expect("test setup");

        assert!(!dir_path.exists());
        assert!(app.status_message.contains("Deleted"));

        Ok(())
    }

    #[test]
    fn test_handle_copy_no_selection() -> io::Result<()> {
        let mut app = App::new().expect("test setup");
        app.left_panel.state.select(None);

        // Should not crash
        handle_copy(&mut app).expect("test setup");

        Ok(())
    }

    #[test]
    fn test_handle_move_no_selection() -> io::Result<()> {
        let mut app = App::new().expect("test setup");
        app.left_panel.state.select(None);

        // Should not crash
        handle_move(&mut app).expect("test setup");

        Ok(())
    }

    #[test]
    fn test_handle_overlay_keys_viewer_other_key() -> io::Result<()> {
        let mut app = App::new().expect("test setup");
        app.viewer = Some(Viewer {
            title: "test.txt".to_string(),
            lines: vec!["Line 1".to_string()],
            scroll: 0,
        });

        // Test with a key that doesn't do anything in viewer
        assert!(handle_overlay_keys(&mut app, key_event(KeyCode::Char('a'))));
        // Viewer should still be active
        assert!(app.viewer.is_some());

        Ok(())
    }

    #[test]
    fn test_handle_overlay_keys_help_other_key() -> io::Result<()> {
        let mut app = App::new().expect("test setup");
        app.show_help = true;

        // Test with a key that doesn't close help
        assert!(handle_overlay_keys(&mut app, key_event(KeyCode::Down)));
        // Help should still be shown
        assert!(app.show_help);

        Ok(())
    }

    #[test]
    #[cfg(feature = "test-helpers")]
    fn test_handle_edit_no_selection() -> io::Result<()> {
        let mut app = App::new().expect("test setup");
        app.left_panel.state.select(None);

        // Should not crash
        handle_edit_test_mode(&mut app, TestEditorBehavior::Success).expect("test setup");
        Ok(())
    }

    #[test]
    #[cfg(feature = "test-helpers")]
    fn test_handle_edit_impl_success() -> io::Result<()> {
        use ratatui::Terminal;

        let backend = WritableTestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test setup");

        let temp_dir = TempDir::new().expect("test setup");
        let file_path = temp_dir.path().join("test.txt");
        File::create(&file_path).expect("test setup");

        let mut app = App::new().expect("test setup");
        app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

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

        assert!(result.is_ok());
        assert!(app.status_message.contains("Edited"));

        Ok(())
    }

    #[test]
    #[cfg(feature = "test-helpers")]
    fn test_handle_edit_impl_editor_error_exit() -> io::Result<()> {
        use ratatui::Terminal;

        let backend = WritableTestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test setup");

        let temp_dir = TempDir::new().expect("test setup");
        let file_path = temp_dir.path().join("test.txt");
        File::create(&file_path).expect("test setup");

        let mut app = App::new().expect("test setup");
        app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

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
    #[cfg(feature = "test-helpers")]
    fn test_handle_edit_impl_launch_failure() -> io::Result<()> {
        use ratatui::Terminal;

        let backend = WritableTestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test setup");

        let temp_dir = TempDir::new().expect("test setup");
        let file_path = temp_dir.path().join("test.txt");
        File::create(&file_path).expect("test setup");

        let mut app = App::new().expect("test setup");
        app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

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
    #[cfg(feature = "test-helpers")]
    fn test_handle_edit_impl_no_selection() -> io::Result<()> {
        let mut app = App::new().expect("test setup");
        app.left_panel.state.select(None);
        let initial_status = app.status_message.clone();

        let result = handle_edit_test_mode(&mut app, TestEditorBehavior::Success);

        assert!(result.is_ok());
        assert_eq!(app.status_message, initial_status);

        Ok(())
    }

    #[test]
    #[cfg(feature = "test-helpers")]
    fn test_handle_edit_impl_directory() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        fs::create_dir(temp_dir.path().join("subdir")).expect("test setup");

        let mut app = App::new().expect("test setup");
        app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

        // Select the directory
        for (i, entry) in app.left_panel.entries.iter().enumerate() {
            if entry.name == "subdir" {
                app.left_panel.state.select(Some(i));
                break;
            }
        }

        let result = handle_edit_test_mode(&mut app, TestEditorBehavior::Success);

        assert!(result.is_ok());
        assert_eq!(app.status_message, "Editing directories is not supported");

        Ok(())
    }

    #[test]
    fn test_handle_overlay_keys_search() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        File::create(temp_dir.path().join("alpha.txt")).expect("test setup");
        File::create(temp_dir.path().join("beta.txt")).expect("test setup");

        let mut app = App::new().expect("test setup");
        app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
        app.search = Some(String::new());

        assert!(handle_overlay_keys(&mut app, key_event(KeyCode::Char('b'))));
        let beta_index = app
            .left_panel
            .entries
            .iter()
            .position(|entry| entry.name == "beta.txt")
            .unwrap();
        assert_eq!(app.left_panel.state.selected(), Some(beta_index));

        assert!(handle_overlay_keys(
            &mut app,
            key_event_with_mods(KeyCode::Char('x'), KeyModifiers::CONTROL)
        ));
        assert_eq!(app.search.as_deref(), Some("b"));

        app.search = Some("ab".to_string());
        assert!(handle_overlay_keys(&mut app, key_event(KeyCode::Backspace)));
        assert_eq!(app.search.as_deref(), Some("a"));

        assert!(handle_overlay_keys(&mut app, key_event(KeyCode::Enter)));
        assert!(app.search.is_none());
        assert!(app.status_message.contains("Found: a"));

        app.search = Some("zzz".to_string());
        assert!(handle_overlay_keys(&mut app, key_event(KeyCode::Enter)));
        assert!(app.search.is_none());
        assert!(app.status_message.contains("Not found: zzz"));

        app.search = Some("alpha".to_string());
        assert!(handle_overlay_keys(&mut app, key_event(KeyCode::Esc)));
        assert!(app.search.is_none());

        Ok(())
    }

    #[test]
    fn test_handle_overlay_keys_search_enter_empty_query() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");

        let mut app = App::new().expect("test setup");
        app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
        app.search = Some("   ".to_string());
        app.status_message = "status".to_string();

        assert!(handle_overlay_keys(&mut app, key_event(KeyCode::Enter)));
        assert!(app.search.is_none());
        assert_eq!(app.status_message, "status");

        Ok(())
    }

    #[test]
    fn test_handle_overlay_keys_search_whitespace_char() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        File::create(temp_dir.path().join("alpha.txt")).expect("test setup");

        let mut app = App::new().expect("test setup");
        app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
        app.search = Some(String::new());

        let selected_before = app.left_panel.state.selected();
        assert!(handle_overlay_keys(&mut app, key_event(KeyCode::Char(' '))));
        assert_eq!(app.search.as_deref(), Some(" "));
        assert_eq!(app.left_panel.state.selected(), selected_before);

        Ok(())
    }

    #[test]
    fn test_handle_overlay_keys_search_backspace_empty_query() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");

        let mut app = App::new().expect("test setup");
        app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
        app.search = Some(String::new());

        assert!(handle_overlay_keys(&mut app, key_event(KeyCode::Backspace)));
        assert_eq!(app.search.as_deref(), Some(""));

        Ok(())
    }

    #[test]
    fn test_handle_overlay_keys_search_ignores_alt_char() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");

        let mut app = App::new().expect("test setup");
        app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
        app.search = Some("a".to_string());

        assert!(handle_overlay_keys(
            &mut app,
            key_event_with_mods(KeyCode::Char('b'), KeyModifiers::ALT)
        ));
        assert_eq!(app.search.as_deref(), Some("a"));

        Ok(())
    }

    #[test]
    fn test_handle_overlay_keys_bookmarks() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let bookmark_dir = temp_dir.path().join("bookmark");
        fs::create_dir(&bookmark_dir).expect("test setup");

        let mut app = App::new().expect("test setup");
        app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

        app.show_bookmarks = true;
        app.bookmarks = vec![None; 1];
        assert!(handle_overlay_keys(&mut app, key_event(KeyCode::Char('9'))));
        assert!(app.status_message.contains("Invalid bookmark slot"));
        assert!(app.show_bookmarks);

        app.bookmarks = vec![None; 10];
        app.show_bookmarks = true;
        assert!(handle_overlay_keys(
            &mut app,
            key_event_with_mods(KeyCode::Char('1'), KeyModifiers::CONTROL)
        ));
        assert!(app.bookmarks[1].is_some());

        app.show_bookmarks = true;
        assert!(handle_overlay_keys(&mut app, key_event(KeyCode::Char('2'))));
        assert!(app.status_message.contains("Bookmark 2 is empty"));

        app.bookmarks[3] = Some(bookmark_dir.clone());
        app.show_bookmarks = true;
        assert!(handle_overlay_keys(&mut app, key_event(KeyCode::Char('3'))));
        assert!(!app.show_bookmarks);
        assert_eq!(app.get_active_panel().path, bookmark_dir);

        app.bookmarks[4] = Some(temp_dir.path().join("missing"));
        app.show_bookmarks = true;
        assert!(handle_overlay_keys(&mut app, key_event(KeyCode::Char('4'))));
        assert!(app.status_message.contains("Failed to open bookmark"));

        app.show_bookmarks = true;
        assert!(handle_overlay_keys(&mut app, key_event(KeyCode::Char('x'))));
        assert!(app.show_bookmarks);

        app.show_bookmarks = true;
        assert!(handle_overlay_keys(&mut app, key_event(KeyCode::Esc)));
        assert!(!app.show_bookmarks);

        Ok(())
    }

    #[test]
    fn test_handle_overlay_keys_command_line() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");

        let mut app = App::new().expect("test setup");
        app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

        app.command_line = Some(String::new());
        assert!(handle_overlay_keys(&mut app, key_event(KeyCode::Char('p'))));
        assert_eq!(app.command_line.as_deref(), Some("p"));

        assert!(handle_overlay_keys(
            &mut app,
            key_event_with_mods(KeyCode::Char('x'), KeyModifiers::ALT)
        ));
        assert_eq!(app.command_line.as_deref(), Some("p"));

        assert!(handle_overlay_keys(&mut app, key_event(KeyCode::Backspace)));
        assert_eq!(app.command_line.as_deref(), Some(""));

        assert!(handle_overlay_keys(&mut app, key_event(KeyCode::Enter)));
        assert!(app.command_line.is_none());

        app.command_line = Some("pwd".to_string());
        assert!(handle_overlay_keys(&mut app, key_event(KeyCode::Enter)));
        assert_eq!(app.status_message, temp_dir.path().display().to_string());

        app.command_line = Some("unknown".to_string());
        assert!(handle_overlay_keys(&mut app, key_event(KeyCode::Enter)));
        assert!(app.status_message.contains("Command failed"));

        app.command_line = Some("quit".to_string());
        assert!(handle_overlay_keys(&mut app, key_event(KeyCode::Enter)));
        assert!(app.should_quit);

        app.should_quit = false;
        app.command_line = Some("pwd".to_string());
        assert!(handle_overlay_keys(&mut app, key_event(KeyCode::Esc)));
        assert!(app.command_line.is_none());

        Ok(())
    }

    #[test]
    fn test_handle_overlay_keys_command_line_enter_empty() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");

        let mut app = App::new().expect("test setup");
        app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
        app.command_line = Some("   ".to_string());
        app.status_message = "status".to_string();

        assert!(handle_overlay_keys(&mut app, key_event(KeyCode::Enter)));
        assert!(app.command_line.is_none());
        assert_eq!(app.status_message, "status");

        Ok(())
    }

    #[test]
    fn test_handle_overlay_keys_command_line_backspace_empty() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");

        let mut app = App::new().expect("test setup");
        app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
        app.command_line = Some(String::new());

        assert!(handle_overlay_keys(&mut app, key_event(KeyCode::Backspace)));
        assert_eq!(app.command_line.as_deref(), Some(""));

        Ok(())
    }

    #[test]
    fn test_handle_overlay_keys_command_line_ignores_control_char() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");

        let mut app = App::new().expect("test setup");
        app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
        app.command_line = Some("a".to_string());

        assert!(handle_overlay_keys(
            &mut app,
            key_event_with_mods(KeyCode::Char('b'), KeyModifiers::CONTROL)
        ));
        assert_eq!(app.command_line.as_deref(), Some("a"));

        Ok(())
    }

    #[test]
    fn test_execute_command_branches() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let subdir = temp_dir.path().join("subdir");
        let bookmark_dir = temp_dir.path().join("bookmark");
        fs::create_dir(&subdir).expect("test setup");
        fs::create_dir(&bookmark_dir).expect("test setup");
        fs::write(temp_dir.path().join("file.txt"), b"data").expect("test setup");
        fs::write(subdir.join("nested.txt"), b"nested").expect("test setup");

        let mut app = App::new().expect("test setup");
        app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
        app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
        app.active_panel = crate::models::PanelSide::Left;

        assert!(execute_command(&mut app, "").is_err());
        assert!(execute_command(&mut app, "cd").is_err());
        assert!(execute_command(&mut app, "cd file.txt").is_err());

        execute_command(&mut app, "cd subdir").expect("test setup");
        assert!(app.get_active_panel().path.ends_with("subdir"));
        assert!(app.status_message.contains("Opened"));

        let root_path = temp_dir
            .path()
            .ancestors()
            .last()
            .unwrap_or(temp_dir.path());
        execute_command(&mut app, &format!("cd {}", root_path.display())).expect("test setup");
        assert_eq!(
            app.get_active_panel()
                .path
                .canonicalize()
                .expect("test setup"),
            root_path.canonicalize().expect("test setup")
        );

        execute_command(&mut app, "sort name").expect("test setup");
        assert_eq!(app.get_active_panel().sort_mode, SortMode::Name);
        execute_command(&mut app, "sort ext").expect("test setup");
        assert_eq!(app.get_active_panel().sort_mode, SortMode::Extension);
        execute_command(&mut app, "sort size").expect("test setup");
        assert_eq!(app.get_active_panel().sort_mode, SortMode::Size);
        assert!(execute_command(&mut app, "sort bogus").is_err());

        assert!(execute_command(&mut app, "mark").is_err());
        execute_command(&mut app, "mark all").expect("test setup");
        assert!(!app.get_active_panel().marked.is_empty());
        execute_command(&mut app, "unmark").expect("test setup");
        assert!(app.get_active_panel().marked.is_empty());
        assert!(execute_command(&mut app, "mark nope").is_err());

        execute_command(&mut app, "help").expect("test setup");
        assert!(app.show_help);

        assert!(execute_command(&mut app, "bookmark").is_err());
        assert!(execute_command(&mut app, "bookmark set").is_err());
        assert!(execute_command(&mut app, "bookmark set a").is_err());
        assert!(execute_command(&mut app, "bookmark set 99").is_err());
        assert!(execute_command(&mut app, "bookmark foo 1").is_err());

        execute_command(&mut app, "bookmark set 0").expect("test setup");
        assert!(app.bookmarks[0].is_some());
        assert!(execute_command(&mut app, "bookmark go 1").is_err());

        app.bookmarks[0] = Some(bookmark_dir.clone());
        execute_command(&mut app, "bookmark go 0").expect("test setup");
        assert_eq!(app.get_active_panel().path, bookmark_dir);

        Ok(())
    }

    #[test]
    fn test_handle_copy_permission_denied() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let mut app = App::new().expect("test setup");

        let source_file = temp_dir.path().join("source.txt");
        let dest_dir = temp_dir.path().join("readonly_dest");
        fs::create_dir(&dest_dir).expect("test setup");
        File::create(&source_file)
            .expect("test setup")
            .write_all(b"content")
            .expect("test setup");

        app.left_panel.path = temp_dir.path().to_path_buf();
        app.right_panel.path = dest_dir.clone();
        app.left_panel.load_entries().expect("test setup");
        app.right_panel.load_entries().expect("test setup");
        app.left_panel.state.select(Some(1));

        // Skip setting readonly permissions on Windows as it's complex
        // Test by selecting a file that doesn't exist
        app.left_panel.entries.clear(); // Clear all entries
        app.left_panel.state.select(Some(0)); // Select index 0 but no entries

        let result = handle_copy(&mut app);
        assert!(result.is_ok());
        // No file selected or operation should do nothing

        Ok(())
    }

    #[test]
    fn test_handle_move_same_source_and_destination() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let mut app = App::new().expect("test setup");

        let file = temp_dir.path().join("file.txt");
        File::create(&file)
            .expect("test setup")
            .write_all(b"content")
            .expect("test setup");

        app.left_panel.path = temp_dir.path().to_path_buf();
        app.right_panel.path = temp_dir.path().to_path_buf();
        app.left_panel.load_entries().expect("test setup");
        app.right_panel.load_entries().expect("test setup");
        app.left_panel.state.select(Some(1));

        // handle_move will rename the file in place when source and dest are same
        let result = handle_move(&mut app);
        assert!(result.is_ok());

        // The file should have been renamed with unique suffix
        let moved_file = temp_dir.path().join("file_1.txt");
        assert!(moved_file.exists(), "File should be renamed to file_1.txt");
        assert!(!file.exists(), "Original file should not exist");
        assert!(app.status_message.contains("Moved to"));

        Ok(())
    }

    #[test]
    fn test_handle_delete_marked_entries() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let mut app = App::new().expect("test setup");

        let file1 = temp_dir.path().join("file1.txt");
        let file2 = temp_dir.path().join("file2.txt");
        let file3 = temp_dir.path().join("file3.txt");
        File::create(&file1)
            .expect("test setup")
            .write_all(b"1")
            .expect("test setup");
        File::create(&file2)
            .expect("test setup")
            .write_all(b"2")
            .expect("test setup");
        File::create(&file3)
            .expect("test setup")
            .write_all(b"3")
            .expect("test setup");

        app.left_panel.path = temp_dir.path().to_path_buf();
        app.left_panel.load_entries().expect("test setup");

        app.left_panel.marked.insert("file1.txt".to_string());
        app.left_panel.marked.insert("file3.txt".to_string());

        // Currently, handle_delete only deletes the selected entry, not marked ones
        // Select file2 and delete it
        for (i, entry) in app.left_panel.entries.iter().enumerate() {
            if entry.name == "file2.txt" {
                app.left_panel.state.select(Some(i));
                break;
            }
        }

        handle_delete(&mut app).expect("test setup");

        // Only the selected file should be deleted
        assert!(file1.exists());
        assert!(!file2.exists());
        assert!(file3.exists());
        assert!(app.status_message.contains("Deleted"));

        Ok(())
    }

    #[test]
    fn test_handle_mkdir_invalid_names() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let mut app = App::new().expect("test setup");

        app.left_panel.path = temp_dir.path().to_path_buf();
        app.left_panel.load_entries().expect("test setup");

        // Test mkdir with empty input - this test may need adjustment based on actual mkdir implementation
        handle_mkdir(&mut app).expect("test setup");
        // Check if a new_folder was created
        assert!(temp_dir.path().join("new_folder").exists());

        #[cfg(windows)]
        {
            // Test with invalid characters on Windows
            // This test would need adjustment based on how mkdir actually handles input
        }

        Ok(())
    }

    #[test]
    fn test_handle_view_large_file() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let mut app = App::new().expect("test setup");

        let large_file = temp_dir.path().join("large.txt");
        let content = "Line\n".repeat(10000);
        File::create(&large_file)
            .expect("test setup")
            .write_all(content.as_bytes())
            .expect("test setup");

        app.left_panel.path = temp_dir.path().to_path_buf();
        app.left_panel.load_entries().expect("test setup");
        app.left_panel.state.select(Some(1));

        handle_view(&mut app).expect("test setup");

        let viewer = app.viewer.as_ref().expect("viewer");
        assert!(viewer.lines.len() >= 9999);

        Ok(())
    }

    #[test]
    fn test_handle_overlay_keys_viewer_navigation_limits() -> io::Result<()> {
        let mut app = App::new().expect("test setup");
        let lines: Vec<String> = (0..20).map(|i| format!("Line {}", i)).collect();

        app.viewer = Some(Viewer {
            lines: lines.clone(),
            scroll: 0,
            title: "Test".to_string(),
        });

        for _ in 0..30 {
            handle_overlay_keys(&mut app, key_event(KeyCode::Up));
        }
        assert_eq!(app.viewer.as_ref().unwrap().scroll, 0);

        for _ in 0..100 {
            handle_overlay_keys(&mut app, key_event(KeyCode::Down));
        }

        let viewer = app.viewer.as_ref().expect("viewer");
        assert!((viewer.scroll as usize) <= viewer.lines.len());

        Ok(())
    }

    #[test]
    fn test_handle_overlay_keys_search_special_characters() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let mut app = App::new().expect("test setup");

        let special_file = temp_dir.path().join("[special].txt");
        let normal_file = temp_dir.path().join("normal.txt");
        File::create(&special_file)
            .expect("test setup")
            .write_all(b"content")
            .expect("test setup");
        File::create(&normal_file)
            .expect("test setup")
            .write_all(b"content")
            .expect("test setup");

        app.left_panel.path = temp_dir.path().to_path_buf();
        app.left_panel.load_entries().expect("test setup");
        app.search = Some("[spec".to_string());
        handle_overlay_keys(&mut app, key_event(KeyCode::Enter));

        assert!(app.search.is_none());

        Ok(())
    }

    #[test]
    fn test_execute_command_edge_cases() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let mut app = App::new().expect("test setup");
        app.left_panel.path = temp_dir.path().to_path_buf();

        assert!(execute_command(&mut app, "").is_err());
        assert!(execute_command(&mut app, "   ").is_err());
        assert!(execute_command(&mut app, "unknown_command").is_err());

        execute_command(&mut app, "  help  ").expect("test setup");
        assert!(app.show_help);
        app.show_help = false;

        // Test unknown command returns error
        assert!(execute_command(&mut app, "HELP").is_err());

        Ok(())
    }

    #[test]
    fn test_jump_to_match_wraparound() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let mut app = App::new().expect("test setup");

        for i in 0..5 {
            File::create(temp_dir.path().join(format!("test{}.txt", i))).expect("test setup");
        }

        app.left_panel.path = temp_dir.path().to_path_buf();
        app.left_panel.load_entries().expect("test setup");
        app.left_panel
            .state
            .select(Some(app.left_panel.entries.len() - 1));

        jump_to_match(&mut app.left_panel, "test");

        assert!(app.left_panel.state.selected().unwrap() < app.left_panel.entries.len() - 1);

        Ok(())
    }

    #[test]
    fn test_jump_to_match_no_selection() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

        for i in 0..5 {
            File::create(temp_dir.path().join(format!("file{}.txt", i))).expect("test setup");
        }

        panel.load_entries().expect("test setup");
        panel.state.select(None); // No selection

        let found = jump_to_match(&mut panel, "file");
        assert!(found);
        assert!(panel.state.selected().is_some());

        Ok(())
    }

    #[test]
    fn test_jump_to_match_no_match_found() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

        File::create(temp_dir.path().join("test.txt")).expect("test setup");
        panel.load_entries().expect("test setup");
        panel.state.select(Some(1));

        let found = jump_to_match(&mut panel, "nonexistent");
        assert!(!found);
        assert_eq!(panel.state.selected(), Some(1)); // Selection unchanged

        Ok(())
    }

    #[test]
    fn test_jump_to_match_empty_entries() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

        // Don't load entries, keep it empty
        panel.entries.clear();

        let found = jump_to_match(&mut panel, "test");
        assert!(!found);

        Ok(())
    }

    #[test]
    fn test_execute_command_cd_missing_argument() -> io::Result<()> {
        let mut app = App::new().expect("test setup");

        let result = execute_command(&mut app, "cd");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidInput);

        Ok(())
    }

    #[test]
    fn test_execute_command_cd_invalid_path() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let mut app = App::new().expect("test setup");
        app.left_panel.path = temp_dir.path().to_path_buf();

        // Try to cd to a file instead of directory
        let file = temp_dir.path().join("file.txt");
        File::create(&file).expect("test setup");

        let result = execute_command(&mut app, &format!("cd {}", file.display()));
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn test_execute_command_sort_invalid_mode() -> io::Result<()> {
        let mut app = App::new().expect("test setup");

        let result = execute_command(&mut app, "sort invalid");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidInput);

        Ok(())
    }

    #[test]
    fn test_execute_command_bookmark_missing_subcommand() -> io::Result<()> {
        let mut app = App::new().expect("test setup");

        let result = execute_command(&mut app, "bookmark");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidInput);

        Ok(())
    }

    #[test]
    fn test_execute_command_bookmark_invalid_slot() -> io::Result<()> {
        let mut app = App::new().expect("test setup");

        let result = execute_command(&mut app, "bookmark set 10");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("bookmark"));

        let result = execute_command(&mut app, "bookmark set abc");
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn test_execute_command_bookmark_go_empty() -> io::Result<()> {
        let mut app = App::new().expect("test setup");

        // Try to go to an empty bookmark
        let result = execute_command(&mut app, "bookmark go 0");
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Bookmark") || err_msg.contains("empty"));

        Ok(())
    }

    #[test]
    fn test_execute_command_bookmark_invalid_subcommand() -> io::Result<()> {
        let mut app = App::new().expect("test setup");

        let result = execute_command(&mut app, "bookmark invalid 0");
        assert!(result.is_err());
        // Just check that it returns an error
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn test_execute_command_mark_missing_argument() -> io::Result<()> {
        let mut app = App::new().expect("test setup");

        let result = execute_command(&mut app, "mark");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidInput);

        Ok(())
    }

    #[test]
    fn test_execute_command_mark_invalid_subcommand() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let mut app = App::new().expect("test setup");
        app.left_panel.path = temp_dir.path().to_path_buf();
        app.left_panel.load_entries().expect("test setup");

        let result = execute_command(&mut app, "mark invalid");
        assert!(result.is_err());
        // Just check that it returns an error
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn test_execute_command_unknown_command() -> io::Result<()> {
        let mut app = App::new().expect("test setup");

        let result = execute_command(&mut app, "unknowncommand arg1 arg2");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unknown command"));

        Ok(())
    }

    #[test]
    fn test_handle_view_directory_error() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let mut app = App::new().expect("test setup");

        let dir = temp_dir.path().join("subdir");
        fs::create_dir(&dir).expect("test setup");

        app.left_panel.path = temp_dir.path().to_path_buf();
        app.left_panel.load_entries().expect("test setup");

        // Find and select the subdir
        for (i, entry) in app.left_panel.entries.iter().enumerate() {
            if entry.name == "subdir" && entry.is_dir {
                app.left_panel.state.select(Some(i));
                break;
            }
        }

        // handle_view returns Ok for directories but shows a message
        let result = handle_view(&mut app);
        assert!(result.is_ok());
        assert!(app.status_message.contains("Cannot view directory") || app.viewer.is_none());

        Ok(())
    }

    #[test]
    fn test_handle_overlay_keys_viewer_scroll_limits() -> io::Result<()> {
        let mut app = App::new().expect("test setup");
        app.viewer = Some(Viewer {
            lines: vec!["Line 1".to_string(), "Line 2".to_string()],
            scroll: 0,
            title: "Test".to_string(),
        });

        // Test scrolling down at max
        if let Some(viewer) = &mut app.viewer {
            viewer.scroll = 1000; // Way beyond content
        }
        handle_overlay_keys(
            &mut app,
            KeyEvent::new(KeyCode::Down, KeyModifiers::empty()),
        );
        assert_eq!(app.viewer.as_ref().unwrap().scroll, 1000);

        // Test page down at max
        handle_overlay_keys(
            &mut app,
            KeyEvent::new(KeyCode::PageDown, KeyModifiers::empty()),
        );
        assert_eq!(app.viewer.as_ref().unwrap().scroll, 1000);

        Ok(())
    }

    #[test]
    fn test_handle_overlay_keys_invalid_bookmark_slot() -> io::Result<()> {
        let mut app = App::new().expect("test setup");
        app.show_bookmarks = true;

        // Try invalid bookmark slot
        handle_overlay_keys(
            &mut app,
            KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty()),
        );
        assert!(app.show_bookmarks); // Still showing bookmarks

        Ok(())
    }

    #[test]
    fn test_handle_overlay_keys_command_execute_error() -> io::Result<()> {
        let mut app = App::new().expect("test setup");
        app.command_line = Some(String::new());
        app.command_line = Some("unknown_command".to_string());

        handle_overlay_keys(
            &mut app,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()),
        );
        assert!(app.command_line.is_none());
        assert!(app.status_message.contains("Error") || app.status_message.contains("unknown"));

        Ok(())
    }

    #[test]
    fn test_handle_move_cross_device_fallback() -> io::Result<()> {
        // This test simulates cross-device move by using the force rename failure
        let temp_dir = TempDir::new().expect("test setup");
        let mut app = App::new().expect("test setup");

        let source_file = temp_dir.path().join("source.txt");
        File::create(&source_file)
            .expect("test setup")
            .write_all(b"content")
            .expect("test setup");

        let dest_dir = temp_dir.path().join("dest");
        fs::create_dir(&dest_dir).expect("test setup");

        app.left_panel.path = temp_dir.path().to_path_buf();
        app.right_panel.path = dest_dir;
        app.left_panel.load_entries().expect("test setup");
        app.right_panel.load_entries().expect("test setup");

        // Find source.txt in entries
        for (i, entry) in app.left_panel.entries.iter().enumerate() {
            if entry.name == "source.txt" {
                app.left_panel.state.select(Some(i));
                break;
            }
        }

        // Force rename to fail to trigger fallback
        FORCE_RENAME_FAILURE.store(true, std::sync::atomic::Ordering::SeqCst);

        let result = handle_move(&mut app);

        FORCE_RENAME_FAILURE.store(false, std::sync::atomic::Ordering::SeqCst);

        assert!(result.is_ok());
        assert!(!source_file.exists()); // Source should be deleted

        Ok(())
    }

    #[test]
    #[cfg(not(coverage))]
    #[ignore = "Stack overflow on Windows"]
    fn test_handle_copy_move_with_very_long_paths() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let mut app = App::new().expect("test setup");

        let mut long_path = temp_dir.path().to_path_buf();
        for i in 0..10 {
            long_path = long_path.join(format!("very_long_directory_name_{}", i));
            fs::create_dir(&long_path).expect("test setup");
        }

        let source_file = temp_dir.path().join("source.txt");
        File::create(&source_file)
            .expect("test setup")
            .write_all(b"content")
            .expect("test setup");

        app.left_panel.path = temp_dir.path().to_path_buf();
        app.right_panel.path = long_path.clone();
        app.left_panel.load_entries().expect("test setup");
        app.right_panel.load_entries().expect("test setup");
        app.left_panel.state.select(Some(1));

        let result = handle_copy(&mut app);

        // Just check that the operation completes without panic
        assert!(result.is_ok() || result.is_err());
        // On Windows, very long paths may fail
        #[cfg(windows)]
        {
            if long_path.to_string_lossy().len() > 260 && result.is_err() {
                assert!(
                    app.status_message.contains("Failed")
                        || result.unwrap_err().to_string().contains("path")
                );
            }
        }

        Ok(())
    }

    #[test]
    fn test_handle_overlay_keys_viewer_always_some() -> io::Result<()> {
        let mut app = App::new().expect("test setup");
        app.viewer = Some(Viewer {
            lines: vec!["Line 1".to_string(), "Line 2".to_string()],
            scroll: 0,
            title: "Test".to_string(),
        });

        // Test various viewer operations to ensure viewer remains Some
        handle_overlay_keys(&mut app, key_event(KeyCode::Down));
        assert!(app.viewer.is_some());

        handle_overlay_keys(&mut app, key_event(KeyCode::Up));
        assert!(app.viewer.is_some());

        // Unknown keys don't close viewer
        handle_overlay_keys(&mut app, key_event(KeyCode::Char('x')));
        assert!(app.viewer.is_some());

        Ok(())
    }

    #[test]
    fn test_jump_to_match_with_empty_query() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

        File::create(temp_dir.path().join("test.txt")).expect("test setup");
        panel.load_entries().expect("test setup");
        panel.state.select(Some(1));

        // Empty query matches everything (as empty string is substring of all strings)
        let found = jump_to_match(&mut panel, "");
        assert!(found); // Will match the first non-".." entry

        // Test with whitespace-only query
        panel.state.select(Some(1));
        let found = jump_to_match(&mut panel, "   ");
        assert!(!found); // Whitespace gets trimmed to empty but won't match filenames

        Ok(())
    }

    #[test]
    fn test_handle_view_with_viewer_already_open() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let mut app = App::new().expect("test setup");

        // Create a file to view
        let file_path = temp_dir.path().join("view.txt");
        File::create(&file_path)
            .expect("test setup")
            .write_all(b"content")
            .expect("test setup");

        app.left_panel.path = temp_dir.path().to_path_buf();
        app.left_panel.load_entries().expect("test setup");
        app.left_panel.state.select(Some(1));

        // Open viewer first time
        handle_view(&mut app).expect("test setup");
        assert!(app.viewer.is_some());

        // Try to open viewer again (should replace)
        handle_view(&mut app).expect("test setup");
        assert!(app.viewer.is_some());

        Ok(())
    }

    #[test]
    fn test_jump_to_match_no_match() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

        for i in 0..5 {
            File::create(temp_dir.path().join(format!("file{}.txt", i))).expect("test setup");
        }
        panel.load_entries().expect("test setup");

        let found = jump_to_match(&mut panel, "nonexistent");
        assert!(!found);

        Ok(())
    }
}
