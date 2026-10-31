//! Purpose: Cleanup after an archive run removes only what the run owns: a
//! partial archive that cannot be removed is reported, a link to the archive is
//! never deleted as a source, and a folder reached by another path still holds
//! the archive.

use std::fs;

use tempfile::TempDir;

use super::{contains_archive, delete_sources, discard_partial};
use crate::models::{App, Panel};

fn app_on(root: &std::path::Path) -> App {
    App::with_panels(
        Panel::new(root.to_path_buf()).unwrap(),
        Panel::new(root.to_path_buf()).unwrap(),
    )
}

#[test]
fn a_partial_archive_that_cannot_be_removed_is_reported_on_the_console() {
    let root = TempDir::new().unwrap();
    let archive = root.path().join("out.tar");
    fs::create_dir(&archive).unwrap();
    fs::write(archive.join("kept"), b"x").unwrap();
    let mut app = app_on(root.path());

    discard_partial(&mut app, &archive);

    assert!(archive.join("kept").exists());
    assert!(app
        .console
        .visible(20)
        .iter()
        .any(|line| line.text().contains("could not be removed")));
}

#[cfg(unix)]
#[test]
fn a_link_to_the_archive_is_never_deleted_as_a_source() {
    let root = TempDir::new().unwrap();
    let archive = root.path().join("out.tar");
    fs::write(&archive, b"archive").unwrap();
    std::os::unix::fs::symlink(&archive, root.path().join("alias.tar")).unwrap();

    let failed = delete_sources(root.path(), &["alias.tar".to_owned()], &archive);

    assert_eq!(failed, 0);
    assert!(archive.exists());
    assert!(root.path().join("alias.tar").exists());
}

#[test]
fn a_folder_reached_by_another_path_still_holds_the_archive() {
    let root = TempDir::new().unwrap();
    let folder = root.path().join("sub");
    fs::create_dir(&folder).unwrap();
    let archive = folder.join("out.tar");
    fs::write(&archive, b"archive").unwrap();
    let detour = folder.join("..").join("sub");

    assert!(contains_archive(&detour, &archive));
    assert!(!contains_archive(&root.path().join("other"), &archive));
}
