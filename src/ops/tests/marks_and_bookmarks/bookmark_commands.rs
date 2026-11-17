//! Purpose: Typed bookmark and cd commands, and bookmarks that outlive a restart.

use super::*;

#[test]
fn test_execute_command_bookmark_set_slot_9() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    execute_command(&mut app, "bookmark set 9").expect("bookmark set 9");

    assert!(
        app.bookmarks
            .get(BookmarkSlot::from_usize(9).expect("slot"))
            .is_some(),
        "bookmark slot 9 should be set"
    );
    assert_eq!(
        app.bookmarks
            .get(BookmarkSlot::from_usize(9).expect("slot"))
            .expect("set"),
        temp_dir.path()
    );

    Ok(())
}

#[test]
fn test_execute_command_cd_relative() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let subdir = temp_dir.path().join("child");
    fs::create_dir(&subdir).expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    execute_command(&mut app, "cd child").expect("test setup");
    assert!(app.status_text().contains("Opened"));

    Ok(())
}

#[test]
fn test_execute_command_bookmark_go_0_set_then_navigate() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let bookmark_dir = temp_dir.path().join("bookmark_target");
    fs::create_dir(&bookmark_dir).expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.bookmarks.set(
        BookmarkSlot::from_usize(0).expect("slot"),
        bookmark_dir.clone(),
    );

    execute_command(&mut app, "bookmark go 0").expect("bookmark go 0");

    assert_eq!(app.get_active_panel().path, bookmark_dir);
    assert!(app.status_text().contains("Opened bookmark 0"));

    Ok(())
}

/// `bookmark set` writes the slot where the next run's `bookmark go` reads it.
#[test]
fn bookmark_set_survives_a_restart() -> io::Result<()> {
    let temp = TempDir::new()?;
    let data = temp.path().join("data");
    let _xdg = crate::test_support::EnvVarGuard::set("XDG_DATA_HOME", &data);
    let dir = temp.path().to_path_buf();
    let mut app = App::with_panels(Panel::new(dir.clone())?, Panel::new(dir)?);
    app.bookmarks = crate::models::Bookmarks::load();

    execute_command(&mut app, "bookmark set 3")?;

    let restarted = crate::models::Bookmarks::load();
    let slot = BookmarkSlot::new(3).expect("slot");
    assert_eq!(restarted.get(slot), Some(temp.path()));
    Ok(())
}
