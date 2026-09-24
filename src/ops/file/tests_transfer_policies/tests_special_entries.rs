//! Purpose: Dangling links, FIFOs, and the probes for claimed names and read-only folders.

use super::*;

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
