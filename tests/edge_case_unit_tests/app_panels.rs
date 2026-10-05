//! Purpose: Active and inactive panels, swapping, and Enter where it does nothing.

use super::*;

// ---------------------------------------------------------------------------
// 7. App::get_active_panel_ref and get_inactive_panel_ref
// ---------------------------------------------------------------------------

#[test]
fn test_app_active_and_inactive_panel_ref() -> io::Result<()> {
    let app = App::new()?;
    let active = app.get_active_panel_ref();
    let inactive = app.get_inactive_panel_ref();
    // Active is left by default, inactive is right
    assert_eq!(active.path, app.left_panel.path);
    assert_eq!(inactive.path, app.right_panel.path);

    Ok(())
}

#[test]
fn test_app_active_panel_ref_after_switch() -> io::Result<()> {
    let mut app = App::new()?;
    app.active_panel = PanelSide::Right;

    let active = app.get_active_panel_ref();
    let inactive = app.get_inactive_panel_ref();
    assert_eq!(active.path, app.right_panel.path);
    assert_eq!(inactive.path, app.left_panel.path);

    Ok(())
}

// ---------------------------------------------------------------------------
// 17. App::swap_panels actually exchanges paths
// ---------------------------------------------------------------------------

#[test]
fn test_swap_panels_exchanges_paths() -> io::Result<()> {
    let temp1 = TempDir::new()?;
    let temp2 = TempDir::new()?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp1.path().to_path_buf())?;
    app.right_panel = Panel::new(temp2.path().to_path_buf())?;

    let left_path = app.left_panel.path.clone();
    let right_path = app.right_panel.path.clone();

    app.swap_panels();

    assert_eq!(app.left_panel.path, right_path);
    assert_eq!(app.right_panel.path, left_path);

    Ok(())
}

// ---------------------------------------------------------------------------
// 18. Panel::enter on non-directory is a no-op
// ---------------------------------------------------------------------------

#[test]
fn test_enter_on_file_is_noop() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    File::create(temp_dir.path().join("file.txt"))?;

    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;
    // Select the file entry
    let file_idx = panel
        .entries
        .iter()
        .position(|e| e.name == "file.txt")
        .unwrap();
    panel.select(Some(file_idx));

    let path_before = panel.path.clone();
    panel.enter()?;
    assert_eq!(panel.path, path_before);

    Ok(())
}

// ---------------------------------------------------------------------------
// 19. Panel::enter with no selection is a no-op
// ---------------------------------------------------------------------------

#[test]
fn test_enter_with_no_selection_is_noop() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;
    panel.select(None);

    let path_before = panel.path.clone();
    panel.enter()?;
    assert_eq!(panel.path, path_before);

    Ok(())
}
