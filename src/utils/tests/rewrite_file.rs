//! Purpose: unit tests for writing new contents through an existing file.

use super::*;

use tempfile::TempDir;

#[test]
fn rewrite_file_keeps_the_inode_and_leaves_a_folder_alone() -> io::Result<()> {
    use std::os::unix::fs::MetadataExt;

    let dir = TempDir::new()?;
    let (source, dest) = (dir.path().join("source"), dir.path().join("dest"));
    fs::write(&source, b"new")?;
    fs::write(&dest, b"a longer old text")?;
    let inode = fs::metadata(&dest)?.ino();
    let rules = CopyRules {
        modes: FileMode::Fresh,
        ..CopyRules::PLAIN
    };

    rewrite_file(&open_regular_file(&source)?, &dest, rules)?;

    assert_eq!(fs::read(&dest)?, b"new");
    assert_eq!(fs::metadata(&dest)?.ino(), inode);
    let folder = dir.path().join("folder");
    fs::create_dir(&folder)?;
    rewrite_file(&open_regular_file(&source)?, &folder, rules).expect_err("not a file");
    assert!(
        open_regular_file(&folder).is_err(),
        "a folder is not opened"
    );
    Ok(())
}

/// Something that is not a regular file is refused before anything is
/// written: a device opens and is turned away, a FIFO with no reader is not
/// waited for.
#[test]
fn rewrite_file_refuses_what_is_not_a_regular_file_without_waiting() -> io::Result<()> {
    let dir = TempDir::new()?;
    let source = dir.path().join("source");
    fs::write(&source, b"new")?;
    let input = open_regular_file(&source)?;

    let device = rewrite_file(&input, Path::new("/dev/null"), CopyRules::PLAIN);
    assert!(device
        .expect_err("a device")
        .to_string()
        .contains("not a regular file"));
    let fifo = dir.path().join("fifo");
    if crate::test_support::make_fifo(&fifo).is_ok() {
        rewrite_file(&input, &fifo, CopyRules::PLAIN).expect_err("no reader, no wait");
    }
    Ok(())
}
