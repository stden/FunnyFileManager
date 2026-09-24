//! Purpose: unit tests for what `copy_dir_all` and `copy_symlink` do when a real step fails.

use super::*;

// Only the Unix branch of the permission test writes through a `File`.
#[cfg(unix)]
use std::fs::File;
#[cfg(unix)]
use std::io::Write;
use tempfile::TempDir;

#[test]
fn test_copy_dir_all_src_not_exist() -> io::Result<()> {
    let _lock = copy_step_mutex().lock().expect("copy step mutex");
    let temp_dir = TempDir::new().expect("test setup");
    let src = temp_dir.path().join("non_existent_src");
    let dst = temp_dir.path().join("dst");

    let result = copy_dir_all(&src, &dst);
    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_copy_dir_all_src_not_exist_rebase_copy() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let src = temp_dir.path().join("non_existent_src");
    let dst = temp_dir.path().join("dst");

    let result = copy_dir_all(&src, &dst);
    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_copy_dir_all_permission_errors() -> io::Result<()> {
    let _lock = copy_step_mutex().lock().expect("copy step mutex");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = TempDir::new().expect("test setup");
        let src = temp_dir.path().join("src");
        let dst = temp_dir.path().join("dst");

        fs::create_dir(&src).expect("test setup");
        File::create(src.join("file.txt"))
            .expect("test setup")
            .write_all(b"content")
            .expect("test setup");

        fs::create_dir(&dst).expect("test setup");
        let mut perms = fs::metadata(&dst).expect("test setup").permissions();
        perms.set_mode(0o000);
        fs::set_permissions(&dst, perms).expect("test setup");

        // Root bypasses Unix DAC permissions, so a 0o000 dst may still be
        // writable; only assert failure when perms are actually enforced.
        let perms_enforced = fs::read_dir(&dst).is_err();
        let result = copy_dir_all(&src, &dst);

        let mut perms = fs::metadata(&dst).expect("test setup").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&dst, perms).expect("test setup");

        if perms_enforced {
            assert!(result.is_err());
        }
    }

    Ok(())
}

#[test]
fn copy_dir_all_reports_not_found_for_a_missing_source() {
    let _lock = copy_step_mutex().lock().expect("copy step mutex");
    // Only the copy step is reset: `reset_forced_errors` would also clear the
    // refresh flag, which a different mutex guards.
    set_copy_dir_all_step(0);

    // Two separate roots: the self-copy guard compares canonicalised
    // ancestors, and a shared parent would trip it before `read_dir` runs.
    let source_root = TempDir::new().expect("test setup");
    let dest_root = TempDir::new().expect("test setup");
    let dest = dest_root.path().join("out");

    let err = copy_dir_all(&source_root.path().join("missing"), &dest)
        .expect_err("a missing source cannot be copied");

    assert_eq!(err.kind(), io::ErrorKind::NotFound);
    assert!(
        !dest.exists() || fs::read_dir(&dest).expect("dest dir").next().is_none(),
        "a failed copy must not leave files behind"
    );
}

/// A relative path that does not exist has no canonicalisable ancestor, so the
/// self-copy guard cannot compare anything. The copy must then fail honestly
/// at `read_dir` instead of being rejected as a copy into itself.
#[test]
fn copy_dir_all_without_canonical_ancestors_fails_at_read_dir() {
    let _lock = copy_step_mutex().lock().expect("copy step mutex");
    // Only the copy step is reset: `reset_forced_errors` would also clear the
    // refresh flag, which a different mutex guards.
    set_copy_dir_all_step(0);

    let dest_root = TempDir::new().expect("test setup");

    let err = copy_dir_all(
        Path::new("ffm-no-such-relative-source"),
        &dest_root.path().join("out"),
    )
    .expect_err("a missing source cannot be copied");

    assert_eq!(err.kind(), io::ErrorKind::NotFound);
}

/// Copying must never silently replace an existing entry: a destination name
/// already taken makes the symlink step fail and leaves the entry alone.
#[cfg(unix)]
#[test]
fn copy_dir_all_fails_when_a_symlink_name_is_already_taken() -> io::Result<()> {
    use std::os::unix::fs::symlink;

    let _lock = copy_step_mutex().lock().expect("copy step mutex");
    // Only the copy step is reset: `reset_forced_errors` would also clear the
    // refresh flag, which a different mutex guards.
    set_copy_dir_all_step(0);

    let temp_dir = TempDir::new().expect("test setup");
    let src = temp_dir.path().join("src");
    let dst = temp_dir.path().join("dst");
    fs::create_dir(&src).expect("test setup");
    fs::write(src.join("target.txt"), b"target").expect("test setup");
    symlink("target.txt", src.join("link")).expect("test setup");

    fs::create_dir(&dst).expect("test setup");
    fs::write(dst.join("link"), b"existing").expect("test setup");

    let err = copy_dir_all(&src, &dst).expect_err("an occupied name must not be overwritten");

    assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
    assert_eq!(fs::read(dst.join("link"))?, b"existing");
    Ok(())
}

/// `copy_symlink` reads the link target first. Handed something that is not a
/// link, it must report that failure and create nothing.
#[test]
fn copy_symlink_reports_a_read_link_failure_and_creates_nothing() {
    let temp_dir = TempDir::new().expect("test setup");
    let regular = temp_dir.path().join("regular.txt");
    fs::write(&regular, b"not a link").expect("test setup");
    let dest = temp_dir.path().join("copy.txt");

    copy_symlink(&regular, &dest).expect_err("a regular file has no link target");

    assert!(
        !dest.exists(),
        "nothing may be created when the target cannot be read"
    );
}
