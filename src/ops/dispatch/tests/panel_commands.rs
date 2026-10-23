//! Purpose: Built-ins that act on the active panel: cd, pwd, sort, bookmarks, mark and unmark.

use super::*;

// ─── cd ───────────────────────────────────────────────────────────────

#[test]
fn test_cd_absolute_path() {
    let temp = TempDir::new().expect("tempdir");
    let subdir = temp.path().join("sub");
    fs::create_dir(&subdir).expect("mkdir");

    let mut app = make_app(&temp);
    execute_command(&mut app, &format!("cd {}", subdir.display())).expect("cd");
    assert_eq!(app.get_active_panel().path.file_name().unwrap(), "sub");
    assert!(app.status_text().contains("Opened"));
}

#[test]
fn test_cd_relative_path() {
    let temp = TempDir::new().expect("tempdir");
    let subdir = temp.path().join("child");
    fs::create_dir(&subdir).expect("mkdir");

    let mut app = make_app(&temp);
    execute_command(&mut app, "cd child").expect("cd");
    assert_eq!(app.get_active_panel().path.file_name().unwrap(), "child");
}

#[test]
fn test_cd_nonexistent_errors() {
    let temp = TempDir::new().expect("tempdir");
    let mut app = make_app(&temp);
    let result = execute_command(&mut app, "cd nonexistent_xyz");
    assert!(result.is_err());
}

#[test]
fn test_cd_to_file_errors() {
    let temp = TempDir::new().expect("tempdir");
    File::create(temp.path().join("afile.txt")).expect("create");
    let mut app = make_app(&temp);
    let result = execute_command(
        &mut app,
        &format!("cd {}", temp.path().join("afile.txt").display()),
    );
    assert!(result.is_err());
}

// ─── pwd ──────────────────────────────────────────────────────────────

#[test]
fn test_pwd_sets_status() {
    let temp = TempDir::new().expect("tempdir");
    let mut app = make_app(&temp);
    execute_command(&mut app, "pwd").expect("pwd");
    assert_ne!(app.status_text().len(), 0);
}

// ─── sort ─────────────────────────────────────────────────────────────

#[test]
fn test_sort_name() {
    let temp = TempDir::new().expect("tempdir");
    let mut app = make_app(&temp);
    execute_command(&mut app, "sort name").expect("sort");
    assert_eq!(app.get_active_panel().sort_mode, SortMode::Name);
    assert!(app.status_text().contains("Sort"));
}

#[test]
fn test_sort_extension() {
    let temp = TempDir::new().expect("tempdir");
    let mut app = make_app(&temp);
    execute_command(&mut app, "sort ext").expect("sort");
    assert_eq!(app.get_active_panel().sort_mode, SortMode::Extension);
}

#[test]
fn test_sort_size() {
    let temp = TempDir::new().expect("tempdir");
    let mut app = make_app(&temp);
    execute_command(&mut app, "sort size").expect("sort");
    assert_eq!(app.get_active_panel().sort_mode, SortMode::Size);
}

// ─── bookmarks ───────────────────────────────────────────────────────

#[test]
fn test_bookmark_set_and_go() {
    let temp = TempDir::new().expect("tempdir");
    let mut app = make_app(&temp);
    execute_command(&mut app, "bookmark set 0").expect("set");
    assert!(app
        .bookmarks
        .get(BookmarkSlot::from_usize(0).expect("slot"))
        .is_some());
    assert!(app.status_text().contains("Bookmark 0 set"));

    execute_command(&mut app, "bookmark go 0").expect("go");
    assert!(app.status_text().contains("Opened bookmark 0"));
}

#[test]
fn test_bookmark_go_empty_errors() {
    let temp = TempDir::new().expect("tempdir");
    let mut app = make_app(&temp);
    let result = execute_command(&mut app, "bookmark go 5");
    assert!(result.is_err());
}

// ─── mark / unmark ───────────────────────────────────────────────────

#[test]
fn test_mark_all_and_unmark() {
    let temp = TempDir::new().expect("tempdir");
    File::create(temp.path().join("a.txt")).expect("create");
    File::create(temp.path().join("b.txt")).expect("create");

    let mut app = make_app(&temp);
    app.left_panel.load_entries().expect("load");

    execute_command(&mut app, "mark all").expect("mark");
    assert_ne!(app.get_active_panel().marked.len(), 0);
    assert!(app.status_text().contains("Marked"));

    execute_command(&mut app, "unmark").expect("unmark");
    assert_eq!(app.get_active_panel().marked.len(), 0);
    assert_eq!(app.status_text(), "Unmarked all");
}

/// Setting a bookmark cannot fail for lack of a slot to put it in.
///
/// This used to be a bounds test: it emptied the bookmarks `Vec` and
/// asserted both commands errored. `Bookmarks` has one entry per
/// `BookmarkSlot` by construction, so that state no longer exists — what is
/// left to check is that every legal slot round-trips, and that an *empty*
/// slot is still the one thing `bookmark go` refuses.
#[test]
fn every_legal_slot_round_trips_and_only_an_empty_one_refuses() {
    let temp = TempDir::new().expect("tempdir");
    let mut app = make_app(&temp);

    for n in 0..=BookmarkSlot::MAX {
        let slot = BookmarkSlot::new(n).expect("legal slot");
        assert!(
            cmd_bookmark_go(&mut app, slot).is_err(),
            "slot {n} is empty, so going to it must fail"
        );
        cmd_bookmark_set(&mut app, slot);
        cmd_bookmark_go(&mut app, slot).expect("go after set");
    }
}
