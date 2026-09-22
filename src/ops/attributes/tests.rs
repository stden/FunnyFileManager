//! Unit tests: Ctrl+A reads the selection into the dialog, and Set writes the
//! dialog back — to every marked entry, into subfolders when asked, and with
//! an honest count when something refuses.

use std::fs;
use std::path::Path;

use tempfile::TempDir;

use super::{apply, open};
use crate::models::{App, AttributesDialog, Overlay, Panel};
#[cfg(unix)]
use crate::models::{AttributesField, Flag};

fn app_at(dir: &Path) -> App {
    let mut app = App::new().expect("test app");
    app.left_panel = Panel::new(dir.to_path_buf()).expect("panel");
    app
}

fn dialog(app: &App) -> &AttributesDialog {
    let Overlay::Attributes(dialog) = &app.overlay else {
        panic!("the attributes dialog is open");
    };
    dialog
}

fn status_text(app: &App) -> String {
    app.status_text()
}

/// Space until the box reads `[x]` — one press for a file, two for a
/// three-state box that opened on `[ ]`.
#[cfg(unix)]
fn set_flag(dialog: &mut AttributesDialog, flag: Flag) {
    let index = dialog
        .flags
        .iter()
        .position(|it| it.flag == flag)
        .expect("platform flag");
    dialog.field = AttributesField::Flag(index);
    while dialog.flags[index].current != crate::models::Check::Set {
        dialog.toggle();
    }
}

#[cfg(unix)]
fn mode(path: &Path) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path).expect("metadata").permissions().mode() & 0o7777
}

#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).expect("chmod");
}

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
    assert!(!dialog.group.is_empty(), "the group resolves on Unix");
    assert!(!dialog.modified.is_empty(), "one file offers its own time");
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
    assert!(dialog.owner.is_empty() && dialog.modified.is_empty());
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
        assert!(!dialog.owner.is_empty(), "the folder's owner is looked up");
    }
}

/// Set writes the moved boxes to every target and keeps the other bits.
#[cfg(unix)]
#[test]
fn apply_changes_only_the_moved_bits_on_every_target() {
    let temp = TempDir::new().expect("temp");
    let a = temp.path().join("a");
    let b = temp.path().join("b");
    fs::write(&a, "x").expect("write");
    fs::write(&b, "x").expect("write");
    set_mode(&a, 0o644);
    set_mode(&b, 0o600);
    let mut app = app_at(temp.path());
    app.left_panel
        .marked
        .insert(crate::models::FileName::from("a"));
    app.left_panel
        .marked
        .insert(crate::models::FileName::from("b"));
    open(&mut app);
    let mut dialog = dialog(&app).clone();
    set_flag(&mut dialog, Flag::OwnerExecute);
    app.close_overlay();

    apply(&mut app, &dialog);

    assert_eq!(mode(&a), 0o744);
    assert_eq!(mode(&b), 0o700);
    assert!(status_text(&app).contains('2'), "{}", status_text(&app));
    assert!(
        app.left_panel.marked.is_empty(),
        "the operation consumed the marks"
    );
}

/// Process subfolders reaches what is inside; without it the folder alone
/// changes.
#[cfg(unix)]
#[test]
fn process_subfolders_recurses_and_its_absence_does_not() {
    let temp = TempDir::new().expect("temp");
    let dir = temp.path().join("d");
    let inner = dir.join("inner.txt");
    let deeper = dir.join("deep").join("x.txt");
    fs::create_dir_all(deeper.parent().expect("parent")).expect("mkdir");
    fs::write(&inner, "x").expect("write");
    fs::write(&deeper, "x").expect("write");
    set_mode(&inner, 0o644);
    set_mode(&deeper, 0o644);
    let mut app = app_at(temp.path());
    app.left_panel.select_named("d");
    open(&mut app);
    let mut dialog = dialog(&app).clone();
    set_flag(&mut dialog, Flag::OtherWrite);
    app.close_overlay();

    apply(&mut app, &dialog);
    assert_eq!(mode(&dir) & 0o002, 0o002);
    assert_eq!(mode(&inner), 0o644, "not asked to recurse");

    dialog.process_subfolders = true;
    apply(&mut app, &dialog);
    assert_eq!(mode(&inner), 0o646);
    assert_eq!(mode(&deeper), 0o646);
}

/// A filled time field sets that time, to the second.
#[test]
fn apply_sets_the_times_that_were_typed() {
    let temp = TempDir::new().expect("temp");
    let file = temp.path().join("t.txt");
    fs::write(&file, "x").expect("write");
    let mut app = app_at(temp.path());
    app.left_panel.select_named("t.txt");
    open(&mut app);
    let mut dialog = dialog(&app).clone();
    dialog.modified = "2001-02-03 04:05:06".to_owned();
    dialog.accessed = "2002-03-04 05:06:07".to_owned();
    app.close_overlay();

    apply(&mut app, &dialog);

    let metadata = fs::metadata(&file).expect("metadata");
    assert_eq!(
        crate::models::format_time(metadata.modified().expect("mtime")),
        "2001-02-03 04:05:06"
    );
    assert_eq!(
        crate::models::format_time(metadata.accessed().expect("atime")),
        "2002-03-04 05:06:07"
    );
}

#[test]
fn timestamps_can_be_set_on_a_directory() {
    let temp = TempDir::new().expect("temp");
    assert_timestamps(temp.path());
}

#[test]
fn timestamps_can_be_set_without_changing_read_only_file_contents() {
    let temp = TempDir::new().expect("temp");
    let path = temp.path().join("readonly.txt");
    fs::write(&path, "unchanged").unwrap();
    let original = fs::metadata(&path).unwrap().permissions();
    let mut readonly = original.clone();
    readonly.set_readonly(true);
    fs::set_permissions(&path, readonly).unwrap();
    let result = std::panic::catch_unwind(|| assert_timestamps(&path));
    fs::set_permissions(&path, original).unwrap();
    result.unwrap();
    assert_eq!(fs::read_to_string(path).unwrap(), "unchanged");
}

fn assert_timestamps(path: &Path) {
    let time = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000_000);
    let times = crate::models::RequestedTimes {
        modified: Some(time),
        accessed: Some(time),
    };
    super::stamp(path, times).unwrap();
    let metadata = fs::metadata(path).unwrap();
    assert_eq!(metadata.modified().unwrap(), time);
    assert_eq!(metadata.accessed().unwrap(), time);
}

/// A time that does not parse changes nothing and is named in the status.
#[cfg(unix)]
#[test]
fn a_bad_time_is_refused_before_anything_changes() {
    let temp = TempDir::new().expect("temp");
    let file = temp.path().join("t.txt");
    fs::write(&file, "x").expect("write");
    set_mode(&file, 0o644);
    let mut app = app_at(temp.path());
    app.left_panel.select_named("t.txt");
    open(&mut app);
    let mut dialog = dialog(&app).clone();
    set_flag(&mut dialog, Flag::OtherWrite);
    dialog.modified = "soon".to_owned();
    app.close_overlay();

    apply(&mut app, &dialog);

    assert_eq!(mode(&file), 0o644, "nothing was written");
    assert!(status_text(&app).contains("soon"), "{}", status_text(&app));
}

/// A target that has gone is counted as a failure, not hidden.
#[test]
fn a_missing_target_is_counted_and_named() {
    let temp = TempDir::new().expect("temp");
    let file = temp.path().join("gone.txt");
    fs::write(&file, "x").expect("write");
    let mut app = app_at(temp.path());
    app.left_panel.select_named("gone.txt");
    open(&mut app);
    let mut dialog = dialog(&app).clone();
    dialog.modified = "2001-02-03 04:05:06".to_owned();
    app.close_overlay();
    fs::remove_file(&file).expect("remove");

    apply(&mut app, &dialog);

    let text = status_text(&app);
    assert!(text.contains("1 failed"), "{text}");
    assert!(text.contains("gone.txt"), "{text}");
}
