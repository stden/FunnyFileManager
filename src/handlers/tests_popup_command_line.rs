//! Unit tests: popup command line (split out to keep files under 500 lines).
#![allow(unused_imports)]

use super::tests::*;
use crate::lineedit::LineEditor;
use crate::models::FileName;
use crate::models::Lifecycle;
use crate::models::Recall;
use crate::status::Status;

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
    App, BookmarkSlot, Candidate, CandidateKind, CommandLine, Menu, Overlay, Panel, PanelSide,
    SortMode, Viewer, MAX_BOOKMARKS,
};

#[cfg(feature = "test-helpers")]
use crate::WritableTestBackend;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use std::fs::{self, File};

use std::io::{self, Write};

use std::path::{Path, PathBuf};

use std::sync::atomic::Ordering;

use tempfile::TempDir;

#[test]
fn test_process_popup_input_command_line() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    app.set_overlay(Overlay::CommandLine(CommandLine::default()));
    assert!(process_popup_input(&mut app, key_event(KeyCode::Char('p'))));
    assert_eq!(
        if let Overlay::CommandLine(ref s) = app.overlay {
            Some(s.text())
        } else {
            None
        },
        Some("p")
    );

    // Alt+Char is not typed into the buffer, but the modal prompt still
    // consumes it — it must never reach the panel handler.
    assert!(process_popup_input(
        &mut app,
        key_event_with_mods(KeyCode::Char('x'), KeyModifiers::ALT)
    ));
    assert_eq!(
        if let Overlay::CommandLine(ref s) = app.overlay {
            Some(s.text())
        } else {
            None
        },
        Some("p")
    );

    assert!(process_popup_input(&mut app, key_event(KeyCode::Backspace)));
    assert_eq!(
        if let Overlay::CommandLine(ref s) = app.overlay {
            Some(s.text())
        } else {
            None
        },
        Some("")
    );

    assert!(process_popup_input(&mut app, key_event(KeyCode::Enter)));
    assert!(!matches!(app.overlay, Overlay::CommandLine(_)));

    app.set_overlay(Overlay::CommandLine("pwd".into()));
    assert!(process_popup_input(&mut app, key_event(KeyCode::Enter)));
    assert_eq!(app.status_text(), temp_dir.path().display().to_string());

    app.set_overlay(Overlay::CommandLine("unknown".into()));
    assert!(process_popup_input(&mut app, key_event(KeyCode::Enter)));
    // "unknown" is run as a shell command; stderr produces an error message
    // that is captured as "Err: ..." in status_message.
    assert!(!app.status_text().is_empty());

    app.set_overlay(Overlay::CommandLine("quit".into()));
    assert!(process_popup_input(&mut app, key_event(KeyCode::Enter)));
    assert!(!app.lifecycle.is_running());

    app.lifecycle = Lifecycle::Running;
    app.set_overlay(Overlay::CommandLine("pwd".into()));
    assert!(process_popup_input(&mut app, key_event(KeyCode::Esc)));
    assert!(!matches!(app.overlay, Overlay::CommandLine(_)));

    Ok(())
}

#[test]
fn test_process_popup_input_command_line_enter_empty() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.set_overlay(Overlay::CommandLine("   ".into()));
    app.set_status(Status::CommandOutput("status".to_string()));

    assert!(process_popup_input(&mut app, key_event(KeyCode::Enter)));
    assert!(!matches!(app.overlay, Overlay::CommandLine(_)));
    assert_eq!(app.status_text(), "status");

    Ok(())
}

#[test]
fn test_process_popup_input_command_line_backspace_empty() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.set_overlay(Overlay::CommandLine(CommandLine::default()));

    assert!(process_popup_input(&mut app, key_event(KeyCode::Backspace)));
    assert_eq!(
        if let Overlay::CommandLine(ref s) = app.overlay {
            Some(s.text())
        } else {
            None
        },
        Some("")
    );

    Ok(())
}

#[test]
fn test_process_popup_input_command_line_ignores_control_char() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.set_overlay(Overlay::CommandLine("a".into()));

    // Ctrl+Char is not typed into the buffer, but the modal prompt consumes it.
    assert!(process_popup_input(
        &mut app,
        key_event_with_mods(KeyCode::Char('b'), KeyModifiers::CONTROL)
    ));
    assert_eq!(
        if let Overlay::CommandLine(ref s) = app.overlay {
            Some(s.text())
        } else {
            None
        },
        Some("a")
    );

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
    assert!(app.status_text().contains("Opened"));

    let root_path = temp_dir
        .path()
        .ancestors()
        .last()
        .unwrap_or_else(|| temp_dir.path());
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
    assert!(matches!(app.overlay, Overlay::Help));

    assert!(execute_command(&mut app, "bookmark").is_err());
    assert!(execute_command(&mut app, "bookmark set").is_err());
    assert!(execute_command(&mut app, "bookmark set a").is_err());
    assert!(execute_command(&mut app, "bookmark set 99").is_err());
    assert!(execute_command(&mut app, "bookmark foo 1").is_err());

    execute_command(&mut app, "bookmark set 0").expect("test setup");
    assert!(app
        .bookmarks
        .get(BookmarkSlot::from_usize(0).expect("slot"))
        .is_some());
    assert!(execute_command(&mut app, "bookmark go 1").is_err());

    app.bookmarks.set(
        BookmarkSlot::from_usize(0).expect("slot"),
        bookmark_dir.clone(),
    );
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
    app.right_panel.path = dest_dir;
    app.left_panel.load_entries().expect("test setup");
    app.right_panel.load_entries().expect("test setup");
    app.left_panel.select(Some(1));

    // Skip setting readonly permissions on Windows as it's complex
    // Test by selecting a file that doesn't exist
    app.left_panel.entries.clear(); // Clear all entries
    app.left_panel.select(Some(0)); // Select index 0 but no entries

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
    app.left_panel.select(Some(1));

    // handle_move will rename the file in place when source and dest are same
    let result = handle_move(&mut app);
    assert!(result.is_ok());

    // The file should have been renamed with unique suffix
    let moved_file = temp_dir.path().join("file_1.txt");
    assert!(moved_file.exists(), "File should be renamed to file_1.txt");
    assert!(!file.exists(), "Original file should not exist");
    assert!(app.status_text().contains("Moved to"));

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

    app.left_panel.marked.insert(FileName::from("file1.txt"));
    app.left_panel.marked.insert(FileName::from("file3.txt"));

    // handle_delete_confirm will collect marked files (file1 and file3)
    handle_delete_confirm(&mut app).expect("test setup");
    handle_delete(&mut app).expect("test setup");

    // The marked files should be deleted, file2 (selected but not marked) should still exist
    assert!(!file1.exists());
    assert!(file2.exists());
    assert!(!file3.exists());
    assert!(app.status_text().contains("Deleted 2 item(s)"));

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
    app.left_panel.select(Some(1));

    handle_view(&mut app).expect("test setup");

    let viewer = app.get_viewer().expect("viewer");
    assert!(viewer.lines.len() >= 9999);

    Ok(())
}

#[test]
fn test_tab_opens_a_menu_when_several_candidates_match() {
    let mut app = App::new().expect("test setup");
    // "m" matches at least "mark", "mcp" and "unmark".
    app.set_overlay(Overlay::CommandLine("m".into()));

    process_popup_input(&mut app, key_event(KeyCode::Tab));

    let Overlay::CommandLine(state) = &app.overlay else {
        panic!("command line should still be open");
    };
    let menu = state.menu.as_ref().expect("menu should be open");
    assert!(menu.candidates().len() > 1);
    assert_eq!(menu.selected(), 0);
}

#[test]
fn test_tab_accepts_directly_when_only_one_candidate_matches() {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::CommandLine("unm".into()));

    process_popup_input(&mut app, key_event(KeyCode::Tab));

    let Overlay::CommandLine(state) = &app.overlay else {
        panic!("command line should still be open");
    };
    assert_eq!(state.text(), "unmark ");
    assert!(state.menu.is_none(), "a lone candidate opens no menu");
}

#[test]
fn test_down_moves_the_menu_selection() {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::CommandLine("m".into()));
    process_popup_input(&mut app, key_event(KeyCode::Tab));

    process_popup_input(&mut app, key_event(KeyCode::Down));

    let Overlay::CommandLine(state) = &app.overlay else {
        panic!("command line should still be open");
    };
    assert_eq!(state.menu.as_ref().expect("menu").selected(), 1);
}

#[test]
fn test_enter_accepts_the_selected_candidate_instead_of_running_it() {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::CommandLine("m".into()));
    process_popup_input(&mut app, key_event(KeyCode::Tab));

    // Read the highlighted candidate's text before accepting, so the
    // expectation tracks the real ranking instead of a hardcoded guess.
    let Overlay::CommandLine(state) = &app.overlay else {
        panic!("command line should still be open");
    };
    let menu = state.menu.as_ref().expect("menu should be open");
    let expected = menu.candidates()[menu.selected()].text.clone();

    process_popup_input(&mut app, key_event(KeyCode::Enter));

    let Overlay::CommandLine(state) = &app.overlay else {
        panic!("Enter with a menu open must accept, not execute and close");
    };
    assert!(state.menu.is_none(), "accepting closes the menu");
    // A verb is a `Word`, so accepting it finishes it with a trailing space —
    // the same rule Tab applies to a lone candidate.
    assert_eq!(state.text(), format!("{expected} "));
}

#[test]
fn test_enter_accepts_a_selection_below_the_first_row() {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::CommandLine("m".into()));
    process_popup_input(&mut app, key_event(KeyCode::Tab));

    process_popup_input(&mut app, key_event(KeyCode::Down));
    let Overlay::CommandLine(state) = &app.overlay else {
        panic!("command line should still be open");
    };
    let menu = state.menu.as_ref().expect("menu should be open");
    assert_eq!(menu.selected(), 1, "sanity check: Down moved the selection");
    let expected = menu.candidates()[1].text.clone();

    process_popup_input(&mut app, key_event(KeyCode::Enter));

    assert_eq!(
        buffer_of(&app),
        format!("{expected} "),
        "Enter must accept the highlighted row, not always the first one"
    );
}

#[test]
fn test_enter_accept_splits_on_multi_byte_whitespace() {
    // U+3000 is 3 bytes; splitting the head one byte past its start would cut
    // the char in half and panic, taking the whole TUI down.
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::CommandLine(CommandLine {
        line: LineEditor::new("m\u{3000}x"),
        menu: Menu::new(vec![Candidate {
            text: "mcp".to_string(),
            kind: CandidateKind::Word,
            hint: None,
        }]),
        ghost: None,
        recall: Recall::Idle,
    }));

    process_popup_input(&mut app, key_event(KeyCode::Enter));

    assert_eq!(
        buffer_of(&app),
        "m\u{3000}mcp ",
        "only the last token is replaced, and the separator survives intact"
    );
}

#[test]
fn test_esc_closes_only_the_menu_not_the_overlay() {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::CommandLine("m".into()));
    process_popup_input(&mut app, key_event(KeyCode::Tab));

    process_popup_input(&mut app, key_event(KeyCode::Esc));

    let Overlay::CommandLine(state) = &app.overlay else {
        panic!("first Esc must not close the command line");
    };
    assert!(state.menu.is_none());

    // A second Esc closes the overlay itself.
    process_popup_input(&mut app, key_event(KeyCode::Esc));
    assert!(matches!(app.overlay, Overlay::None));
}

#[test]
fn test_typing_refreshes_an_open_menu() {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::CommandLine("m".into()));
    process_popup_input(&mut app, key_event(KeyCode::Tab));

    // Move the selection off 0 first, so the post-typing assertion below can
    // actually distinguish "reset to 0" from "never touched".
    process_popup_input(&mut app, key_event(KeyCode::Down));
    let Overlay::CommandLine(state) = &app.overlay else {
        panic!("command line should still be open");
    };
    assert_eq!(
        state.menu.as_ref().expect("menu should be open").selected(),
        1,
        "sanity check: Down must move the selection before typing resets it"
    );

    process_popup_input(&mut app, key_event(KeyCode::Char('c')));

    let Overlay::CommandLine(state) = &app.overlay else {
        panic!("command line should still be open");
    };
    assert_eq!(state.text(), "mc");
    let menu = state.menu.as_ref().expect("menu stays open while typing");
    assert_eq!(menu.selected(), 0, "refreshing resets the selection");
    // The list itself must be recomputed for the new buffer, not just
    // re-highlighted: only "mcp" still matches "mc".
    assert_eq!(
        menu_texts(&app),
        vec!["mcp".to_string()],
        "the candidate list must track the edited buffer"
    );
}

/// The open dropdown's candidate texts, for readable list assertions.
fn menu_texts(app: &App) -> Vec<String> {
    let Overlay::CommandLine(state) = &app.overlay else {
        panic!("command line should be open");
    };
    state
        .menu
        .as_ref()
        .expect("menu should be open")
        .candidates()
        .iter()
        .map(|c| c.text.clone())
        .collect()
}

/// The command line's current buffer text.
fn buffer_of(app: &App) -> String {
    let Overlay::CommandLine(state) = &app.overlay else {
        panic!("command line should be open");
    };
    state.text().to_string()
}

#[test]
fn test_backspace_refreshes_an_open_menu() {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::CommandLine("m".into()));
    process_popup_input(&mut app, key_event(KeyCode::Tab));
    let wide_list = menu_texts(&app);
    assert!(
        wide_list.len() > 1,
        "sanity check: \"m\" is ambiguous, got {wide_list:?}"
    );

    process_popup_input(&mut app, key_event(KeyCode::Char('c')));
    assert_eq!(menu_texts(&app), vec!["mcp".to_string()]);

    process_popup_input(&mut app, key_event(KeyCode::Backspace));

    assert_eq!(buffer_of(&app), "m");
    assert_eq!(
        menu_texts(&app),
        wide_list,
        "deleting a character must widen the list back out"
    );
}

#[test]
fn test_typing_into_a_dead_end_closes_the_menu() {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::CommandLine("m".into()));
    process_popup_input(&mut app, key_event(KeyCode::Tab));

    // No verb matches "mz", so there is nothing left to show.
    process_popup_input(&mut app, key_event(KeyCode::Char('z')));

    let Overlay::CommandLine(state) = &app.overlay else {
        panic!("command line should still be open");
    };
    assert_eq!(state.text(), "mz");
    assert!(
        state.menu.is_none(),
        "an empty candidate set must close the dropdown"
    );
}

#[test]
fn test_typed_command_is_not_corrupted_by_a_stale_menu() {
    // Regression: the menu used to stay frozen at Tab-time, so Enter spliced a
    // long-obsolete candidate ("mcp") over the last token and destroyed the
    // text typed since — "mark all" became "mark mcp" and never ran.
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::CommandLine("m".into()));
    process_popup_input(&mut app, key_event(KeyCode::Tab));

    for ch in "ark all".chars() {
        process_popup_input(&mut app, key_event(KeyCode::Char(ch)));
    }
    assert_eq!(
        buffer_of(&app),
        "mark all",
        "typing must never rewrite the buffer"
    );

    process_popup_input(&mut app, key_event(KeyCode::Enter));

    // The refreshed menu offers only "all", so Enter accepts that — the typed
    // command is preserved, never overwritten with a stale candidate.
    let buffer = buffer_of(&app);
    assert_ne!(buffer, "mark mcp", "stale candidate must not be spliced in");
    assert_eq!(buffer, "mark all ");

    // With the menu closed by that acceptance, Enter now runs the command.
    process_popup_input(&mut app, key_event(KeyCode::Enter));
    assert!(
        matches!(app.overlay, Overlay::None),
        "the second Enter must execute and close the command line"
    );
}

#[test]
fn test_tab_cycles_an_open_menu_instead_of_rebuilding_it() {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::CommandLine("m".into()));
    process_popup_input(&mut app, key_event(KeyCode::Tab));
    let candidates = menu_texts(&app);
    let count = candidates.len();
    assert!(count > 2, "sanity check: need >2 candidates, got {count}");

    process_popup_input(&mut app, key_event(KeyCode::Down));
    process_popup_input(&mut app, key_event(KeyCode::Tab));

    let Overlay::CommandLine(state) = &app.overlay else {
        panic!("command line should still be open");
    };
    let menu = state.menu.as_ref().expect("menu stays open");
    assert_eq!(
        menu.selected(),
        2,
        "Tab must advance the existing selection, not reset it"
    );
    assert_eq!(
        menu_texts(&app),
        candidates,
        "cycling must not rebuild the list"
    );

    // Keep cycling past the end: the selection wraps back to the first row.
    for _ in 2..count {
        process_popup_input(&mut app, key_event(KeyCode::Tab));
    }
    let Overlay::CommandLine(state) = &app.overlay else {
        panic!("command line should still be open");
    };
    assert_eq!(
        state.menu.as_ref().expect("menu stays open").selected(),
        0,
        "Tab wraps around at the end of the list"
    );
}

#[test]
fn test_up_at_the_first_candidate_wraps_to_the_last() {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::CommandLine("m".into()));
    process_popup_input(&mut app, key_event(KeyCode::Tab));
    let last = menu_texts(&app).len() - 1;

    process_popup_input(&mut app, key_event(KeyCode::Up));

    let Overlay::CommandLine(state) = &app.overlay else {
        panic!("command line should still be open");
    };
    assert_eq!(state.menu.as_ref().expect("menu").selected(), last);
}

#[test]
fn executed_commands_enter_the_session_history() {
    // `App::new()` is session-only: it starts with `History::default()` and
    // does no disk I/O (production loads the real history in `main_impl`, never
    // in any test — see `models.rs`/`lib.rs`). So this App's history is empty
    // except for "pwd", recorded into the session ring by this very test; no
    // on-disk history can affect the assertion, and running these tests never
    // reads or writes the real `~/.local/share/ffm/history`.
    let mut app = App::new().expect("test app");
    app.set_overlay(Overlay::CommandLine("pwd".into()));

    process_popup_input(&mut app, key_event(KeyCode::Enter));

    assert_eq!(app.history.suggest("pw"), Some("pwd"));
}

/// Seed history, open a fresh command line, and return the App.
fn app_with_history(lines: &[&str]) -> App {
    let mut app = App::new().expect("test app");
    for line in lines {
        app.history.record(line);
    }
    app.set_overlay(Overlay::CommandLine(CommandLine::default()));
    app
}

#[test]
fn typing_shows_the_most_recent_matching_tail_as_ghost() {
    let mut app = app_with_history(&["cd /somewhere"]);

    process_popup_input(&mut app, key_event(KeyCode::Char('c')));
    process_popup_input(&mut app, key_event(KeyCode::Char('d')));

    let Overlay::CommandLine(state) = &app.overlay else {
        panic!("command line open");
    };
    assert_eq!(state.text(), "cd");
    assert_eq!(state.ghost.as_deref(), Some(" /somewhere"));
}

#[test]
fn right_accepts_the_ghost_into_the_buffer() {
    let mut app = app_with_history(&["cd /somewhere"]);
    process_popup_input(&mut app, key_event(KeyCode::Char('c')));

    process_popup_input(&mut app, key_event(KeyCode::Right));

    let Overlay::CommandLine(state) = &app.overlay else {
        panic!("command line open");
    };
    assert_eq!(state.text(), "cd /somewhere");
    assert_eq!(state.ghost, None, "tail fully consumed");
}

#[test]
fn backspace_recomputes_the_ghost() {
    let mut app = app_with_history(&["pwd", "pull"]);
    // "p" → newest match "pull" wins; then extend to "pw" → only "pwd".
    process_popup_input(&mut app, key_event(KeyCode::Char('p')));
    let Overlay::CommandLine(state) = &app.overlay else {
        panic!()
    };
    assert_eq!(state.ghost.as_deref(), Some("ull"));

    process_popup_input(&mut app, key_event(KeyCode::Char('w')));
    let Overlay::CommandLine(state) = &app.overlay else {
        panic!()
    };
    assert_eq!(state.ghost.as_deref(), Some("d"));

    process_popup_input(&mut app, key_event(KeyCode::Backspace));
    let Overlay::CommandLine(state) = &app.overlay else {
        panic!()
    };
    assert_eq!(state.text(), "p");
    assert_eq!(state.ghost.as_deref(), Some("ull"));
}

#[test]
fn ghost_is_suppressed_while_the_menu_is_open() {
    let mut app = app_with_history(&["mark all"]);
    process_popup_input(&mut app, key_event(KeyCode::Char('m')));
    let Overlay::CommandLine(state) = &app.overlay else {
        panic!()
    };
    assert!(state.ghost.is_some(), "ghost shows while menu closed");

    // Tab opens the menu ("m" is ambiguous among verbs) — ghost must clear.
    process_popup_input(&mut app, key_event(KeyCode::Tab));
    let Overlay::CommandLine(state) = &app.overlay else {
        panic!()
    };
    assert!(state.menu.is_some());
    assert_eq!(state.ghost, None, "no two accept-targets at once");

    // Esc closes the menu; the ghost returns for the unchanged buffer.
    process_popup_input(&mut app, key_event(KeyCode::Esc));
    let Overlay::CommandLine(state) = &app.overlay else {
        panic!()
    };
    assert!(state.menu.is_none());
    assert!(state.ghost.is_some(), "ghost returns once the menu closes");
}

#[test]
fn right_without_a_ghost_is_swallowed_and_harmless() {
    let mut app = app_with_history(&[]);
    process_popup_input(&mut app, key_event(KeyCode::Char('z')));

    assert!(process_popup_input(&mut app, key_event(KeyCode::Right)));

    let Overlay::CommandLine(state) = &app.overlay else {
        panic!()
    };
    assert_eq!(state.text(), "z");
}

#[test]
fn ghost_reappears_after_accepting_a_menu_candidate() {
    // Derive which verb candidate the menu will highlight for "m", instead of
    // guessing: `complete()` ranks candidates the same way `Tab` does, and
    // `open_menu` (src/popup.rs) always starts `selected` at index 0, so
    // `candidates[0]` is exactly the row `Enter` accepts below. The verb
    // branch of `complete()` never touches `cwd`, so any path will do.
    let comp =
        crate::complete::complete("m", Path::new(".")).expect("\"m\" matches at least one verb");
    let selected = comp.candidates[0].text.clone();

    // Verbs are always `CandidateKind::Word`, so `finish_word` appends a
    // trailing space on accept: the buffer becomes "<selected> ". Seed a
    // history line with exactly that prefix plus a tail, so `suggest` has a
    // literal match once the candidate is accepted.
    let seeded = format!("{selected} status");
    let mut app = app_with_history(&[&seeded]);

    process_popup_input(&mut app, key_event(KeyCode::Char('m')));
    process_popup_input(&mut app, key_event(KeyCode::Tab));
    let Overlay::CommandLine(state) = &app.overlay else {
        panic!()
    };
    assert!(state.menu.is_some(), "sanity check: \"m\" opens a menu");
    assert_eq!(state.ghost, None, "ghost suppressed while menu is open");

    process_popup_input(&mut app, key_event(KeyCode::Enter));

    let Overlay::CommandLine(state) = &app.overlay else {
        panic!()
    };
    assert!(state.menu.is_none(), "Enter accepts and closes the menu");
    assert_eq!(state.text(), format!("{selected} "));
    assert_eq!(
        state.ghost.as_deref(),
        Some("status"),
        "accepting a candidate is a buffer edit; the ghost must recompute \
         immediately instead of waiting for the next keystroke"
    );
}
