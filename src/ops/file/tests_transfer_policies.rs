//! Unit tests: what the F5/F6 dialog's options actually do to the filesystem.
//!
//! Every control in that dialog is a promise about bytes on disk — "skip what
//! is already there", "keep the timestamps", "copy the permissions" — and a
//! promise nothing checks is the kind that is quietly wrong for months. The
//! dialog's *state* is tested next door in `copy::tests`; this is about the
//! transfer it ends in.

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use super::*;

use crate::copy::{CopyPrompt, ExistingFiles};
use crate::models::{FileName, Panel, PanelSide};

use tempfile::TempDir;

/// Two panels, left active and holding `sources`, right empty.
pub(super) fn app_with(sources: &[(&str, &[u8])]) -> (App, TempDir, TempDir) {
    let from = TempDir::new().expect("test setup");
    let to = TempDir::new().expect("test setup");
    for (name, body) in sources {
        fs::write(from.path().join(name), body).expect("test setup");
    }
    let app = app_between(from.path(), to.path());
    (app, from, to)
}

/// Two panels on directories that already exist, left active on `from`.
pub(super) fn app_between(from: &Path, to: &Path) -> App {
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(from.to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(to.to_path_buf()).expect("test setup");
    app.active_panel = PanelSide::Left;
    app
}

/// Mark `name` in the active panel, which is what the transfer acts on.
pub(super) fn mark(app: &mut App, name: &str) {
    app.get_active_panel()
        .marked
        .insert(FileName::new(name.to_owned()));
}

/// The dialog as it would be filled in, aimed at `destination`.
pub(super) fn settings(destination: &Path) -> CopyPrompt {
    CopyPrompt::new("marked", destination)
}

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
    dialog.destination = format!("archive{}", std::path::MAIN_SEPARATOR);
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
    dialog.destination = format!("{}{}", archive.display(), std::path::MAIN_SEPARATOR);
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

// ─── the refusals and the awkward destinations ──────────────────────────────

/// Copying a directory into itself would recurse until the disk filled, so it
/// is refused before anything is written.
#[test]
fn a_directory_cannot_be_transferred_into_itself() {
    let (mut app, from, _to) = app_with(&[]);
    let tree = from.path().join("project");
    fs::create_dir_all(&tree).expect("test setup");
    fs::write(tree.join("a.txt"), b"x").expect("test setup");
    app.get_active_panel().load_entries().expect("reload");
    mark(&mut app, "project");

    let mut dialog = settings(&tree);
    dialog.destination = tree.to_string_lossy().into_owned();
    copy_with(&mut app, &dialog);

    assert!(
        !tree.join("project").exists(),
        "nothing was written inside itself"
    );
    let status = app.status.to_string();
    assert!(
        status.to_lowercase().contains("error") || status.contains('1'),
        "and it said so, got {status}"
    );
}

/// A destination name already taken by a *file* while the source is a
/// directory is a plain failure, not a merge or a silent skip.
#[test]
fn a_directory_onto_an_existing_file_fails_rather_than_guessing() {
    let (mut app, from, to) = app_with(&[]);
    fs::create_dir_all(from.path().join("data")).expect("test setup");
    fs::write(to.path().join("data"), b"in the way").expect("test setup");
    app.get_active_panel().load_entries().expect("reload");
    mark(&mut app, "data");

    let mut dialog = settings(to.path());
    dialog.existing = ExistingFiles::Ask;
    copy_with(&mut app, &dialog);

    assert_eq!(
        fs::read(to.path().join("data")).unwrap(),
        b"in the way",
        "the file that was there is untouched"
    );
}

/// When the plain `fs::rename` cannot manage the move, `place` falls back to
/// copying and then removing the original — the same fallback `move_into`
/// uses, but reached through the F6 dialog's own path. Denying write on the
/// source directory is what forces the fallback here: `fs::rename` needs
/// write permission on both parent directories and so fails, while reading
/// the file to copy it does not.
#[cfg(unix)]
#[test]
fn a_move_whose_rename_fails_falls_back_to_copy_and_remove() {
    let (mut app, from, to) = app_with(&[("notes.txt", b"payload")]);
    mark(&mut app, "notes.txt");

    fs::set_permissions(from.path(), fs::Permissions::from_mode(0o555)).expect("test setup");

    let mut dialog = settings(to.path());
    dialog.purpose = Purpose::Move;
    copy_with(&mut app, &dialog);

    // Restore so the `TempDir` can clean itself up.
    fs::set_permissions(from.path(), fs::Permissions::from_mode(0o755)).expect("cleanup");

    assert_eq!(
        fs::read(to.path().join("notes.txt")).unwrap(),
        b"payload",
        "the fallback's copy still has to land, even though the cleanup after it is denied"
    );
}

/// A move whose source vanishes between the listing and the transfer reports
/// the failure rather than claiming a move that did not happen.
#[test]
fn a_move_of_something_that_is_no_longer_there_is_reported() {
    let (mut app, _from, to) = app_with(&[]);
    mark(&mut app, "ghost.txt");

    let mut dialog = settings(to.path());
    dialog.purpose = Purpose::Move;
    copy_with(&mut app, &dialog);

    let status = app.status.to_string();
    assert!(
        status.to_lowercase().contains("error") || status.contains('1'),
        "got {status}"
    );
}

// ─── a file onto itself ─────────────────────────────────────────────────────

/// Both panels start in the same directory, so the dialog's default
/// destination is often the source's own. "Overwrite" there used to remove the
/// file and then copy from what was no longer there, leaving it empty.
#[test]
fn overwriting_a_file_onto_itself_is_refused_and_the_file_survives() {
    for purpose in [Purpose::Copy, Purpose::Move] {
        let (mut app, from, _to) = app_with(&[("notes.txt", b"payload")]);
        mark(&mut app, "notes.txt");

        let mut dialog = settings(from.path());
        dialog.purpose = purpose;
        dialog.existing = ExistingFiles::Overwrite;
        copy_with(&mut app, &dialog);

        assert_eq!(
            fs::read(from.path().join("notes.txt")).unwrap(),
            b"payload",
            "{purpose:?} onto itself must leave the bytes alone"
        );
        let status = app.status.to_string();
        assert!(
            status.to_lowercase().contains("error"),
            "{purpose:?}: and say why nothing happened, got {status}"
        );
    }
}

/// "Only newer" onto itself is a skip, not a failure: a file is never newer
/// than itself.
#[test]
fn only_newer_onto_itself_is_skipped() {
    let (mut app, from, _to) = app_with(&[("notes.txt", b"payload")]);
    mark(&mut app, "notes.txt");

    let mut dialog = settings(from.path());
    dialog.existing = ExistingFiles::OnlyNewer;
    copy_with(&mut app, &dialog);

    assert_eq!(fs::read(from.path().join("notes.txt")).unwrap(), b"payload");
    assert_eq!(
        app.status,
        Status::Transferred {
            purpose: Purpose::Copy,
            landed: Landed::Nowhere,
            skipped: 1,
            renamed: 0,
        }
    );
}

/// Preserving timestamps applies to a directory as much as to a file, and a
/// directory cannot be opened for writing — so it must not turn a complete
/// copy into a reported failure.
#[test]
fn preserving_timestamps_works_for_a_directory_too() {
    use std::time::{Duration, SystemTime};

    let (mut app, from, to) = app_with(&[]);
    let tree = from.path().join("project");
    fs::create_dir_all(&tree).expect("test setup");
    fs::write(tree.join("a.txt"), b"x").expect("test setup");
    let long_ago = SystemTime::now() - Duration::from_hours(24);
    crate::ops::timestamps::open(&tree)
        .and_then(|dir| dir.set_modified(long_ago))
        .expect("test setup");
    app.get_active_panel().load_entries().expect("reload");
    mark(&mut app, "project");

    let mut dialog = settings(to.path());
    dialog.timestamps = true;
    copy_with(&mut app, &dialog);

    let copied = to.path().join("project");
    assert_eq!(fs::read(copied.join("a.txt")).unwrap(), b"x");
    let status = app.status.to_string();
    assert!(
        !status.to_lowercase().contains("error"),
        "a complete copy is not a failure, got {status}"
    );
    let source_time = fs::metadata(&tree).unwrap().modified().unwrap();
    let copied_time = fs::metadata(&copied).unwrap().modified().unwrap();
    assert_eq!(copied_time, source_time);
}

/// A move across filesystems with timestamps ticked used to fail after the
/// copy and so never remove the source; with a same-filesystem rename the
/// question does not arise, but the fallback must reach the removal.
#[test]
fn a_move_of_a_directory_with_timestamps_still_removes_the_source() {
    use std::sync::atomic::Ordering;

    let _guard = RENAME_TEST_MUTEX
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let (mut app, from, to) = app_with(&[]);
    let tree = from.path().join("project");
    fs::create_dir_all(&tree).expect("test setup");
    fs::write(tree.join("a.txt"), b"x").expect("test setup");
    app.get_active_panel().load_entries().expect("reload");
    mark(&mut app, "project");

    let mut dialog = settings(to.path());
    dialog.purpose = Purpose::Move;
    dialog.timestamps = true;
    FORCE_RENAME_FAILURE.store(true, Ordering::SeqCst);
    copy_with(&mut app, &dialog);
    FORCE_RENAME_FAILURE.store(false, Ordering::SeqCst);

    assert_eq!(fs::read(to.path().join("project/a.txt")).unwrap(), b"x");
    assert!(!tree.exists(), "the fallback reached the removal");
}

/// A destination that does not exist yet, nested inside the source, must be
/// refused before anything is created — or the refusal leaves a stray
/// directory inside the tree it protected.
#[test]
fn a_nested_destination_inside_the_source_leaves_nothing_behind() {
    let (mut app, from, _to) = app_with(&[]);
    let tree = from.path().join("project");
    fs::create_dir_all(&tree).expect("test setup");
    fs::write(tree.join("a.txt"), b"x").expect("test setup");
    app.get_active_panel().load_entries().expect("reload");
    mark(&mut app, "project");

    let mut dialog = settings(&tree);
    dialog.destination = tree.join("archive/2026").to_string_lossy().into_owned();
    copy_with(&mut app, &dialog);

    assert!(!tree.join("archive").exists(), "nothing was created inside");
    assert!(
        app.status.to_string().to_lowercase().contains("error"),
        "{}",
        app.status
    );
}

// ─── copy contents of symbolic links ────────────────────────────────────────

/// Off — the default — a link is recreated, pointing where it always did.
#[cfg(unix)]
#[test]
fn a_link_is_recreated_when_its_contents_are_not_wanted() {
    use std::os::unix::fs::symlink;

    let (mut app, from, to) = app_with(&[("target.txt", b"contents")]);
    symlink("target.txt", from.path().join("link.txt")).expect("test setup");
    mark(&mut app, "link.txt");

    let dialog = settings(to.path());
    copy_with(&mut app, &dialog);

    let copied = to.path().join("link.txt");
    assert!(
        fs::symlink_metadata(&copied).expect("copied").is_symlink(),
        "the link was followed although the box was clear"
    );
}

/// On, the link is replaced by what it points at — Far's "Copy contents of
/// symbolic links". Before this the box could be ticked and changed nothing.
#[cfg(unix)]
#[test]
fn ticking_symlink_contents_copies_what_the_link_points_at() {
    use std::os::unix::fs::symlink;

    let (mut app, from, to) = app_with(&[("target.txt", b"contents")]);
    symlink("target.txt", from.path().join("link.txt")).expect("test setup");
    mark(&mut app, "link.txt");

    let mut dialog = settings(to.path());
    dialog.symlink_contents = true;
    copy_with(&mut app, &dialog);

    let copied = to.path().join("link.txt");
    assert!(
        !fs::symlink_metadata(&copied).expect("copied").is_symlink(),
        "the link was recreated although the box was ticked"
    );
    assert_eq!(fs::read(&copied).expect("contents"), b"contents");
}

/// Following links is what makes a tree able to contain itself, so the walk
/// refuses to re-enter a directory it is already inside instead of recursing
/// until the path length gives out. `ln -s .. loop` is the one-line way to
/// arrange it.
#[cfg(unix)]
#[test]
fn a_link_loop_is_refused_instead_of_walked_forever() {
    use std::os::unix::fs::symlink;

    let (mut app, from, to) = app_with(&[]);
    let tree = from.path().join("tree");
    fs::create_dir(&tree).expect("test setup");
    fs::write(tree.join("file.txt"), b"x").expect("test setup");
    symlink("..", tree.join("loop")).expect("test setup");
    mark(&mut app, "tree");

    let mut dialog = settings(to.path());
    dialog.symlink_contents = true;
    copy_with(&mut app, &dialog);

    // The copy failed, and it failed quickly: what matters is that the walk
    // came back at all, and left no runaway tree behind.
    assert!(
        matches!(app.status, Status::CopiedWithErrors { .. }),
        "the loop must be reported, not silently truncated: {:?}",
        app.status
    );
    let depth = walk_depth(&to.path().join("tree"));
    assert!(depth < 8, "the walk kept descending: {depth} levels deep");
}

/// A link into the copy's own output is the loop the `entered` stack cannot
/// see: every level is a path the walk has never met, so it would copy what it
/// is writing until the disk or the path length gave out.
#[cfg(unix)]
#[test]
fn a_link_into_the_destination_is_refused() {
    use std::os::unix::fs::symlink;

    let (mut app, from, to) = app_with(&[]);
    let tree = from.path().join("docs");
    fs::create_dir(&tree).expect("test setup");
    fs::write(tree.join("note.txt"), b"x").expect("test setup");
    symlink(to.path(), tree.join("archive")).expect("test setup");
    mark(&mut app, "docs");

    let mut dialog = settings(to.path());
    dialog.symlink_contents = true;
    copy_with(&mut app, &dialog);

    assert!(
        matches!(app.status, Status::CopiedWithErrors { .. }),
        "reaching the destination must be reported: {:?}",
        app.status
    );
    let depth = walk_depth(&to.path().join("docs"));
    assert!(
        depth < 5,
        "the walk copied its own output: {depth} levels deep"
    );
}

/// A link with nothing behind it is carried over as a link. Following it would
/// fail the whole transfer, and `copy_directory_entry` would then delete every
/// file that had already landed.
#[cfg(unix)]
#[test]
fn a_dangling_link_does_not_take_the_whole_copy_with_it() {
    use std::os::unix::fs::symlink;

    let (mut app, from, to) = app_with(&[]);
    let tree = from.path().join("tree");
    fs::create_dir(&tree).expect("test setup");
    fs::write(tree.join("kept.txt"), b"kept").expect("test setup");
    symlink("nowhere", tree.join("stale")).expect("test setup");
    mark(&mut app, "tree");

    let mut dialog = settings(to.path());
    dialog.symlink_contents = true;
    copy_with(&mut app, &dialog);

    assert_eq!(
        fs::read(to.path().join("tree/kept.txt")).expect("the copy survived"),
        b"kept"
    );
    assert!(fs::symlink_metadata(to.path().join("tree/stale"))
        .expect("the stale link was carried over")
        .is_symlink());
}

/// A link to a FIFO is carried over too: following it would block the file
/// manager inside `open` until a writer appeared, with the event loop stopped
/// and no key able to reach it.
#[cfg(unix)]
#[test]
fn a_link_to_a_fifo_is_recreated_rather_than_opened() {
    use std::os::unix::fs::symlink;

    let (mut app, from, to) = app_with(&[]);
    // The FIFO lives outside the tree being copied: what is under test is the
    // link to it, not the node itself.
    let Some(fifo) = make_fifo(&from.path().join("outside.fifo")) else {
        return;
    };
    let tree = from.path().join("tree");
    fs::create_dir(&tree).expect("test setup");
    fs::write(tree.join("kept.txt"), b"kept").expect("test setup");
    symlink(&fifo, tree.join("link-to-pipe")).expect("test setup");
    mark(&mut app, "tree");

    let mut dialog = settings(to.path());
    dialog.symlink_contents = true;
    copy_with(&mut app, &dialog);

    assert!(
        fs::symlink_metadata(to.path().join("tree/link-to-pipe"))
            .expect("the link was carried over")
            .is_symlink(),
        "the link was followed into a pipe"
    );
    assert_eq!(
        fs::read(to.path().join("tree/kept.txt")).expect("copied"),
        b"kept"
    );
}

/// And the node itself is reported, not opened. `fs::copy` opens the source
/// before it checks what it is, so this used to stop the file manager dead.
#[cfg(unix)]
#[test]
fn copying_a_fifo_reports_it_instead_of_waiting_for_a_writer() {
    let (mut app, from, to) = app_with(&[]);
    let tree = from.path().join("tree");
    fs::create_dir(&tree).expect("test setup");
    if make_fifo(&tree.join("pipe")).is_none() {
        return;
    }
    mark(&mut app, "tree");

    copy_with(&mut app, &settings(to.path()));

    assert!(
        matches!(app.status, Status::CopiedWithErrors { .. }),
        "a pipe must be reported: {:?}",
        app.status
    );
}

/// Make a FIFO at `path`, or `None` where the platform will not have one.
#[cfg(unix)]
pub(super) fn make_fifo(path: &Path) -> Option<PathBuf> {
    crate::test_support::make_fifo(path).ok()?;
    Some(path.to_path_buf())
}

/// How deep the tree under `root` goes, so a runaway walk is visible as a
/// number rather than as a test that never ends.
#[cfg(unix)]
fn walk_depth(root: &Path) -> usize {
    let Ok(entries) = fs::read_dir(root) else {
        return 0;
    };
    1 + entries
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .map(|entry| walk_depth(&entry.path()))
        .max()
        .unwrap_or(0)
}

#[test]
fn zz_probe_create_new_already_exists() {
    let dir = TempDir::new().expect("setup");
    let source = dir.path().join("a.txt");
    fs::write(&source, b"x").expect("setup");
    let dest = dir.path().join("b.txt");
    fs::write(&dest, b"y").expect("setup");
    let err = copy_entry(&source, &dest, CopyRules::PLAIN).expect_err("must fail");
    assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
    assert_eq!(
        fs::read(&dest).unwrap(),
        b"y",
        "the file in the way is untouched"
    );
}

#[cfg(unix)]
#[test]
fn zz_probe_dangling_symlink_dest_via_public_copy() {
    let (mut app, _from, to) = app_with(&[("notes.txt", b"payload")]);
    std::os::unix::fs::symlink("nowhere-at-all", to.path().join("notes.txt")).expect("setup");
    mark(&mut app, "notes.txt");
    let mut dialog = settings(to.path());
    dialog.existing = ExistingFiles::Overwrite;
    copy_with(&mut app, &dialog);
    let status = app.status.to_string();
    assert!(
        status.contains("error") || status.contains("Error") || status.contains("exists"),
        "{status}"
    );
}

#[cfg(unix)]
#[test]
fn zz_probe_readonly_target_dir_permission_denied() {
    let (mut app, _from, to) = app_with(&[("notes.txt", b"payload")]);
    mark(&mut app, "notes.txt");
    fs::set_permissions(to.path(), fs::Permissions::from_mode(0o555)).expect("setup");
    let euid = crate::platform::effective_uid();
    let result = handle_copy(&mut app);
    fs::set_permissions(to.path(), fs::Permissions::from_mode(0o755)).expect("cleanup");
    let status = app.status.to_string();
    eprintln!("euid={euid} result={result:?} status={status}");
    assert!(
        euid == 0
            || status.to_lowercase().contains("denied")
            || status.to_lowercase().contains("error"),
        "{status}"
    );
}
