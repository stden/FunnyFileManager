//! Purpose: Opening the confirmation, running it, and cancelling it.

use super::*;

#[test]
fn open_asks_about_the_selected_entry() {
    let dir = TempDir::new().expect("temp dir");
    fs::write(dir.path().join("one.txt"), b"1").expect("write");
    let mut app = app_in(dir.path());
    assert!(app.get_active_panel().select_named("one.txt"));

    open(&mut app);

    let Overlay::WipeConfirm(paths) = &app.overlay else {
        panic!("no wipe dialog");
    };
    assert_eq!(paths, &[dir.path().join("one.txt")]);
}

#[test]
fn open_asks_about_every_marked_entry() {
    let dir = TempDir::new().expect("temp dir");
    fs::write(dir.path().join("one.txt"), b"1").expect("write");
    fs::write(dir.path().join("two.txt"), b"2").expect("write");
    let mut app = app_in(dir.path());
    app.get_active_panel()
        .marked
        .insert(FileName::from("one.txt"));
    app.get_active_panel()
        .marked
        .insert(FileName::from("two.txt"));

    open(&mut app);

    let Overlay::WipeConfirm(paths) = &app.overlay else {
        panic!("no wipe dialog");
    };
    assert_eq!(paths.len(), 2);
}

#[test]
fn open_with_the_cursor_on_dotdot_says_there_is_nothing_to_wipe() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    assert!(app.get_active_panel().select_named(".."));

    open(&mut app);

    assert!(!app.overlay.is_active(), "no dialog about nothing");
    assert_eq!(status_text(&app), "Nothing to wipe");
}

#[test]
fn run_wipes_every_confirmed_entry_clears_marks_and_reports() {
    let dir = TempDir::new().expect("temp dir");
    fs::write(dir.path().join("one.txt"), b"1").expect("write");
    fs::write(dir.path().join("two.txt"), b"2").expect("write");
    fs::write(dir.path().join("keep.txt"), b"k").expect("write");
    let mut app = app_in(dir.path());
    app.get_active_panel()
        .marked
        .insert(FileName::from("one.txt"));
    app.get_active_panel()
        .marked
        .insert(FileName::from("two.txt"));
    open(&mut app);

    run(&mut app);

    assert!(!app.overlay.is_active());
    assert!(!dir.path().join("one.txt").exists());
    assert!(!dir.path().join("two.txt").exists());
    assert!(dir.path().join("keep.txt").exists());
    assert_eq!(app.get_active_panel().marked.len(), 0);
    assert_eq!(status_text(&app), "Wiped 2 item(s)");
    let listed: Vec<&str> = app
        .get_active_panel_ref()
        .entries
        .iter()
        .map(|entry| entry.name.as_str())
        .collect();
    assert!(!listed.contains(&"one.txt"), "panel reloaded: {listed:?}");
}

#[test]
fn run_folds_a_failure_into_the_summary_and_wipes_the_rest() {
    let dir = TempDir::new().expect("temp dir");
    fs::write(dir.path().join("one.txt"), b"1").expect("write");
    let mut app = app_in(dir.path());
    app.set_overlay(Overlay::WipeConfirm(vec![
        dir.path().join("absent"),
        dir.path().join("one.txt"),
    ]));

    run(&mut app);

    assert!(!dir.path().join("one.txt").exists());
    let status = status_text(&app);
    assert!(
        status.starts_with("Wiped 1 item(s), 1 error(s): "),
        "{status}"
    );
    assert!(status.contains("absent"), "{status}");
}

#[test]
fn run_does_nothing_unless_the_wipe_dialog_is_up() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    app.set_overlay(Overlay::DeleteConfirm(vec![dir.path().join("x")]));

    run(&mut app);

    assert!(matches!(app.overlay, Overlay::DeleteConfirm(_)));
    assert!(matches!(app.status, Status::KeyHints));
}

#[test]
fn cancel_closes_the_dialog_and_touches_nothing() {
    let dir = TempDir::new().expect("temp dir");
    fs::write(dir.path().join("one.txt"), b"1").expect("write");
    let mut app = app_in(dir.path());
    app.set_overlay(Overlay::WipeConfirm(vec![dir.path().join("one.txt")]));

    cancel(&mut app);

    assert!(!app.overlay.is_active());
    assert_eq!(fs::read(dir.path().join("one.txt")).expect("read"), b"1");
    assert_eq!(status_text(&app), "Wipe cancelled");
}
