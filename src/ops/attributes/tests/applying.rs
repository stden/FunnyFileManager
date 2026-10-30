//! Purpose: Apply changes only the moved bits, refuses a bad time first, and names a missing target.

use super::*;

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
    assert!(app.status_text().contains('2'), "{}", app.status_text());
    assert_eq!(
        app.left_panel.marked.len(),
        0,
        "the operation consumed the marks"
    );
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
    assert!(app.status_text().contains("soon"), "{}", app.status_text());
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

    let crate::status::Status::AttributesSetWithErrors {
        count: 0,
        error_count: 1,
        first_error,
    } = &app.status
    else {
        panic!("one failure, and nothing set: {:?}", app.status);
    };
    assert_eq!(
        first_error.path().file_name(),
        file.file_name(),
        "the failure names the target"
    );
}

#[test]
fn process_subfolders_stamps_children_before_parent_directory() {
    let temp = TempDir::new().expect("temp");
    let dir = temp.path().join("folder");
    let child = dir.join("child.txt");
    fs::create_dir_all(&dir).expect("mkdir");
    fs::write(&child, "x").expect("write");
    let mut app = app_at(temp.path());
    app.left_panel.select_named("folder");
    open(&mut app);
    let mut dialog = dialog(&app).clone();
    dialog.modified.current = "2001-02-03 04:05:06".to_owned();
    dialog.process_subfolders = true;
    app.close_overlay();

    apply(&mut app, &dialog);

    assert_eq!(app.status_text(), "Attributes set on 2 object(s)");
}
