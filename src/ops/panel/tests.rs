//! Purpose: a panel whose folder was deleted gets out of it, as Far's does —
//! to the nearest folder above it that still exists.

use std::{fs, io, path::Path};

use tempfile::TempDir;

use crate::models::{App, Panel};

/// A panel sitting in `root/gone/inner`, with `root/gone` then deleted.
fn stranded(root: &Path) -> io::Result<Panel> {
    let inner = root.join("gone").join("inner");
    fs::create_dir_all(&inner)?;
    fs::write(inner.join("g.txt"), "g")?;
    let panel = Panel::new(inner)?;
    fs::remove_dir_all(root.join("gone"))?;
    Ok(panel)
}

fn lists(panel: &Panel, name: &str) -> bool {
    panel.entries.iter().any(|entry| entry.name == name)
}

/// Ctrl+R used to say "Refresh failed" and keep listing the deleted files.
#[test]
fn refreshing_a_deleted_folder_moves_to_the_nearest_existing_one() -> io::Result<()> {
    let _lock = crate::utils::refresh_panels_mutex()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let temp = TempDir::new()?;
    fs::write(temp.path().join("kept.txt"), "k")?;
    let left = stranded(temp.path())?;
    let right = Panel::new(temp.path().to_path_buf())?;
    let mut app = App::with_panels(left, right);

    crate::utils::refresh_panels(&mut app)?;

    assert_eq!(app.left_panel.path, temp.path());
    assert!(!lists(&app.left_panel, "g.txt"), "the deleted file is gone");
    assert!(lists(&app.left_panel, "kept.txt"));
    Ok(())
}

/// Enter on `..` used to try only the direct parent, which was deleted too.
#[test]
fn going_up_from_a_deleted_folder_skips_the_deleted_parents() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut panel = stranded(temp.path())?;
    panel.select(Some(0));
    assert_eq!(
        panel.get_selected_entry().map(|e| e.name.as_str()),
        Some("..")
    );

    panel.enter()?;

    assert_eq!(panel.path, temp.path());
    assert!(!lists(&panel, "gone"));
    Ok(())
}

/// Going up still puts the cursor on the folder that was left, and a folder
/// that merely lost its contents is not skipped.
#[test]
fn going_up_normally_still_lands_on_the_folder_left() -> io::Result<()> {
    let temp = TempDir::new()?;
    let inner = temp.path().join("outer").join("inner");
    fs::create_dir_all(&inner)?;
    let mut panel = Panel::new(inner)?;
    panel.select(Some(0));

    panel.enter()?;

    assert_eq!(panel.path, temp.path().join("outer"));
    let cursor = panel.get_selected_entry().map(|entry| entry.name.as_str());
    assert_eq!(cursor, Some("inner"));
    Ok(())
}

/// The case that needs no outside shell: F8 in the right panel deletes the
/// folder the left panel sits in, and the left panel must not be left behind.
#[test]
fn deleting_the_other_panels_folder_moves_that_panel_out() -> io::Result<()> {
    let _lock = crate::utils::refresh_panels_mutex()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let temp = TempDir::new()?;
    let inner = temp.path().join("gone5").join("inner");
    fs::create_dir_all(&inner)?;
    fs::write(inner.join("z.txt"), "z")?;
    let mut app = App::with_panels(Panel::new(inner)?, Panel::new(temp.path().to_path_buf())?);
    app.active_panel = crate::models::PanelSide::Right;
    assert!(app.right_panel.select_named("gone5"));

    crate::ops::handle_delete_confirm(&mut app)?;
    crate::ops::handle_delete(&mut app)?;

    assert_eq!(app.left_panel.path, temp.path());
    assert!(!lists(&app.left_panel, "z.txt"));
    Ok(())
}
