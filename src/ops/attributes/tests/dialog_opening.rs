//! Purpose: Opening the dialog on the cursor entry, the marked entries, an unreadable target and `..`.

use super::*;

/// Opening reads the entry under the cursor: its path, bits, owner and time.
#[cfg(unix)]
#[test]
fn open_reads_the_entry_under_the_cursor() {
    let temp = TempDir::new().expect("temp");
    let file = temp.path().join("a.txt");
    fs::write(&file, "x").expect("write");
    set_mode(&file, 0o640);
    let mut app = app_at(temp.path());
    app.left_panel.select_named("a.txt");

    open(&mut app);

    let dialog = dialog(&app);
    assert_eq!(dialog.targets, vec![file]);
    let expected_owner = app
        .left_panel
        .entries
        .iter()
        .find(|e| e.name == "a.txt")
        .and_then(|e| e.owner.clone());
    assert_eq!(dialog.owner, expected_owner.unwrap_or_default());
    assert_ne!(dialog.group.len(), 0, "the group resolves on Unix");
    assert_ne!(dialog.modified.current.len(), 0, "its own time");
    let group_write = dialog
        .flags
        .iter()
        .find(|it| it.flag == Flag::GroupWrite)
        .expect("flag");
    assert_eq!(group_write.current, crate::models::Check::Clear);
    let owner_write = dialog
        .flags
        .iter()
        .find(|it| it.flag == Flag::OwnerWrite)
        .expect("flag");
    assert_eq!(owner_write.current, crate::models::Check::Set);
}

/// Marks win over the cursor, and a directory among them is noticed.
#[test]
fn open_takes_every_marked_entry() {
    let temp = TempDir::new().expect("temp");
    fs::write(temp.path().join("a.txt"), "x").expect("write");
    fs::write(temp.path().join("b.txt"), "x").expect("write");
    fs::create_dir(temp.path().join("sub")).expect("mkdir");
    let mut app = app_at(temp.path());
    app.left_panel
        .marked
        .insert(crate::models::FileName::from("a.txt"));
    app.left_panel
        .marked
        .insert(crate::models::FileName::from("sub"));

    open(&mut app);

    let dialog = dialog(&app);
    assert_eq!(dialog.targets.len(), 2);
    assert!(dialog.has_directories);
    assert_eq!(dialog.headline(), "2 objects");
}

/// A marked name that has gone still opens — with nothing set — rather than
/// blocking the dialog for the entries that are there.
#[test]
fn open_keeps_an_unreadable_target_with_nothing_set() {
    let temp = TempDir::new().expect("temp");
    fs::write(temp.path().join("a.txt"), "x").expect("write");
    let mut app = app_at(temp.path());
    app.left_panel
        .marked
        .insert(crate::models::FileName::from("ghost"));

    open(&mut app);

    let dialog = dialog(&app);
    assert_eq!(dialog.targets, vec![temp.path().join("ghost")]);
    assert!(!dialog.has_directories);
    assert!(dialog
        .flags
        .iter()
        .all(|it| it.current == crate::models::Check::Clear));
    assert!(dialog.owner.is_empty() && dialog.modified.current.is_empty());
}

/// On `..` the dialog is for the folder itself, as Far's is.
#[test]
fn open_on_the_parent_row_edits_the_folder_itself() {
    let temp = TempDir::new().expect("temp");
    let mut app = app_at(temp.path());
    app.left_panel.select(Some(0));

    open(&mut app);

    let dialog = dialog(&app);
    assert_eq!(dialog.targets, vec![temp.path().to_path_buf()]);
    assert!(dialog.has_directories);
    if cfg!(unix) {
        assert_ne!(dialog.owner.len(), 0, "the folder's owner is looked up");
    }
}
