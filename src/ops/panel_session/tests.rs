//! Panel directories survive restart independently of the launch directory.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use tempfile::TempDir;

use super::{checkpoint, load};
use crate::history::Persistence;

fn directory(parent: &Path, name: &str) -> io::Result<PathBuf> {
    let path = parent.join(name);
    fs::create_dir(&path)?;
    Ok(path)
}

fn saved_session(parent: &Path, left: &Path, right: &Path) -> io::Result<PathBuf> {
    let path = parent.join("panel-session.json");
    let text = serde_json::json!({ "left": left, "right": right });
    fs::write(&path, serde_json::to_vec(&text)?)?;
    Ok(path)
}

#[test]
fn restores_both_panels_from_a_different_launch_directory() -> io::Result<()> {
    let temp = TempDir::new()?;
    let left = directory(temp.path(), "Левая папка")?;
    let right = directory(temp.path(), "Правая папка")?;
    let launch = directory(temp.path(), "launch")?;
    fs::write(left.join("left.txt"), "left")?;
    fs::write(right.join("right.txt"), "right")?;
    let file = saved_session(temp.path(), &left, &right)?;

    let app = load(&launch, Persistence::File(file))?;

    assert_eq!(app.left_panel.path, left);
    assert_eq!(app.right_panel.path, right);
    assert!(app
        .left_panel
        .entries
        .iter()
        .any(|entry| entry.name == "left.txt"));
    assert!(app
        .right_panel
        .entries
        .iter()
        .any(|entry| entry.name == "right.txt"));
    Ok(())
}

#[test]
fn unavailable_directory_falls_back_without_losing_the_other_panel() -> io::Result<()> {
    let temp = TempDir::new()?;
    let valid = directory(temp.path(), "valid")?;
    let launch = directory(temp.path(), "launch")?;
    let missing = temp.path().join("removed");
    let not_directory = temp.path().join("file.txt");
    fs::write(&not_directory, "file")?;
    for unavailable in [missing, not_directory, PathBuf::from("relative/path")] {
        let file = saved_session(temp.path(), &unavailable, &valid)?;
        let app = load(&launch, Persistence::File(file))?;
        assert_eq!(app.left_panel.path, launch);
        assert_eq!(app.right_panel.path, valid);
        let file = saved_session(temp.path(), &valid, &unavailable)?;
        let app = load(&launch, Persistence::File(file))?;
        assert_eq!(app.left_panel.path, valid);
        assert_eq!(app.right_panel.path, launch);
    }
    Ok(())
}

#[test]
fn missing_or_corrupt_session_starts_in_the_launch_directory() -> io::Result<()> {
    let temp = TempDir::new()?;
    let file = temp.path().join("panel-session.json");
    for text in [None, Some(""), Some("{invalid"), Some("{\"left\":42}")] {
        if let Some(text) = text {
            fs::write(&file, text)?;
        }
        let app = load(temp.path(), Persistence::File(file.clone()))?;
        assert_eq!(app.left_panel.path, temp.path());
        assert_eq!(app.right_panel.path, temp.path());
    }
    Ok(())
}

#[test]
fn valid_session_does_not_need_a_readable_launch_directory() -> io::Result<()> {
    let temp = TempDir::new()?;
    let file = saved_session(temp.path(), temp.path(), temp.path())?;
    let app = load(&temp.path().join("missing"), Persistence::File(file))?;
    assert_eq!(app.left_panel.path, temp.path());
    assert_eq!(app.right_panel.path, temp.path());
    assert!(load(&temp.path().join("missing"), Persistence::SessionOnly).is_err());
    Ok(())
}

#[test]
fn navigation_and_panel_swap_are_saved_without_auto_save_setup() -> io::Result<()> {
    let temp = TempDir::new()?;
    let left = directory(temp.path(), "left")?;
    let right = directory(temp.path(), "right")?;
    let file = temp.path().join("config/panel-session.json");
    let mut app = load(temp.path(), Persistence::File(file.clone()))?;
    assert!(!app.config.system.auto_save_setup);
    app.left_panel.navigate_to(left.clone())?;
    app.right_panel.navigate_to(right.clone())?;
    checkpoint(&mut app);
    std::mem::swap(&mut app.left_panel, &mut app.right_panel);
    checkpoint(&mut app);

    let reopened = load(temp.path(), Persistence::File(file))?;
    assert_eq!(reopened.left_panel.path, right);
    assert_eq!(reopened.right_panel.path, left);
    Ok(())
}

#[test]
fn session_only_and_write_failure_do_not_prevent_navigation() -> io::Result<()> {
    let temp = TempDir::new()?;
    let child = directory(temp.path(), "child")?;
    let blocked = temp.path().join("blocked");
    fs::write(&blocked, "file")?;
    for persistence in [
        Persistence::SessionOnly,
        Persistence::File(blocked.join("session")),
    ] {
        let mut app = load(temp.path(), persistence)?;
        app.left_panel.navigate_to(child.clone())?;
        checkpoint(&mut app);
        assert_eq!(app.left_panel.path, child);
        assert!(app.lifecycle.is_running());
    }
    assert_eq!(fs::read_to_string(blocked)?, "file");
    Ok(())
}
