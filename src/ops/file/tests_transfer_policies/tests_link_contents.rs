//! Purpose: Copy contents of symbolic links: recreate the link, follow it, and refuse loops and links into the destination.

use super::*;

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

#[cfg(unix)]
#[test]
fn merging_a_folder_with_a_followed_link_loop_is_refused() {
    use std::os::unix::fs::symlink;

    let (mut app, from, to) = app_with(&[]);
    let src_tree = from.path().join("tree");
    fs::create_dir(&src_tree).expect("test setup");
    symlink("..", src_tree.join("loop")).expect("test setup");
    fs::create_dir_all(to.path().join("tree/loop")).expect("test setup");
    mark(&mut app, "tree");

    let mut dialog = settings(to.path());
    dialog.existing = ExistingFiles::Overwrite;
    dialog.symlink_contents = true;
    copy_with(&mut app, &dialog);

    assert!(
        matches!(app.status, Status::CopiedWithErrors { .. }),
        "merge loop must be reported: {:?}",
        app.status
    );
    assert!(walk_depth(&to.path().join("tree")) < 6);
}
