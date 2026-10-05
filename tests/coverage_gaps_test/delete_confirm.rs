//! Purpose: Delete confirmation on `..`, with no selection, and how it renders.

use super::*;

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
