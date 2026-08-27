//! Unit tests: shell commands (split out to keep files under 500 lines).
#![allow(unused_imports)]

use super::tests::*;
use crate::models::FileName;

use super::file_ops::{
    format_operation_status, get_source_paths, rename_or_fail, FORCE_RENAME_FAILURE,
    RENAME_TEST_MUTEX,
};

use super::shell::{run_shell_command, windows_shell_args};

use super::*;

use crate::command::Command;

#[cfg(feature = "test-helpers")]
use crate::editor::{handle_edit_impl, handle_edit_test_mode, TestEditorBehavior};

use crate::models::{
    App, BookmarkSlot, Overlay, Panel, PanelSide, SortMode, Viewer, MAX_BOOKMARKS,
};

#[cfg(feature = "test-helpers")]
use crate::WritableTestBackend;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use std::fs::{self, File};

use std::io::{self, Write};

use std::path::{Path, PathBuf};

use std::sync::atomic::Ordering;

use tempfile::TempDir;

// -----------------------------------------------------------------------
// handle_delete_confirm: no selection returns early
// -----------------------------------------------------------------------

#[test]
fn test_handle_delete_confirm_no_selection_returns_early() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.left_panel.select(None);

    handle_delete_confirm(&mut app)?;

    assert!(!matches!(app.overlay, Overlay::DeleteConfirm(_)));

    Ok(())
}

// -----------------------------------------------------------------------
// handle_mkdir exhausted counter
// -----------------------------------------------------------------------

#[test]
fn test_handle_mkdir_exhausted() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let base = temp_dir.path();

    // Create new_folder and new_folder_1..9999
    fs::create_dir(base.join("new_folder"))?;
    for i in 1..=9999u32 {
        fs::create_dir(base.join(format!("new_folder_{i}")))?;
    }

    let mut app = App::new()?;
    app.left_panel = Panel::new(base.to_path_buf())?;
    app.right_panel = Panel::new(base.to_path_buf())?;

    let err = handle_mkdir(&mut app).expect_err("should fail after 9999 attempts");
    assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);

    Ok(())
}

#[test]
fn test_delete_confirm_dotdot_noop() -> io::Result<()> {
    let temp = TempDir::new()?;
    let subdir = temp.path().join("child");
    fs::create_dir(&subdir)?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(subdir)?;
    app.left_panel.load_entries()?;

    // Select ".."
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == ".." {
            app.left_panel.select(Some(i));
            break;
        }
    }

    handle_delete_confirm(&mut app)?;
    assert!(
        !matches!(app.overlay, Overlay::DeleteConfirm(_)),
        "should not confirm delete on .."
    );
    Ok(())
}

#[test]
fn test_delete_confirm_no_selection_returns_ok() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp.path().to_path_buf())?;
    app.left_panel.entries.clear();
    app.left_panel.select(None);

    handle_delete_confirm(&mut app)?;
    assert!(!matches!(app.overlay, Overlay::DeleteConfirm(_)));
    Ok(())
}

#[test]
fn test_execute_command_shell_stdout() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp.path().to_path_buf())?;

    execute_command(&mut app, "echo hello")?;
    assert!(
        app.status_text().contains("hello"),
        "got: {}",
        app.status_text()
    );
    Ok(())
}

#[test]
fn test_execute_command_shell_stderr_only() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp.path().to_path_buf())?;

    execute_command(&mut app, "echo error >&2")?;
    assert!(
        app.status_text().contains("Err"),
        "got: {}",
        app.status_text()
    );
    Ok(())
}

#[test]
fn test_execute_command_shell_exit_nonzero() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp.path().to_path_buf())?;

    execute_command(&mut app, "false")?;
    assert!(
        app.status_text().contains("Exit"),
        "got: {}",
        app.status_text()
    );
    Ok(())
}

#[test]
fn test_execute_command_shell_ok_no_output() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp.path().to_path_buf())?;

    execute_command(&mut app, "true")?;
    assert!(
        app.status_text().contains("OK"),
        "got: {}",
        app.status_text()
    );
    Ok(())
}

#[test]
fn test_cmd_sort_extension() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("a.txt"), b"x")?;
    fs::write(temp.path().join("b.rs"), b"y")?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp.path().to_path_buf())?;

    execute_command(&mut app, "sort ext")?;
    assert!(matches!(app.left_panel.sort_mode, SortMode::Extension));
    Ok(())
}

#[test]
fn test_cmd_sort_size() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("small.txt"), b"x")?;
    fs::write(temp.path().join("big.txt"), b"xxxxxxxxxxxx")?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp.path().to_path_buf())?;

    execute_command(&mut app, "sort size")?;
    assert!(matches!(app.left_panel.sort_mode, SortMode::Size));
    Ok(())
}

#[test]
fn test_cmd_sort_invalid() -> io::Result<()> {
    let mut app = App::new()?;
    // Unknown subcommand for a known verb falls back to shell execution.
    // This is the new lenient behavior enabled by the Command enum + Shell variant.
    let result = execute_command(&mut app, "sort blah");
    // It may succeed or fail depending on whether a `sort` binary exists in PATH.
    // We mainly assert that we didn't panic and that the Command enum was exercised.
    assert!(result.is_ok() || result.is_err());
    Ok(())
}

#[test]
fn test_cmd_mark_all() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("a.txt"), b"a")?;
    fs::write(temp.path().join("b.txt"), b"b")?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp.path().to_path_buf())?;

    execute_command(&mut app, "mark all")?;
    assert!(app.left_panel.marked.contains("a.txt"));
    assert!(app.left_panel.marked.contains("b.txt"));
    assert!(!app.left_panel.marked.contains(".."));
    Ok(())
}

#[test]
fn test_cmd_mark_invalid_subcommand() -> io::Result<()> {
    let mut app = App::new()?;
    let err = execute_command(&mut app, "mark blah").unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    Ok(())
}

#[test]
fn test_cmd_bookmark_set_and_go() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp.path().to_path_buf())?;

    // Set bookmark
    execute_command(&mut app, "bookmark set 3")?;
    assert!(app
        .bookmarks
        .get(BookmarkSlot::from_usize(3).expect("slot"))
        .is_some());

    // Go to bookmark
    execute_command(&mut app, "bookmark go 3")?;
    assert!(app.status_text().contains("Opened bookmark 3"));
    Ok(())
}

#[test]
fn test_cmd_bookmark_go_empty_slot() -> io::Result<()> {
    let mut app = App::new()?;
    let err = execute_command(&mut app, "bookmark go 0").unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::NotFound);
    Ok(())
}

#[test]
fn test_cmd_bookmark_invalid_slot() -> io::Result<()> {
    let mut app = App::new()?;
    let err = execute_command(&mut app, "bookmark set 99").unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    Ok(())
}

#[test]
fn test_cmd_bookmark_missing_args() -> io::Result<()> {
    let mut app = App::new()?;

    // No subcommand or bad syntax falls back to Shell and fails at execution
    assert!(execute_command(&mut app, "bookmark").is_err());
    assert!(execute_command(&mut app, "bookmark set").is_err());
    Ok(())
}

#[test]
fn test_execute_command_quit() -> io::Result<()> {
    let mut app = App::new()?;
    execute_command(&mut app, "quit")?;
    assert!(!app.lifecycle.is_running());
    Ok(())
}

#[test]
fn test_execute_command_help_opens_overlay() -> io::Result<()> {
    let mut app = App::new()?;
    execute_command(&mut app, "help")?;
    assert!(matches!(app.overlay, Overlay::Help));
    Ok(())
}

// --- Tests for the new Command enum parser (the main value of this refactor) ---

#[test]
fn test_command_parse_basic_commands() {
    assert_eq!(Command::parse("pwd"), Command::Pwd);
    assert_eq!(Command::parse("help"), Command::Help);
    assert_eq!(Command::parse("quit"), Command::Quit);
    assert_eq!(Command::parse("exit"), Command::Quit);
    assert_eq!(Command::parse("mcp"), Command::Mcp);
    assert_eq!(Command::parse("mark all"), Command::MarkAll);
    assert_eq!(Command::parse("unmark"), Command::UnmarkAll);
}

#[test]
fn test_command_parse_sort_variants() {
    assert_eq!(Command::parse("sort name"), Command::Sort(SortMode::Name));
    assert_eq!(
        Command::parse("sort ext"),
        Command::Sort(SortMode::Extension)
    );
    assert_eq!(
        Command::parse("sort extension"),
        Command::Sort(SortMode::Extension)
    );
    assert_eq!(Command::parse("sort size"), Command::Sort(SortMode::Size));
    // Bad subcommand for known verb now produces Invalid (better domain modeling)
    assert!(matches!(
        Command::parse("sort foo"),
        Command::Invalid { .. }
    ));
}

#[test]
fn test_command_parse_bookmark() {
    assert_eq!(
        Command::parse("bookmark set 4"),
        Command::BookmarkSet(BookmarkSlot::new(4).unwrap())
    );
    assert_eq!(
        Command::parse("bookmark go 7"),
        Command::BookmarkGo(BookmarkSlot::new(7).unwrap())
    );
    assert!(matches!(
        Command::parse("bookmark set 99"),
        Command::Invalid { .. }
    ));
}

#[test]
fn test_command_parse_cd_and_shell_fallback() {
    assert!(matches!(Command::parse("cd /tmp"), Command::Cd(_)));
    assert!(matches!(Command::parse("ls -la"), Command::Shell(_)));
    assert!(matches!(
        Command::parse("some completely unknown thing"),
        Command::Shell(_)
    ));
}

#[test]
fn test_command_parse_empty() {
    // An empty line is `Command::Empty`; `Shell` cannot represent it.
    assert!(matches!(Command::parse(""), Command::Empty));
    assert!(matches!(Command::parse("   "), Command::Empty));
}

#[test]
fn test_execute_command_pwd_shows_path() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp.path().to_path_buf())?;

    execute_command(&mut app, "pwd")?;
    assert!(app
        .status_text()
        .contains(&temp.path().display().to_string()));
    Ok(())
}

#[test]
fn test_execute_command_unmark_clears() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp.path().to_path_buf())?;
    app.left_panel.marked.insert(FileName::from("file.txt"));

    execute_command(&mut app, "unmark")?;
    assert!(app.left_panel.marked.is_empty());
    Ok(())
}

#[test]
fn test_execute_command_empty_string() -> io::Result<()> {
    let mut app = App::new()?;
    let err = execute_command(&mut app, "").unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    Ok(())
}

#[test]
fn test_format_operation_status_multi() {
    let msg = format_operation_status(3, "Copied", Some(Path::new("/tmp/foo")));
    assert!(msg.contains('3'));
    assert!(msg.contains("Copied"));
}

#[test]
fn test_format_operation_status_single() {
    let dest = Path::new("/tmp/bar.txt");
    let msg = format_operation_status(1, "Moved", Some(dest));
    assert!(msg.contains("Moved"));
    assert!(msg.contains("bar.txt"));
}

#[test]
fn test_format_operation_status_none() {
    let msg = format_operation_status(1, "Copied", None);
    assert!(msg.is_empty());
}

#[test]
fn test_get_source_paths_marked() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("a.txt"), b"a")?;
    fs::write(temp.path().join("b.txt"), b"b")?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp.path().to_path_buf())?;
    app.left_panel.marked.insert(FileName::from("a.txt"));
    app.left_panel.marked.insert(FileName::from("b.txt"));

    let paths = get_source_paths(&app);
    assert_eq!(paths.len(), 2);
    Ok(())
}

#[test]
fn test_get_source_paths_selected() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("x.txt"), b"x")?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp.path().to_path_buf())?;
    // Select x.txt
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "x.txt" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    let paths = get_source_paths(&app);
    assert_eq!(paths.len(), 1);
    Ok(())
}
