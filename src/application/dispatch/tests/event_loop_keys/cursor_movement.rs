//! Purpose: cursor movement one row and one page at a time.

use super::*;

#[test]
fn test_navigation_moves_one_item() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    // Create multiple files to test navigation
    for i in 1..=5 {
        fs::write(temp_dir.path().join(format!("file{i}.txt")), b"data").expect("test setup");
    }

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    // Start at first item
    app.left_panel.select(Some(0));

    // Move down once
    app.left_panel.next();
    assert_eq!(
        app.left_panel.selected(),
        Some(1),
        "Should move to second item"
    );

    // Move down again
    app.left_panel.next();
    assert_eq!(
        app.left_panel.selected(),
        Some(2),
        "Should move to third item"
    );

    // Move up once
    app.left_panel.previous();
    assert_eq!(
        app.left_panel.selected(),
        Some(1),
        "Should move back to second item"
    );

    Ok(())
}

#[test]
fn test_apply_action_pages_selection_by_one_screen() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    for i in 0..40 {
        fs::write(temp_dir.path().join(format!("file{i:03}.txt")), b"x")?;
    }

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.active_panel = PanelSide::Left;
    app.left_panel.select(Some(0));

    // 80x24 terminal => 24 - 3 chrome (menu bar, command line, key bar)
    // - 2 borders - 3 in-panel rows (header, rule, info) = 16 visible rows,
    // so one page down lands on the entry just below the last visible one.
    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend)?;

    apply_action(&mut terminal, &mut app, Action::PageDown);
    assert_eq!(app.left_panel.selected(), Some(16));

    apply_action(&mut terminal, &mut app, Action::PageUp);
    assert_eq!(app.left_panel.selected(), Some(0));

    Ok(())
}
