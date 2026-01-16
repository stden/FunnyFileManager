use funny_file_manager::handlers::{handle_overlay_keys, handle_view, handle_copy, handle_move, handle_mkdir, handle_delete};
use funny_file_manager::models::{App, Panel, PanelSide, SortMode, Viewer};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::fs::{self, File};
use std::io::{self, Write};
use tempfile::TempDir;

fn key_event(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::empty())
}

fn key_event_ctrl(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::CONTROL)
}

#[test]
fn test_jump_to_match() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    File::create(temp_dir.path().join("alpha.txt"))?;
    File::create(temp_dir.path().join("another-alpha.txt"))?;
    File::create(temp_dir.path().join("beta.txt"))?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.search = Some("alpha".to_string());

    // First match
    handle_overlay_keys(&mut app, key_event(KeyCode::Enter));
    assert_eq!(app.left_panel.state.selected(), Some(1));

    // Second match
    app.search = Some("alpha".to_string());
    handle_overlay_keys(&mut app, key_event(KeyCode::Enter));
    assert_eq!(app.left_panel.state.selected(), Some(2));

    Ok(())
}

#[test]
fn test_cd_dotdot() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let subdir = temp_dir.path().join("subdir");
    fs::create_dir(&subdir)?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(subdir)?;

    app.command_line = Some("cd ..".to_string());
    handle_overlay_keys(&mut app, key_event(KeyCode::Enter));

    assert_eq!(
        app.left_panel.path.canonicalize()?,
        temp_dir.path().canonicalize()?
    );

    Ok(())
}

#[test]
fn test_search_backspace() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    File::create(temp_dir.path().join("alpha.txt"))?;
    File::create(temp_dir.path().join("beta.txt"))?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.search = Some("bet".to_string());

    handle_overlay_keys(&mut app, key_event(KeyCode::Backspace));
    assert_eq!(app.search, Some("be".to_string()));

    handle_overlay_keys(&mut app, key_event(KeyCode::Backspace));
    handle_overlay_keys(&mut app, key_event(KeyCode::Backspace));
    assert_eq!(app.search, Some("".to_string()));

    handle_overlay_keys(&mut app, key_event(KeyCode::Backspace));
    assert_eq!(app.search, Some("".to_string()));

    Ok(())
}

#[test]
fn test_search_escape_cancels() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    File::create(temp_dir.path().join("test.txt"))?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.search = Some("test".to_string());

    handle_overlay_keys(&mut app, key_event(KeyCode::Esc));
    assert!(app.search.is_none());

    Ok(())
}

#[test]
fn test_search_enter_not_found() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    File::create(temp_dir.path().join("alpha.txt"))?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.search = Some("zzzzz".to_string());

    handle_overlay_keys(&mut app, key_event(KeyCode::Enter));
    assert!(app.search.is_none());
    assert!(app.status_message.contains("Not found"));

    Ok(())
}

#[test]
fn test_search_empty_query_enter() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    File::create(temp_dir.path().join("test.txt"))?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.search = Some("".to_string());

    handle_overlay_keys(&mut app, key_event(KeyCode::Enter));
    assert!(app.search.is_none());

    Ok(())
}

#[test]
fn test_viewer_scroll_down_up() -> io::Result<()> {
    let mut app = App::new()?;
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

    // Scroll down
    handle_overlay_keys(&mut app, key_event(KeyCode::Down));
    assert_eq!(app.viewer.as_ref().unwrap().scroll, 1);

    // Scroll up
    handle_overlay_keys(&mut app, key_event(KeyCode::Up));
    assert_eq!(app.viewer.as_ref().unwrap().scroll, 0);

    // Scroll up at 0 stays at 0
    handle_overlay_keys(&mut app, key_event(KeyCode::Up));
    assert_eq!(app.viewer.as_ref().unwrap().scroll, 0);

    Ok(())
}

#[test]
fn test_viewer_close_with_f3() -> io::Result<()> {
    let mut app = App::new()?;
    app.viewer = Some(Viewer {
        title: "test.txt".to_string(),
        lines: vec!["Line 1".to_string()],
        scroll: 0,
    });

    handle_overlay_keys(&mut app, key_event(KeyCode::F(3)));
    assert!(app.viewer.is_none());

    Ok(())
}

#[test]
fn test_help_toggle_with_f1() -> io::Result<()> {
    let mut app = App::new()?;
    app.show_help = true;

    handle_overlay_keys(&mut app, key_event(KeyCode::F(1)));
    assert!(!app.show_help);

    Ok(())
}

#[test]
fn test_bookmarks_set_and_go() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let bookmark_dir = temp_dir.path().join("bookmarked");
    fs::create_dir(&bookmark_dir)?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(bookmark_dir.clone())?;
    app.bookmarks = vec![None; 10];
    app.show_bookmarks = true;

    // Set bookmark with Ctrl+0
    handle_overlay_keys(&mut app, key_event_ctrl(KeyCode::Char('0')));
    assert!(app.bookmarks[0].is_some());
    assert_eq!(app.bookmarks[0].as_ref().unwrap(), &bookmark_dir);

    // Go back to temp_dir
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.show_bookmarks = true;

    // Go to bookmark 0
    handle_overlay_keys(&mut app, key_event(KeyCode::Char('0')));
    assert_eq!(app.left_panel.path, bookmark_dir);

    Ok(())
}

#[test]
fn test_bookmarks_close_with_f2() -> io::Result<()> {
    let mut app = App::new()?;
    app.show_bookmarks = true;

    handle_overlay_keys(&mut app, key_event(KeyCode::F(2)));
    assert!(!app.show_bookmarks);

    Ok(())
}

#[test]
fn test_command_line_pwd() -> io::Result<()> {
    let temp_dir = TempDir::new()?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.command_line = Some("pwd".to_string());

    handle_overlay_keys(&mut app, key_event(KeyCode::Enter));
    assert!(app.command_line.is_none());
    assert!(app.status_message.contains(temp_dir.path().to_str().unwrap()));

    Ok(())
}

#[test]
fn test_command_line_exit() -> io::Result<()> {
    let temp_dir = TempDir::new()?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.command_line = Some("exit".to_string());

    handle_overlay_keys(&mut app, key_event(KeyCode::Enter));
    assert!(app.should_quit);

    Ok(())
}

#[test]
fn test_command_line_unknown_command() -> io::Result<()> {
    let temp_dir = TempDir::new()?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.command_line = Some("unknowncommand".to_string());

    handle_overlay_keys(&mut app, key_event(KeyCode::Enter));
    assert!(app.status_message.contains("Command failed"));

    Ok(())
}

#[test]
fn test_handle_view_text_file() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test.txt");
    File::create(&file_path)?.write_all(b"Hello World")?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;

    // Select the file
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "test.txt" {
            app.left_panel.state.select(Some(i));
            break;
        }
    }

    handle_view(&mut app)?;

    assert!(app.viewer.is_some());
    assert_eq!(app.viewer.as_ref().unwrap().title, "test.txt");
    assert!(app.viewer.as_ref().unwrap().lines[0].contains("Hello World"));

    Ok(())
}

#[test]
fn test_handle_copy_file() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let left_dir = temp_dir.path().join("left");
    let right_dir = temp_dir.path().join("right");
    fs::create_dir(&left_dir)?;
    fs::create_dir(&right_dir)?;

    let source_file = left_dir.join("source.txt");
    File::create(&source_file)?.write_all(b"content")?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(left_dir)?;
    app.right_panel = Panel::new(right_dir.clone())?;

    // Select the file
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "source.txt" {
            app.left_panel.state.select(Some(i));
            break;
        }
    }

    handle_copy(&mut app)?;

    assert!(right_dir.join("source.txt").exists());
    assert!(app.status_message.contains("Copied to"));

    Ok(())
}

#[test]
fn test_handle_move_file() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let left_dir = temp_dir.path().join("left");
    let right_dir = temp_dir.path().join("right");
    fs::create_dir(&left_dir)?;
    fs::create_dir(&right_dir)?;

    let source_file = left_dir.join("moveme.txt");
    File::create(&source_file)?.write_all(b"content")?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(left_dir.clone())?;
    app.right_panel = Panel::new(right_dir.clone())?;

    // Select the file
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "moveme.txt" {
            app.left_panel.state.select(Some(i));
            break;
        }
    }

    handle_move(&mut app)?;

    assert!(!left_dir.join("moveme.txt").exists());
    assert!(right_dir.join("moveme.txt").exists());
    assert!(app.status_message.contains("Moved to"));

    Ok(())
}

#[test]
fn test_handle_mkdir_creates_folder() -> io::Result<()> {
    let temp_dir = TempDir::new()?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;

    handle_mkdir(&mut app)?;

    assert!(temp_dir.path().join("new_folder").exists());
    assert!(app.status_message.contains("Created directory"));

    Ok(())
}

#[test]
fn test_handle_delete_file() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("delete_me.txt");
    File::create(&file_path)?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;

    // Select the file
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "delete_me.txt" {
            app.left_panel.state.select(Some(i));
            break;
        }
    }

    handle_delete(&mut app)?;

    assert!(!file_path.exists());
    assert!(app.status_message.contains("Deleted"));

    Ok(())
}

#[test]
fn test_command_line_sort_extension() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    File::create(temp_dir.path().join("a.txt"))?;
    File::create(temp_dir.path().join("b.rs"))?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.command_line = Some("sort extension".to_string());

    handle_overlay_keys(&mut app, key_event(KeyCode::Enter));
    assert!(app.status_message.contains("Sort: Ext"));

    Ok(())
}

#[test]
fn test_command_line_sort_size_right_panel() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    File::create(temp_dir.path().join("small.txt"))?.write_all(b"1")?;
    File::create(temp_dir.path().join("large.txt"))?.write_all(b"12345")?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.active_panel = PanelSide::Right;
    app.command_line = Some("sort size".to_string());

    handle_overlay_keys(&mut app, key_event(KeyCode::Enter));
    assert!(matches!(app.right_panel.sort_mode, SortMode::Size));
    assert!(app.status_message.contains("Sort: Size"));

    Ok(())
}

#[test]
fn test_command_line_sort_invalid() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    File::create(temp_dir.path().join("a.txt"))?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.command_line = Some("sort nope".to_string());

    handle_overlay_keys(&mut app, key_event(KeyCode::Enter));
    assert!(app.status_message.contains("sort expects"));

    Ok(())
}

#[test]
fn test_command_line_mark_all() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    File::create(temp_dir.path().join("a.txt"))?;
    File::create(temp_dir.path().join("b.txt"))?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.command_line = Some("mark all".to_string());

    handle_overlay_keys(&mut app, key_event(KeyCode::Enter));
    assert!(!app.left_panel.marked.is_empty());
    assert!(app.status_message.contains("Marked"));

    Ok(())
}

#[test]
fn test_command_line_unmark() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    File::create(temp_dir.path().join("a.txt"))?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.left_panel.marked.insert("a.txt".to_string());
    app.command_line = Some("unmark".to_string());

    handle_overlay_keys(&mut app, key_event(KeyCode::Enter));
    assert!(app.left_panel.marked.is_empty());
    assert!(app.status_message.contains("Unmarked"));

    Ok(())
}

#[test]
fn test_command_line_help() -> io::Result<()> {
    let temp_dir = TempDir::new()?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.command_line = Some("help".to_string());

    handle_overlay_keys(&mut app, key_event(KeyCode::Enter));
    assert!(app.show_help);

    Ok(())
}
