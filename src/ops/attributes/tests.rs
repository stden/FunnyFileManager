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
    assert!(!dialog.modified.current.is_empty(), "its own time");
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

#[cfg(unix)]
fn set_mtime(path: &Path, text: &str) {
    let time = chrono::NaiveDateTime::parse_from_str(text, "%Y-%m-%d %H:%M:%S")
        .expect("fixture time")
        .and_local_timezone(chrono::Local)
        .single()
        .expect("unambiguous fixture time");
    let file = fs::File::open(path).expect("open fixture");
    file.set_modified(std::time::SystemTime::from(time))
        .expect("set fixture mtime");
}

#[cfg(unix)]
fn mtime(path: &Path) -> String {
    crate::models::format_time(
        fs::metadata(path)
            .expect("metadata")
            .modified()
            .expect("mtime"),
    )
}

/// Changing one bit with Process subfolders on changes that bit inside and
/// nothing else: the folder's own times, which the dialog offers back
/// untouched, are not stamped on its contents.
#[cfg(unix)]
#[test]
fn process_subfolders_leaves_the_times_inside_alone_when_they_were_not_edited() {
    let temp = TempDir::new().expect("temp");
    let dir = temp.path().join("attrtest");
    let file = dir.join("a.txt");
    let deeper = dir.join("sub").join("b.txt");
    fs::create_dir_all(deeper.parent().expect("parent")).expect("mkdir");
    fs::write(&file, "a").expect("write");
    fs::write(&deeper, "b").expect("write");
    set_mtime(&file, "2001-01-01 01:01:01");
    set_mtime(&deeper, "2002-02-02 02:02:02");
    set_mtime(&dir, "2010-10-10 10:10:10");
    let mut app = app_at(temp.path());
    app.left_panel.select_named("attrtest");
    open(&mut app);
    let mut dialog = dialog(&app).clone();
    set_flag(&mut dialog, Flag::OtherWrite);
    dialog.process_subfolders = true;
    app.close_overlay();

    apply(&mut app, &dialog);

    assert_eq!(mode(&file) & 0o002, 0o002, "the bit reached inside");
    assert_eq!(mtime(&file), "2001-01-01 01:01:01");
    assert_eq!(mtime(&deeper), "2002-02-02 02:02:02");
}

/// Opens the dialog on `name`, asks for Other Write with Process subfolders
/// on, and presses Set.
#[cfg(unix)]
fn set_other_write_recursively(app: &mut App, name: &str) {
    app.left_panel.select_named(name);
    open(app);
    let mut dialog = dialog(app).clone();
    set_flag(&mut dialog, Flag::OtherWrite);
    dialog.process_subfolders = true;
    app.close_overlay();
    apply(app, &dialog);
}

/// Process subfolders walks real folders only: a link inside the selection
/// is neither followed nor changed, the rule `chmod -R` keeps.
#[cfg(unix)]
#[test]
fn process_subfolders_never_follows_a_link_out_of_the_selection() {
    let temp = TempDir::new().expect("temp");
    let dir = temp.path().join("attrtest");
    let outside = temp.path().join("attr-outside");
    fs::create_dir_all(&dir).expect("mkdir");
    fs::create_dir_all(&outside).expect("mkdir");
    fs::write(dir.join("a.txt"), "a").expect("write");
    fs::write(outside.join("c.txt"), "c").expect("write");
    set_mode(&outside, 0o755);
    set_mode(&outside.join("c.txt"), 0o644);
    std::os::unix::fs::symlink("../attr-outside", dir.join("link-out")).expect("symlink");
    let mut app = app_at(temp.path());

    set_other_write_recursively(&mut app, "attrtest");

    assert_eq!(mode(&dir.join("a.txt")) & 0o002, 0o002, "inside changed");
    assert_eq!(mode(&outside), 0o755, "the link's target is outside");
    assert_eq!(mode(&outside.join("c.txt")), 0o644, "and so is its content");
    assert_eq!(status_text(&app), "Attributes set on 2 object(s)");
}

/// A link back to its own folder is not a way down: the walk ends instead
/// of recursing until the kernel refuses the fortieth link.
#[cfg(unix)]
#[test]
fn process_subfolders_does_not_loop_through_a_link_to_its_own_folder() {
    let temp = TempDir::new().expect("temp");
    let sub = temp.path().join("attrtest2").join("sub");
    fs::create_dir_all(&sub).expect("mkdir");
    fs::write(sub.join("x.txt"), "x").expect("write");
    std::os::unix::fs::symlink(".", sub.join("self")).expect("symlink");
    let mut app = app_at(temp.path());

    set_other_write_recursively(&mut app, "attrtest2");

    assert_eq!(status_text(&app), "Attributes set on 3 object(s)");
}

/// The owner of a mode-0000 file can give themselves read back and set a
/// time in the same Set. Checked for real only when the suite runs as a
/// plain user: root opens any file, so the old failure never showed there.
#[cfg(unix)]
#[test]
fn the_owner_restores_read_and_a_time_on_a_mode_0000_file() {
    let temp = TempDir::new().expect("temp");
    let file = temp.path().join("zero.txt");
    fs::write(&file, "z").expect("write");
    set_mode(&file, 0o000);
    let mut app = app_at(temp.path());
    app.left_panel.select_named("zero.txt");
    open(&mut app);
    let mut dialog = dialog(&app).clone();
    set_flag(&mut dialog, Flag::OwnerRead);
    dialog.modified.current = "2001-02-03 04:05:06".to_owned();
    app.close_overlay();

    apply(&mut app, &dialog);

    assert_eq!(status_text(&app), "Attributes set on 1 object(s)");
    assert_eq!(mode(&file), 0o400);
    assert_eq!(mtime(&file), "2001-02-03 04:05:06");
}

/// The other direction: taking every permission away in the same Set as a
/// time edit still writes the time.
#[cfg(unix)]
#[test]
fn a_time_is_set_in_the_same_set_that_drops_every_permission() {
    let temp = TempDir::new().expect("temp");
    let file = temp.path().join("t.txt");
    fs::write(&file, "x").expect("write");
    set_mode(&file, 0o400);
    let mut app = app_at(temp.path());
    app.left_panel.select_named("t.txt");
    open(&mut app);
    let mut dialog = dialog(&app).clone();
    dialog.field = AttributesField::Flag(0);
    dialog.toggle();
    dialog.modified.current = "2001-02-03 04:05:06".to_owned();
    app.close_overlay();

    apply(&mut app, &dialog);

    assert_eq!(status_text(&app), "Attributes set on 1 object(s)");
    assert_eq!(mode(&file), 0o000);
    assert_eq!(mtime(&file), "2001-02-03 04:05:06");
}

/// A time is written by path: a pipe has no reader to wait for, so setting
/// its time must not block the UI the way opening it does.
#[cfg(unix)]
#[test]
fn a_time_is_set_on_a_pipe_without_opening_it() {
    let temp = TempDir::new().expect("temp");
    let fifo = temp.path().join("pipe");
    crate::test_support::make_fifo(&fifo).expect("mkfifo");
    let time = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000_000);
    let times = crate::models::RequestedTimes {
        modified: Some(time),
        accessed: None,
    };

    let target = fifo.clone();
    let result =
        crate::test_support::run_with_timeout(std::time::Duration::from_secs(5), move || {
            super::stamp(&target, times).map_err(|error| error.to_string())
        });

    result
        .expect("setting the time hung on the pipe")
        .expect("stamp");
    let metadata = fs::symlink_metadata(&fifo).expect("metadata");
    assert_eq!(metadata.modified().expect("mtime"), time);
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
    dialog.modified.current = "2001-02-03 04:05:06".to_owned();
    dialog.accessed.current = "2002-03-04 05:06:07".to_owned();
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
    dialog.modified.current = "soon".to_owned();
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
    dialog.modified.current = "2001-02-03 04:05:06".to_owned();
    app.close_overlay();
    fs::remove_file(&file).expect("remove");

    apply(&mut app, &dialog);

    let text = status_text(&app);
    assert!(text.contains("1 failed"), "{text}");
    assert!(text.contains("gone.txt"), "{text}");
}
