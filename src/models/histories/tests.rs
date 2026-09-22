//! Unit tests: touch order, the cap, and the round trip through the data dir.

use std::path::Path;

use tempfile::TempDir;

use super::*;
use crate::test_support::EnvVarGuard;

#[test]
fn touch_moves_a_known_path_to_the_end() {
    let mut history = FolderHistory::default();
    history.touch(Path::new("/a"));
    history.touch(Path::new("/b"));
    history.touch(Path::new("/a"));
    let order: Vec<&Path> = history.entries().collect();
    assert_eq!(order, [Path::new("/b"), Path::new("/a")]);
    assert_eq!(history.newest_first().next(), Some(Path::new("/a")));
    assert_eq!(history.last(), Some(Path::new("/a")));
    assert_eq!(history.len(), 2);
    assert!(!history.is_empty());
}

#[test]
fn the_oldest_falls_off_past_the_cap() {
    let mut history = ViewHistory::default();
    for index in 0..=PATH_HISTORY_CAP {
        history.touch(&Path::new("/f").join(index.to_string()));
    }
    assert_eq!(history.len(), PATH_HISTORY_CAP);
    assert_eq!(history.entries().next(), Some(Path::new("/f/1")));
}

#[test]
fn a_path_with_a_line_break_is_refused() {
    let mut history = ViewHistory::default();
    history.touch(Path::new("/two\nlines"));
    assert!(history.is_empty());
}

#[test]
fn a_session_only_history_never_writes() -> io::Result<()> {
    let temp = TempDir::new()?;
    let _xdg = EnvVarGuard::set("XDG_DATA_HOME", temp.path());
    let mut history = FolderHistory::default();
    history.touch(Path::new("/somewhere"));
    history.save()?;
    assert!(!temp.path().join("ffm").exists());
    Ok(())
}

#[test]
fn load_save_load_round_trips_in_the_data_dir() -> io::Result<()> {
    let temp = TempDir::new()?;
    let _xdg = EnvVarGuard::set("XDG_DATA_HOME", temp.path());
    let mut view = ViewHistory::load();
    assert!(view.is_empty(), "nothing persisted yet");
    view.touch(Path::new("/etc/hosts"));
    view.touch(Path::new("/etc/passwd"));
    view.save()?;
    let text = fs::read_to_string(temp.path().join("ffm/view_history"))?;
    assert_eq!(text, "/etc/hosts\n/etc/passwd\n");
    let reloaded = ViewHistory::load();
    assert_eq!(reloaded.last(), Some(Path::new("/etc/passwd")));
    assert!(
        FolderHistory::load().is_empty(),
        "the two histories live in separate files"
    );
    Ok(())
}

#[test]
fn the_folder_history_collects_each_panel_visit_once() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::create_dir(temp.path().join("sub"))?;
    let mut app = crate::models::App::with_panels(
        crate::models::Panel::new(temp.path().to_path_buf())?,
        crate::models::Panel::new(temp.path().to_path_buf())?,
    );
    assert!(app.folder_history.is_empty(), "nothing until the loop asks");
    app.record_folder_visits();
    assert_eq!(
        app.folder_history.len(),
        1,
        "both panels start in one place"
    );
    app.record_folder_visits();
    assert_eq!(app.folder_history.len(), 1, "a visit is collected once");
    app.left_panel.navigate_to(temp.path().join("sub"))?;
    app.record_folder_visits();
    assert_eq!(
        app.folder_history.last(),
        Some(temp.path().join("sub").as_path())
    );
    assert_eq!(app.folder_history.len(), 2);
    Ok(())
}
