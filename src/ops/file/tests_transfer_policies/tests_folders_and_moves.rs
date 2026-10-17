//! Purpose: Timestamps, folders, moves and destinations that do not exist yet.

use super::*;

// ─── timestamps ─────────────────────────────────────────────────────────────

/// Without the box ticked the copy is new; with it, the copy is as old as the
/// original. That is the whole difference the control makes.
#[test]
fn preserving_timestamps_gives_the_copy_the_source_time() {
    use std::time::{Duration, SystemTime};

    let (mut app, from, to) = app_with(&[("old.txt", b"x")]);
    let long_ago = SystemTime::now() - Duration::from_hours(24);
    fs::File::options()
        .write(true)
        .open(from.path().join("old.txt"))
        .and_then(|file| file.set_modified(long_ago))
        .expect("test setup");
    mark(&mut app, "old.txt");

    let mut dialog = settings(to.path());
    dialog.timestamps = true;
    copy_with(&mut app, &dialog);

    let copied = fs::metadata(to.path().join("old.txt"))
        .unwrap()
        .modified()
        .unwrap();
    let source = fs::metadata(from.path().join("old.txt"))
        .unwrap()
        .modified()
        .unwrap();
    assert_eq!(copied, source);
}

// ─── directories, moves, and destinations that are not there yet ────────────

#[test]
fn a_directory_is_transferred_with_everything_in_it() {
    let (mut app, from, to) = app_with(&[]);
    let tree = from.path().join("project");
    fs::create_dir_all(tree.join("src")).expect("test setup");
    fs::write(tree.join("src/main.rs"), b"fn main() {}").expect("test setup");
    app.get_active_panel().load_entries().expect("reload");
    mark(&mut app, "project");

    copy_with(&mut app, &settings(to.path()));

    assert_eq!(
        fs::read(to.path().join("project/src/main.rs")).unwrap(),
        b"fn main() {}"
    );
}

#[test]
fn a_move_takes_the_original_with_it() {
    let (mut app, from, to) = app_with(&[("notes.txt", b"x")]);
    mark(&mut app, "notes.txt");

    let mut dialog = settings(to.path());
    dialog.purpose = Purpose::Move;
    copy_with(&mut app, &dialog);

    assert!(to.path().join("notes.txt").exists());
    assert!(!from.path().join("notes.txt").exists(), "it moved");
}

/// A relative destination is taken against the active panel, because that is
/// where the user is looking when they type one. The trailing separator asks
/// for a folder (without it, `archive` would be the copy's new name).
#[test]
fn a_relative_destination_is_taken_against_the_active_panel() {
    let (mut app, from, _to) = app_with(&[("notes.txt", b"x")]);
    mark(&mut app, "notes.txt");

    let mut dialog = settings(Path::new("."));
    dialog
        .destination
        .set_text(&format!("archive{}", std::path::MAIN_SEPARATOR));
    copy_with(&mut app, &dialog);

    assert!(
        from.path().join("archive/notes.txt").exists(),
        "and the directory was created on the way"
    );
}

/// A day-to-day archive path often does not exist yet, and names commonly
/// contain both spaces and non-ASCII characters. The relative destination is
/// created below the active panel without changing either the bytes or name.
#[test]
fn a_named_report_copies_into_a_new_nested_relative_archive() {
    let name = "Q3 résumé draft.txt";
    let body: &[u8] = b"review-ready\n";
    let (mut app, from, _to) = app_with(&[(name, body)]);
    mark(&mut app, name);

    let archive = Path::new("archive").join("2026");
    let mut dialog = settings(Path::new("."));
    dialog.destination.set_text(&format!(
        "{}{}",
        archive.display(),
        std::path::MAIN_SEPARATOR
    ));
    copy_with(&mut app, &dialog);

    let copied = from.path().join(&archive).join(name);
    assert_eq!(fs::read(&copied).unwrap(), body);
    assert!(
        from.path().join(name).is_file(),
        "the source remains after F5"
    );
    assert!(
        app.status.to_string().contains(name),
        "the status identifies the copied report, got {}",
        app.status
    );
}

/// A failure on one file must not stop the others, and the status has to name
/// the count as well as the first error.
#[test]
fn one_failure_is_reported_without_losing_the_rest() {
    let (mut app, _from, to) = app_with(&[("good.txt", b"x")]);
    mark(&mut app, "good.txt");
    // A marked name with nothing behind it: the source is gone by the time the
    // transfer runs, which is exactly what a stale panel does.
    mark(&mut app, "vanished.txt");

    copy_with(&mut app, &settings(to.path()));

    assert!(to.path().join("good.txt").exists(), "the good one went");
    let status = app.status.to_string();
    assert!(
        status.contains('1') && status.to_lowercase().contains("error"),
        "the status names the failure, got {status}"
    );
}

/// Marks are dropped after a transfer: leaving them would let the next F8
/// delete files the user believes they already dealt with.
#[test]
fn the_marks_are_cleared_once_the_transfer_is_done() {
    let (mut app, _from, to) = app_with(&[("a.txt", b"x")]);
    mark(&mut app, "a.txt");

    copy_with(&mut app, &settings(to.path()));

    assert!(app.get_active_panel().marked.is_empty());
}

#[test]
fn a_transfer_with_nothing_selected_does_nothing_at_all() {
    let (mut app, _from, to) = app_with(&[("a.txt", b"x")]);
    app.get_active_panel().select(None);

    copy_with(&mut app, &settings(to.path()));

    assert!(!to.path().join("a.txt").exists());
}
