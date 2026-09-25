//! Purpose: What the dialog's "already existing files" choice does to a file already there.

use super::*;
use crate::ops::file::placement::is_newer;

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

/// `pkg` on both sides — `a.txt` and `sub/s.txt` new in `from` and old in
/// `to`, plus `only-src.txt` and `only-dst.txt` — sent over with "Skip".
fn skip_onto_a_folder(purpose: Purpose) -> (App, TempDir, TempDir) {
    let (mut app, from, to) = app_with(&[]);
    for (root, body) in [(from.path(), &b"new"[..]), (to.path(), &b"old"[..])] {
        fs::create_dir_all(root.join("pkg/sub")).expect("test setup");
        fs::write(root.join("pkg/a.txt"), body).expect("test setup");
        fs::write(root.join("pkg/sub/s.txt"), body).expect("test setup");
    }
    fs::write(from.path().join("pkg/only-src.txt"), b"src").expect("test setup");
    fs::write(to.path().join("pkg/only-dst.txt"), b"dst").expect("test setup");
    app.get_active_panel().load_entries().expect("reload");
    mark(&mut app, "pkg");
    let mut dialog = settings(to.path());
    dialog.purpose = purpose;
    dialog.existing = ExistingFiles::Skip;
    copy_with(&mut app, &dialog);
    (app, from, to)
}

/// "Skip" is about files already there, not about folders: a folder onto a
/// folder of the same name merges, and only the files the destination has
/// are left alone — Far's and mc's reading, and the one "Overwrite" and
/// "Only newer" already follow.
#[test]
fn skip_merges_a_folder_and_leaves_only_the_files_already_there() {
    for purpose in [Purpose::Copy, Purpose::Move] {
        let (app, from, to) = skip_onto_a_folder(purpose);

        let pkg = to.path().join("pkg");
        let read = |path: PathBuf| fs::read(path).unwrap();
        assert_eq!(read(pkg.join("only-src.txt")), b"src", "{}", app.status);
        assert_eq!(read(pkg.join("a.txt")), b"old");
        assert_eq!(read(pkg.join("sub/s.txt")), b"old");
        assert_eq!(read(pkg.join("only-dst.txt")), b"dst");
        let source = from.path().join("pkg");
        let moved = !source.join("only-src.txt").exists();
        assert_eq!(moved, purpose == Purpose::Move);
        assert_eq!(read(source.join("sub/s.txt")), b"new", "kept, skipped");
    }
}

/// What "Skip" left alone inside the merged folder is counted, however deep:
/// otherwise "Moved to …/pkg" reads as if all of it went.
#[test]
fn a_skip_merge_counts_every_file_it_left_alone() {
    for purpose in [Purpose::Copy, Purpose::Move] {
        let (app, _from, to) = skip_onto_a_folder(purpose);

        assert_eq!(
            app.status,
            Status::Transferred {
                purpose,
                landed: Landed::At(to.path().join("pkg")),
                skipped: 2,
                renamed: 0,
            }
        );
    }
}
