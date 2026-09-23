//! Purpose: Ctrl+M (Restore selection) brings back only names of the listing
//! the panel shows, so F8 never offers to delete entries that are not there.
//!
//! The remembered selection is a set of bare names. Restored into another
//! folder, or after the files were deleted, those names point at nothing —
//! or at a namesake — and F8 joins them onto the current path.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use tempfile::TempDir;

use crate::mask::Mask;
use crate::models::{App, Overlay, Panel};

/// Both panels in `dir`, nothing persistent.
fn app_in(dir: &Path) -> io::Result<App> {
    Ok(App::with_panels(
        Panel::new(dir.to_path_buf())?,
        Panel::new(dir.to_path_buf())?,
    ))
}

fn write_files(dir: &Path, names: &[&str]) -> io::Result<()> {
    for name in names {
        fs::write(dir.join(name), name)?;
    }
    Ok(())
}

fn marked(app: &mut App) -> Vec<String> {
    let panel = app.get_active_panel();
    panel
        .marked
        .iter()
        .map(|name| name.as_str().to_owned())
        .collect()
}

/// What F8 asks to delete, with the dialog closed again.
fn offered_by_f8(app: &mut App) -> io::Result<Vec<PathBuf>> {
    crate::ops::handle_delete_confirm(app)?;
    let offered = match &app.overlay {
        Overlay::DeleteConfirm(paths) => paths.clone(),
        _ => Vec::new(),
    };
    app.close_overlay();
    Ok(offered)
}

/// The reported case: select everything, invert, enter a folder, restore.
#[test]
fn restoring_after_entering_a_folder_brings_back_nothing_of_the_folder_left() -> io::Result<()> {
    let temp = TempDir::new()?;
    write_files(temp.path(), &["broken", "loop-a", "loop-b"])?;
    let inner = temp.path().join("cycle");
    fs::create_dir(&inner)?;
    write_files(&inner, &["parent"])?;
    let mut app = app_in(temp.path())?;
    app.get_active_panel()
        .mark_matching(&Mask::parse("*"), true);
    app.get_active_panel().invert_marks();
    app.get_active_panel().navigate_to(inner.clone())?;

    app.get_active_panel().restore_marks();

    assert!(marked(&mut app).is_empty(), "got {:?}", marked(&mut app));
    assert!(app.get_active_panel().select_named("parent"));
    assert_eq!(offered_by_f8(&mut app)?, vec![inner.join("parent")]);
    Ok(())
}

/// A name the new folder happens to share is still not the entry that was
/// selected: restoring it would put a different file under F8.
#[test]
fn a_selection_is_not_restored_onto_a_namesake_in_another_folder() -> io::Result<()> {
    let temp = TempDir::new()?;
    let (first, second) = (temp.path().join("first"), temp.path().join("second"));
    for dir in [&first, &second] {
        fs::create_dir(dir)?;
        write_files(dir, &["same.txt"])?;
    }
    let mut app = app_in(&first)?;
    app.get_active_panel()
        .mark_matching(&Mask::parse("same.txt"), true);
    app.get_active_panel().clear_marks();
    app.get_active_panel().navigate_to(second)?;

    app.get_active_panel().restore_marks();

    assert!(marked(&mut app).is_empty(), "got {:?}", marked(&mut app));
    Ok(())
}

/// The same-folder variant: F8 deletes the selection, and Ctrl+M must not
/// resurrect it as marks on files that are gone.
#[test]
fn deleted_entries_are_not_restored_and_f8_does_not_offer_them_again() -> io::Result<()> {
    let _lock = crate::utils::refresh_panels_mutex()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let temp = TempDir::new()?;
    write_files(temp.path(), &["a.txt", "b.txt", "c.txt"])?;
    let mut app = app_in(temp.path())?;
    app.get_active_panel()
        .mark_matching(&Mask::parse("a.txt,b.txt"), true);
    crate::ops::handle_delete_confirm(&mut app)?;
    crate::ops::handle_delete(&mut app)?;

    app.get_active_panel().restore_marks();

    assert!(marked(&mut app).is_empty(), "got {:?}", marked(&mut app));
    assert!(app.get_active_panel().select_named("c.txt"));
    assert_eq!(offered_by_f8(&mut app)?, vec![temp.path().join("c.txt")]);
    Ok(())
}

/// Restoring still works in the folder it was made in: whatever of the old
/// selection is still listed comes back.
#[test]
fn restoring_after_a_reload_brings_back_the_names_still_listed() -> io::Result<()> {
    let temp = TempDir::new()?;
    write_files(temp.path(), &["a.txt", "b.txt", "c.txt"])?;
    let mut app = app_in(temp.path())?;
    app.get_active_panel()
        .mark_matching(&Mask::parse("a.txt,b.txt"), true);
    app.get_active_panel().clear_marks();
    fs::remove_file(temp.path().join("a.txt"))?;
    app.get_active_panel().load_entries()?;

    app.get_active_panel().restore_marks();

    assert_eq!(marked(&mut app), ["b.txt"]);
    Ok(())
}
