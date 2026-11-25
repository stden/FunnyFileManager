//! Purpose: The real wipe adapter truncates the inode before removing its name.
#![cfg(unix)]

use std::{fs, io};

#[test]
fn wiping_a_file_truncates_its_open_inode_and_preserves_its_neighbour() -> io::Result<()> {
    let root = tempfile::TempDir::new()?;
    let path = root.path().join("wipe");
    let neighbour = root.path().join("keep");
    fs::write(&path, b"private payload")?;
    fs::write(&neighbour, b"kept")?;
    let handle = fs::File::open(&path)?;

    let shared = funny_file_manager::ops::wipe::wipe_path(&path)?;

    assert_eq!(shared, 0);
    assert!(!path.exists());
    assert_eq!(handle.metadata()?.len(), 0);
    assert_eq!(fs::read(neighbour)?, b"kept");
    Ok(())
}
