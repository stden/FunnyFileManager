//! The exit save fsyncs the session file it changes; a checkpoint writes the
//! same file without the fsync.

use std::{
    cell::Cell,
    fs, io,
    path::{Path, PathBuf},
};

use tempfile::TempDir;

use super::{checkpoint, flush, load};
use crate::{file_write::SYNC_ALL_CALLS, history::Persistence, models::App};

fn directory(parent: &Path, name: &str) -> io::Result<PathBuf> {
    let path = parent.join(name);
    fs::create_dir(&path)?;
    Ok(path)
}

/// An app whose panels show two new folders while its session file does not
/// hold them yet, so the pair differs from what is saved.
fn app_with_changed_panels(temp: &TempDir) -> io::Result<(PathBuf, App)> {
    let file = temp.path().join("config/panel-session.json");
    let mut app = load(temp.path(), Persistence::File(file.clone()))?;
    app.left_panel
        .navigate_to(directory(temp.path(), "left")?)?;
    app.right_panel
        .navigate_to(directory(temp.path(), "right")?)?;
    Ok((file, app))
}

/// The left folder the session file names, read back the way a restart reads it.
fn saved_left(file: PathBuf, launch: &Path) -> io::Result<PathBuf> {
    Ok(load(launch, Persistence::File(file))?.left_panel.path)
}

#[test]
fn flush_writes_changed_panels_and_syncs_the_file() -> io::Result<()> {
    let temp = TempDir::new()?;
    let (file, mut app) = app_with_changed_panels(&temp)?;

    let before = SYNC_ALL_CALLS.with(Cell::get);
    flush(&mut app);

    assert_eq!(SYNC_ALL_CALLS.with(Cell::get) - before, 1);
    assert_eq!(saved_left(file, temp.path())?, app.left_panel.path);
    Ok(())
}

#[test]
fn checkpoint_of_the_same_changed_panels_writes_without_syncing() -> io::Result<()> {
    let temp = TempDir::new()?;
    let (file, mut app) = app_with_changed_panels(&temp)?;

    let before = SYNC_ALL_CALLS.with(Cell::get);
    checkpoint(&mut app);

    assert_eq!(SYNC_ALL_CALLS.with(Cell::get) - before, 0);
    assert_eq!(saved_left(file, temp.path())?, app.left_panel.path);
    Ok(())
}
