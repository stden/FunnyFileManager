//! Purpose: Setting times: on a mode 0000 file, a pipe, a folder and a read-only file.

use super::*;

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

    assert_eq!(app.status_text(), "Attributes set on 1 object(s)");
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

    assert_eq!(app.status_text(), "Attributes set on 1 object(s)");
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
            stamp(&target, times).map_err(|error| error.to_string())
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
    stamp(path, times).unwrap();
    let metadata = fs::metadata(path).unwrap();
    assert_eq!(metadata.modified().unwrap(), time);
    assert_eq!(metadata.accessed().unwrap(), time);
}
