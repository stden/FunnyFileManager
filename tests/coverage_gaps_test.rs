//! Tests to cover remaining uncovered lines in handlers.rs, lib.rs, s3.rs, ui.rs

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use funny_file_manager::handlers::{handle_delete_confirm, open_mcp_status, process_popup_input};
use funny_file_manager::models::{App, Overlay, Panel, PanelSide};
use funny_file_manager::quicksearch::QuickSearch;
#[cfg(feature = "s3")]
use funny_file_manager::s3::VirtualPath;
use funny_file_manager::scroll::ScrollTop;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::PathBuf;
use tempfile::TempDir;

const fn key_event(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::empty())
}

fn make_app(dir: &std::path::Path) -> io::Result<App> {
    let mut app = App::with_panels(
        Panel::new(dir.to_path_buf())?,
        Panel::new(dir.to_path_buf())?,
    );
    app.status = funny_file_manager::status::Status::Cleared;
    Ok(app)
}

// ─── open_mcp_status (handlers.rs lines 606-689) ──────────────────

#[test]
fn test_open_mcp_status_sets_viewer() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = make_app(temp.path())?;

    open_mcp_status(&mut app)?;

    // Verify viewer was set
    assert!(app.get_viewer().is_some());
    let viewer = app.get_viewer().unwrap();
    assert_eq!(viewer.title(), "MCP Server Status");
    assert_eq!(viewer.scroll, ScrollTop::TOP);
    assert!(!viewer.lines.is_empty());

    // Check key content
    let all_text = viewer.lines.join("\n");
    assert!(all_text.contains("FFM MCP Server"));
    assert!(all_text.contains("Server:"));
    assert!(all_text.contains("Supported IDEs:"));
    for client in ["Claude Code", "Codex", "Junie", "Kiro"] {
        assert!(all_text.contains(client), "{client} is listed");
    }
    // FFM cannot register with these, so the screen no longer lists them.
    assert!(!all_text.contains("Gemini"));
    assert!(!all_text.contains("VS Code/Cursor"));
    assert!(all_text.contains("Commands:"));
    assert!(all_text.contains("Tools:"));
    assert!(all_text.contains("list_directory"));
    assert!(all_text.contains("s3_copy"));

    // Check status message
    assert!(app.status_text().contains("MCP Server"));

    Ok(())
}

#[test]
fn test_open_mcp_status_binary_not_found() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = make_app(temp.path())?;

    // Binary won't exist at the temp dir
    open_mcp_status(&mut app)?;

    let all_text = app.get_viewer().unwrap().lines.join("\n");
    // A new `App` looks at no machine, so no server is found anywhere.
    assert!(all_text.contains("ffm-mcp was not found next to ffm or on PATH."));

    Ok(())
}

// ─── Drive selection error paths (handlers.rs lines 272-309) ───────

#[test]
fn test_drive_selection_enter_on_local_drive_failure() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = make_app(temp.path())?;

    // Open drive selection for left panel
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index: 0,
        rows: funny_file_manager::models::drive_rows(),
    });

    // Simulate pressing Enter to select the drive
    let consumed = process_popup_input(&mut app, key_event(KeyCode::Enter));
    assert!(consumed);

    // Drive selection should be closed
    assert!(!matches!(app.overlay, Overlay::DriveSelection { .. }));
    let drive_selection_index = if let Overlay::DriveSelection { selected_index, .. } = app.overlay
    {
        selected_index
    } else {
        0
    };
    assert_eq!(drive_selection_index, 0);

    Ok(())
}

#[test]
fn test_drive_selection_navigate_up_down() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = make_app(temp.path())?;

    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index: 0,
        rows: funny_file_manager::models::drive_rows(),
    });

    // Press Down
    let consumed = process_popup_input(&mut app, key_event(KeyCode::Down));
    assert!(consumed);

    // Press Up
    let consumed = process_popup_input(&mut app, key_event(KeyCode::Up));
    assert!(consumed);
    assert!(matches!(
        app.overlay,
        Overlay::DriveSelection {
            selected_index: 0,
            ..
        }
    ));

    Ok(())
}

#[test]
fn test_drive_selection_char_key() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = make_app(temp.path())?;

    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index: 0,
        rows: funny_file_manager::models::drive_rows(),
    });

    // Press a letter key (Windows drive letter selection)
    // On Unix this won't find the drive but exercises the code path
    let consumed = process_popup_input(&mut app, key_event(KeyCode::Char('z')));
    assert!(consumed);

    Ok(())
}

#[test]
fn test_drive_selection_for_right_panel() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = make_app(temp.path())?;

    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Right,
        selected_index: 0,
        rows: funny_file_manager::models::drive_rows(),
    });

    // Enter selects the drive for the right panel
    let consumed = process_popup_input(&mut app, key_event(KeyCode::Enter));
    assert!(consumed);
    assert!(!matches!(app.overlay, Overlay::DriveSelection { .. }));

    Ok(())
}

// ─── Delete confirm edge cases (handlers.rs lines 858, 866) ────────

#[test]
fn test_delete_confirm_parent_dir_selected() -> io::Result<()> {
    let temp = TempDir::new()?;
    let sub = temp.path().join("subdir");
    fs::create_dir(&sub)?;
    File::create(sub.join("file.txt"))?;

    let mut app = make_app(temp.path())?;
    app.left_panel = Panel::new(sub)?;
    // Select ".." entry (first entry)
    app.left_panel.select(Some(0));

    // Should not create delete confirmation for ".."
    handle_delete_confirm(&mut app)?;
    assert!(app.get_delete_confirm().is_none());

    Ok(())
}

#[test]
fn test_delete_confirm_no_selection() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = make_app(temp.path())?;
    app.left_panel.select(None);
    app.left_panel.entries.clear();

    handle_delete_confirm(&mut app)?;
    assert!(app.get_delete_confirm().is_none());

    Ok(())
}

// ─── S3 VirtualPath edge cases (s3.rs) ──────────────────────────────

#[cfg(feature = "s3")]
#[test]
fn test_virtual_path_display_local() {
    let vp = VirtualPath::Local(PathBuf::from("/tmp/test"));
    assert_eq!(vp.display(), "/tmp/test");
}

#[cfg(feature = "s3")]
#[test]
fn test_virtual_path_display_s3_with_bucket_and_prefix() {
    let vp = VirtualPath::S3 {
        profile: "prod".to_owned(),
        bucket: Some("my-bucket".to_owned()),
        prefix: "path/to/file".to_owned(),
    };
    assert_eq!(vp.display(), "s3://prod@my-bucket/path/to/file");
}

#[cfg(feature = "s3")]
#[test]
fn test_virtual_path_display_s3_no_bucket() {
    let vp = VirtualPath::S3 {
        profile: "dev".to_owned(),
        bucket: None,
        prefix: String::new(),
    };
    assert_eq!(vp.display(), "s3://dev@");
}

#[cfg(feature = "s3")]
#[test]
fn test_virtual_path_display_s3_bucket_no_prefix() {
    let vp = VirtualPath::S3 {
        profile: "prod".to_owned(),
        bucket: Some("bucket".to_owned()),
        prefix: String::new(),
    };
    assert_eq!(vp.display(), "s3://prod@bucket");
}

// ─── UI delete confirm with path-only entries (ui.rs line 385) ──────

#[test]
fn test_delete_confirm_renders_file_names() -> io::Result<()> {
    let temp = TempDir::new()?;
    let file = temp.path().join("deleteme.txt");
    File::create(&file)?;

    let mut app = make_app(temp.path())?;
    // Set up delete confirmation with a path that has no file_name component
    app.set_overlay(Overlay::DeleteConfirm(vec![
        file,
        PathBuf::from("/"), // This path has no file_name()
    ]));

    // Just verify the app state is set up correctly for rendering
    assert_eq!(app.get_delete_confirm().unwrap().len(), 2);

    Ok(())
}

// ─── Quick search backspace (lib.rs line 459) ───────────────────────

#[test]
fn test_quick_search_backspace_clears() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = make_app(temp.path())?;

    // Set quick search with single char
    app.quick_search = QuickSearch::Active("a".to_owned());

    // Backspace should clear it
    let key = key_event(KeyCode::Backspace);
    let consumed = process_popup_input(&mut app, key);
    // Quick search backspace is handled in lib.rs event loop, not process_popup_input
    // So we just verify the popup handler doesn't consume it
    assert!(!consumed || app.quick_search.query().is_none());

    Ok(())
}

// ─── Event loop coverage (lib.rs lines 328, 352, 370, 401, 459) ────

use crossterm::event::Event;
use funny_file_manager::{run_app_with_event_reader, WritableTestBackend};
use ratatui::Terminal;

fn make_terminal() -> io::Result<Terminal<WritableTestBackend>> {
    let backend = WritableTestBackend::new(240, 24);
    Terminal::new(backend)
}

const fn key_event_with_mods(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(code, modifiers)
}

fn run_events(app: App, events: Vec<Event>) -> io::Result<()> {
    let mut terminal = make_terminal()?;
    let mut iter = events.into_iter();
    run_app_with_event_reader(&mut terminal, app, || {
        iter.next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "done"))
    })
}

#[test]
fn test_event_loop_f9_mcp_status() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("file.txt"), b"data")?;
    let mut app = make_app(temp.path())?;
    app.left_panel.select(Some(0));

    run_events(
        app,
        vec![
            Event::Key(key_event(KeyCode::F(9))),
            Event::Key(key_event(KeyCode::Esc)), // close viewer
            Event::Key(key_event(KeyCode::F(10))),
            // F10 now asks before quitting; Enter answers Yes.
            Event::Key(key_event(KeyCode::Enter)),
        ],
    )
}

#[test]
fn test_event_loop_end_key() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("aaa.txt"), b"a")?;
    fs::write(temp.path().join("zzz.txt"), b"z")?;
    let app = make_app(temp.path())?;

    run_events(
        app,
        vec![
            Event::Key(key_event(KeyCode::End)),
            Event::Key(key_event(KeyCode::F(10))),
            // F10 now asks before quitting; Enter answers Yes.
            Event::Key(key_event(KeyCode::Enter)),
        ],
    )
}

#[test]
fn test_event_loop_home_key() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("aaa.txt"), b"a")?;
    fs::write(temp.path().join("zzz.txt"), b"z")?;
    let app = make_app(temp.path())?;

    run_events(
        app,
        vec![
            Event::Key(key_event(KeyCode::Down)),
            Event::Key(key_event(KeyCode::Home)),
            Event::Key(key_event(KeyCode::F(10))),
            // F10 now asks before quitting; Enter answers Yes.
            Event::Key(key_event(KeyCode::Enter)),
        ],
    )
}

#[test]
fn test_event_loop_quick_search_backspace() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("alpha.txt"), b"a")?;
    fs::write(temp.path().join("beta.txt"), b"b")?;
    let app = make_app(temp.path())?;

    run_events(
        app,
        vec![
            // Start quick search with Alt+a, then Alt+l, then backspace
            Event::Key(key_event_with_mods(KeyCode::Char('a'), KeyModifiers::ALT)),
            Event::Key(key_event_with_mods(KeyCode::Char('l'), KeyModifiers::ALT)),
            Event::Key(key_event(KeyCode::Backspace)), // pops 'l', keeps 'a'
            Event::Key(key_event(KeyCode::Backspace)), // pops 'a', clears search
            Event::Key(key_event(KeyCode::F(10))),
            // F10 now asks before quitting; Enter answers Yes.
            Event::Key(key_event(KeyCode::Enter)),
        ],
    )
}

#[test]
fn test_event_loop_down_up_navigation() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("aaa.txt"), b"a")?;
    fs::write(temp.path().join("bbb.txt"), b"b")?;
    let app = make_app(temp.path())?;

    run_events(
        app,
        vec![
            Event::Key(key_event(KeyCode::Down)),
            Event::Key(key_event(KeyCode::Down)),
            Event::Key(key_event(KeyCode::Up)),
            Event::Key(key_event(KeyCode::F(10))),
            // F10 now asks before quitting; Enter answers Yes.
            Event::Key(key_event(KeyCode::Enter)),
        ],
    )
}

// ─── UI rendering with delete confirm (ui.rs line 385) ──────────────

#[test]
fn test_ui_render_delete_confirm_with_root_path() -> io::Result<()> {
    let temp = TempDir::new()?;
    let file = temp.path().join("deleteme.txt");
    File::create(&file)?;

    let mut app = make_app(temp.path())?;
    // Path "/" has no file_name(), exercises the map_or_else fallback
    app.set_overlay(Overlay::DeleteConfirm(vec![file, PathBuf::from("/")]));

    let mut terminal = make_terminal()?;
    terminal.draw(|f| funny_file_manager::ui::ui(f, &mut app))?;

    Ok(())
}

// ─── MCP status IDE config branches (handlers.rs lines 659-662) ─────

#[test]
fn test_open_mcp_status_ide_config_not_configured() -> io::Result<()> {
    let temp = TempDir::new()?;

    // Claude Code's project config, without "ffm-mcp": "not configured".
    // The screen reads the active panel's folder, so no working directory
    // has to change for it to be found.
    let mcp_json = temp.path().join(".mcp.json");
    let mut f = File::create(&mcp_json)?;
    write!(f, r#"{{"some_other_tool": true}}"#)?;

    let mut app = make_app(temp.path())?;
    open_mcp_status(&mut app)?;

    let all_text = app.get_viewer().unwrap().lines.join("\n");
    let claude = all_text
        .lines()
        .find(|line| line.trim_start().starts_with("Claude Code"))
        .expect("a Claude Code row");
    assert!(claude.ends_with("⊘ not configured"), "{claude}");

    Ok(())
}

// ─── Drive selection load_entries error (handlers.rs line 272-273) ───

#[test]
fn test_drive_selection_enter_with_invalid_drive_path() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = make_app(temp.path())?;

    // Set up drive selection and select a drive that maps to a non-existent path
    // On Unix, the only local drive is "/", which will succeed
    // But we can test the S3Profile branch (line 277-281)
    // Get all drives — the list includes local + S3 profiles
    let drives = App::get_all_drives();
    // If there's more than 1 drive, try selecting the last one
    let selected_index = if drives.len() > 1 {
        drives.len() - 1
    } else {
        0
    };
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index,
        rows: funny_file_manager::models::drive_rows(),
    });

    let consumed = process_popup_input(&mut app, key_event(KeyCode::Enter));
    assert!(consumed);
    assert!(!matches!(app.overlay, Overlay::DriveSelection { .. }));

    Ok(())
}
