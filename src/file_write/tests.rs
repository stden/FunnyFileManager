//! Unit tests: the writability guard and the temp-file-and-rename both file
//! writers share.

use std::fs;
use std::io;

use tempfile::TempDir;

use super::{ensure_writable, temp_beside};

/// A file that is not there yet is the directory's business: the guard lets
/// the write that follows report whatever really happens.
#[test]
fn ensure_writable_lets_a_new_or_writable_file_through() -> io::Result<()> {
    let dir = TempDir::new()?;
    let missing = dir.path().join("new.txt");
    assert_eq!(super::refusal(&missing), None, "ENOENT falls open");
    ensure_writable(&missing)?;
    let existing = dir.path().join("existing.txt");
    fs::write(&existing, "text")?;
    ensure_writable(&existing)
}

/// A file this process may not write is refused with the kernel's answer.
/// Root may write anything, so under root the guard must let it through.
#[cfg(unix)]
#[test]
fn ensure_writable_refuses_a_read_only_file() -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt as _;

    let dir = TempDir::new()?;
    let locked = dir.path().join("locked.txt");
    fs::write(&locked, "text")?;
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o444))?;

    let result = ensure_writable(&locked);

    if crate::platform::is_root() {
        return result;
    }
    assert_eq!(
        super::refusal(&locked),
        Some(super::WriteRefusal::PermissionDenied)
    );
    let error = result.expect_err("a read-only file is refused");
    assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
    assert!(error.to_string().contains("permission denied"), "{error}");
    Ok(())
}

/// The temp file lives beside the target, so the rename stays on one
/// filesystem; persisting it replaces the target and keeps its mode.
#[cfg(unix)]
#[test]
fn temp_beside_and_persist_over_replace_the_file_and_keep_its_mode() -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt as _;

    let dir = TempDir::new()?;
    let target = dir.path().join("notes.txt");
    fs::write(&target, "old")?;
    fs::set_permissions(&target, fs::Permissions::from_mode(0o640))?;

    let temp = temp_beside(&target)?;
    assert_eq!(temp.path().parent(), Some(dir.path()));
    super::persist_over(temp, &target, b"new")?;

    assert_eq!(fs::read(&target)?, b"new");
    let mode = fs::metadata(&target)?.permissions().mode() & 0o777;
    assert_eq!(mode, 0o640);
    assert_eq!(fs::read_dir(dir.path())?.count(), 1, "no temp file is left");
    Ok(())
}

/// When the directory cannot hold a temp file, the error names the
/// directory, not a random temp name the user never chose.
#[test]
fn temp_beside_names_the_directory_that_refused() {
    let dir = TempDir::new().expect("temporary directory");
    let missing = dir.path().join("gone").join("notes.txt");

    let error = temp_beside(&missing).expect_err("the directory does not exist");

    let message = error.to_string();
    assert!(message.contains("gone"), "{message}");
    assert!(message.starts_with("cannot create a file in"), "{message}");
}
