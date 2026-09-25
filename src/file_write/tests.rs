//! Unit tests: the writability guard, the link rules and the
//! temp-file-and-rename every file writer shares.

use std::fs;
use std::io;

use tempfile::TempDir;

use super::{ensure_writable, owned_by_us, real_target, refuse_unwritable, temp_beside};

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
    refuse_unwritable(&missing)?;
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
    assert_eq!(
        error.to_string(),
        format!("{}: permission denied", locked.display())
    );
    // The same refusal without the path, for a caller that names the file.
    let bare = refuse_unwritable(&locked).expect_err("refused");
    assert_eq!(bare.to_string(), "permission denied");
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

/// A path with nothing behind it yet is used as given: a first save creates it.
#[test]
fn real_target_uses_a_missing_plain_path_as_given() -> io::Result<()> {
    let dir = TempDir::new()?;
    let missing = dir.path().join("new.json");
    assert_eq!(real_target(&missing), Ok(missing));
    let file = dir.path().join("file.json");
    fs::write(&file, "{}")?;
    assert_eq!(real_target(&file), Ok(fs::canonicalize(&file)?));
    Ok(())
}

/// A link, and a link to a link, both resolve to the file at the end.
#[cfg(unix)]
#[test]
fn real_target_follows_a_link_and_a_chain_of_links() -> io::Result<()> {
    use std::os::unix::fs::symlink;

    let dir = TempDir::new()?;
    let real = dir.path().join("real.json");
    fs::write(&real, "{}")?;
    let link = dir.path().join("link.json");
    symlink("real.json", &link)?;
    let chain = dir.path().join("chain.json");
    symlink(&link, &chain)?;
    let expected = fs::canonicalize(&real)?;
    assert_eq!(real_target(&link), Ok(expected.clone()));
    assert_eq!(real_target(&chain), Ok(expected));
    Ok(())
}

/// Writing through a dangling link would replace the link with a regular
/// file; it is refused, naming where the link points.
#[cfg(unix)]
#[test]
fn real_target_refuses_a_dangling_link() -> io::Result<()> {
    let dir = TempDir::new()?;
    let missing = dir.path().join("gone.json");
    let link = dir.path().join("settings.yaml");
    std::os::unix::fs::symlink(&missing, &link)?;
    let refused = real_target(&link).expect_err("a dangling link");
    assert_eq!(refused, super::DanglingLink(missing));
    let error = refused.into_io_error();
    assert_eq!(error.kind(), io::ErrorKind::NotFound);
    assert!(error.to_string().contains("gone.json"), "{error}");
    assert!(fs::symlink_metadata(&link)?.is_symlink());
    Ok(())
}

#[cfg(unix)]
#[test]
fn has_other_names_sees_a_hard_link() -> io::Result<()> {
    use super::has_other_names;

    let dir = TempDir::new()?;
    let file = dir.path().join("one.json");
    fs::write(&file, "{}")?;
    assert!(!has_other_names(&file));
    assert!(!super::replacing_changes_identity(&file));
    fs::hard_link(&file, dir.path().join("two.json"))?;
    assert!(has_other_names(&file));
    assert!(super::replacing_changes_identity(&file));
    assert!(!has_other_names(&dir.path().join("missing.json")));
    Ok(())
}

/// A file this process created is its own; a missing one counts as owned.
#[test]
fn owned_by_us_is_true_for_our_own_and_for_a_missing_file() -> io::Result<()> {
    let dir = TempDir::new()?;
    let file = dir.path().join("ours.json");
    fs::write(&file, "{}")?;
    assert!(owned_by_us(&file));
    assert!(owned_by_us(&dir.path().join("missing.json")));
    Ok(())
}

/// Another user's file is not ours. Root gives a file of its own away; any
/// other user already has one to hand, the root directory.
#[cfg(unix)]
#[test]
fn owned_by_us_is_false_for_another_users_file() -> io::Result<()> {
    const ANOTHER_UID: u32 = 4_242_424;
    let dir = TempDir::new()?;
    let theirs = if crate::platform::is_root() {
        let file = dir.path().join("theirs.json");
        fs::write(&file, "{}")?;
        std::os::unix::fs::chown(&file, Some(ANOTHER_UID), None)?;
        file
    } else {
        std::path::PathBuf::from("/")
    };
    assert!(!owned_by_us(&theirs), "{}", theirs.display());
    Ok(())
}
