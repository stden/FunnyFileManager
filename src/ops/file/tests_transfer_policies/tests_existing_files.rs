//! Purpose: What the dialog's "already existing files" choice does to a file already there.

use super::*;

// ─── what to do about a file already there ──────────────────────────────────

/// The default. Nothing already on disk is touched and the copy lands beside
/// it under a free name, which is the non-destructive reading of "ask" until
/// there is a per-file prompt to ask with.
#[test]
fn ask_keeps_the_original_and_renames_the_copy() {
    let (mut app, from, to) = app_with(&[("notes.txt", b"new")]);
    fs::write(to.path().join("notes.txt"), b"old").expect("test setup");
    mark(&mut app, "notes.txt");

    let mut dialog = settings(to.path());
    dialog.existing = ExistingFiles::Ask;
    copy_with(&mut app, &dialog);

    assert_eq!(fs::read(to.path().join("notes.txt")).unwrap(), b"old");
    assert_eq!(fs::read(to.path().join("notes_1.txt")).unwrap(), b"new");
    assert!(from.path().join("notes.txt").exists(), "a copy, not a move");
    assert_eq!(
        app.status,
        Status::Transferred {
            purpose: Purpose::Copy,
            landed: Landed::At(to.path().join("notes_1.txt")),
            skipped: 0,
            renamed: 1,
        },
        "the status has to say the copy was renamed"
    );
}

#[test]
fn overwrite_replaces_what_was_there() {
    let (mut app, _from, to) = app_with(&[("notes.txt", b"new")]);
    fs::write(to.path().join("notes.txt"), b"old").expect("test setup");
    mark(&mut app, "notes.txt");

    let mut dialog = settings(to.path());
    dialog.existing = ExistingFiles::Overwrite;
    copy_with(&mut app, &dialog);

    assert_eq!(fs::read(to.path().join("notes.txt")).unwrap(), b"new");
    assert!(!to.path().join("notes_1.txt").exists(), "no second copy");
}

/// Skipping is a normal outcome, not a failure — so it must be counted apart
/// from the copies, or the status line claims work that did not happen.
#[test]
fn skip_leaves_it_alone_and_says_how_many_it_left() {
    let (mut app, _from, to) = app_with(&[("a.txt", b"new"), ("b.txt", b"new")]);
    fs::write(to.path().join("a.txt"), b"old").expect("test setup");
    mark(&mut app, "a.txt");
    mark(&mut app, "b.txt");

    let mut dialog = settings(to.path());
    dialog.existing = ExistingFiles::Skip;
    copy_with(&mut app, &dialog);

    assert_eq!(fs::read(to.path().join("a.txt")).unwrap(), b"old");
    assert_eq!(fs::read(to.path().join("b.txt")).unwrap(), b"new");
    let status = app.status.to_string();
    assert!(status.contains("1 skipped"), "{status}");
}

#[test]
fn only_newer_replaces_an_older_file_and_leaves_a_newer_one() {
    use std::time::{Duration, SystemTime};

    for (label, destination_age, expect_replaced) in [
        ("destination is older", Duration::from_mins(10), true),
        ("destination is newer", Duration::ZERO, false),
    ] {
        let (mut app, _from, to) = app_with(&[("notes.txt", b"new")]);
        let target = to.path().join("notes.txt");
        fs::write(&target, b"old").expect("test setup");
        if destination_age > Duration::ZERO {
            let stale = SystemTime::now() - destination_age;
            fs::File::options()
                .write(true)
                .open(&target)
                .and_then(|file| file.set_modified(stale))
                .expect("test setup");
        }
        mark(&mut app, "notes.txt");

        let mut dialog = settings(to.path());
        dialog.existing = ExistingFiles::OnlyNewer;
        copy_with(&mut app, &dialog);

        let body = fs::read(&target).unwrap();
        if expect_replaced {
            assert_eq!(body, b"new", "{label}");
        } else {
            assert_eq!(body, b"old", "{label}");
        }
    }
}

/// An unreadable timestamp counts as "copy it": refusing because a time
/// could not be read would silently skip the file the user asked about.
/// Exercised directly against `is_newer`, since `copy_one` only calls it once
/// the destination is already known to exist — the "only newer" tests above
/// cover the two-readable-times case, never a missing source.
#[test]
fn is_newer_treats_an_unreadable_source_time_as_newer() {
    let to = TempDir::new().expect("test setup");
    let dest = to.path().join("dest.txt");
    fs::write(&dest, b"x").expect("test setup");
    let missing_source = to.path().join("does-not-exist.txt");

    assert!(
        is_newer(&missing_source, &dest),
        "a source whose time cannot be read must not be silently skipped"
    );
}
