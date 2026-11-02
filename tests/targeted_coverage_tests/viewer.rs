//! Purpose: Opening the viewer and paging through it.

use super::*;

// ──────────────────────────────────────────────
// Viewer edge cases
// ──────────────────────────────────────────────

#[test]
fn test_view_empty_file() -> io::Result<()> {
    let temp = TempDir::new()?;
    File::create(temp.path().join("empty.txt"))?;

    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;

    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "empty.txt" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    handle_view(&mut app)?;
    assert!(app.get_viewer().is_some());
    let viewer = app.get_viewer().unwrap();
    assert_eq!(viewer.lines, vec!["<empty file>"]);

    Ok(())
}

#[test]
fn test_view_directory_rejected() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::create_dir(temp.path().join("subdir"))?;

    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;

    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "subdir" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    handle_view(&mut app)?;
    assert!(app.get_viewer().is_none());
    assert!(
        matches!(app.overlay, Overlay::None),
        "F3 on a directory measures it in-place without opening an overlay"
    );
    assert!(
        app.left_panel
            .get_selected_entry()
            .is_some_and(|entry| entry.directory_size.is_measured()),
        "F3 on a directory marks its size as measured"
    );

    Ok(())
}

#[test]
fn test_view_no_selection() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.entries.clear();
    app.left_panel.select(None);

    handle_view(&mut app)?;
    assert!(app.get_viewer().is_none());

    Ok(())
}

#[test]
fn test_viewer_page_down_up() -> io::Result<()> {
    let mut app = App::new()?;
    let lines: Vec<String> = (0..100).map(|i| format!("Line {i}")).collect();
    app.set_overlay(Overlay::Viewer(Viewer::report("big.txt", lines)));
    app.set_viewport(funny_file_manager::models::Viewport::new(2));

    // One body row, so the last line may reach the top; the on-screen clamp
    // is pinned by tests that draw a taller terminal first.
    // Scroll down multiple times
    for _ in 0..10 {
        process_popup_input(&mut app, key(KeyCode::Down));
    }
    let after_down = app.get_viewer().unwrap().scroll;
    assert_eq!(after_down.lines(), 10, "Should have scrolled down 10 lines");

    // Scroll up
    for _ in 0..5 {
        process_popup_input(&mut app, key(KeyCode::Up));
    }
    let after_up = app.get_viewer().unwrap().scroll;
    assert_eq!(after_up.lines(), 5, "Should have scrolled back to 5");

    // Verify max scroll cap
    for _ in 0..200 {
        process_popup_input(&mut app, key(KeyCode::Down));
    }
    assert_eq!(
        app.get_viewer().unwrap().scroll.lines(),
        99,
        "Should cap at last line"
    );

    Ok(())
}
