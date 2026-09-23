//! Unit tests: `copy_dir_all` must refuse its own destination and must
//! recreate symlinks instead of dereferencing them.

use super::tests::*;

use super::*;

use tempfile::TempDir;

/// Regression: the walk used to descend into the copy it was writing, nesting
/// the tree until the path length gave out.
#[test]
fn copy_dir_all_refuses_destination_inside_source() -> io::Result<()> {
    let _lock = copy_step_mutex().lock().expect("copy step mutex");
    reset_forced_errors();

    let temp_dir = TempDir::new().expect("test setup");
    let source = temp_dir.path().join("docs");
    let inside = source.join("sub");
    fs::create_dir_all(&inside).expect("test setup");
    fs::write(source.join("a.txt"), b"a").expect("test setup");
    fs::write(source.join("b.txt"), b"b").expect("test setup");

    let err = copy_dir_all(&source, &inside.join("docs")).expect_err("must refuse");
    assert_eq!(err.kind(), io::ErrorKind::InvalidInput);

    // The source must be untouched — no duplicated tree left behind.
    assert_eq!(fs::read_dir(&source)?.count(), 3);
    assert_eq!(fs::read_dir(&inside)?.count(), 0);
    Ok(())
}

#[test]
fn copy_dir_all_refuses_destination_equal_to_source() {
    let _lock = copy_step_mutex().lock().expect("copy step mutex");
    reset_forced_errors();

    let temp_dir = TempDir::new().expect("test setup");
    let source = temp_dir.path().join("docs");
    fs::create_dir(&source).expect("test setup");

    let err = copy_dir_all(&source, &source).expect_err("must refuse");
    assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
}

/// The guard compares path *components*, so a sibling whose name merely starts
/// with the source name (`dir` → `dir_1`) is still a legal destination — the
/// exact case `unique_destination` produces when the name is taken.
#[test]
fn copy_dir_all_allows_sibling_with_shared_name_prefix() {
    let _lock = copy_step_mutex().lock().expect("copy step mutex");
    reset_forced_errors();

    let temp_dir = TempDir::new().expect("test setup");
    let source = temp_dir.path().join("dir");
    fs::create_dir(&source).expect("test setup");
    fs::write(source.join("a.txt"), b"a").expect("test setup");
    let dest = temp_dir.path().join("dir_1");
    // The destination must EXIST for this test to mean anything: the guard
    // canonicalises the closest existing ancestor, so an absent `dir_1` makes it
    // compare the parent against `dir` and the `dir_1` component — the whole
    // point of the test — never takes part. Without this line a naive
    // `real_dst.to_string_lossy().starts_with(&*real_src.to_string_lossy())`
    // guard, and even no guard at all, kept the test green.
    fs::create_dir(&dest).expect("test setup");

    copy_dir_all(&source, &dest).expect("sibling copy is legal");

    assert!(dest.join("a.txt").exists());
}

#[cfg(unix)]
#[test]
fn copy_dir_all_recreates_symlinks_instead_of_following_them() -> io::Result<()> {
    use std::os::unix::fs::symlink;

    let _lock = copy_step_mutex().lock().expect("copy step mutex");
    reset_forced_errors();

    let temp_dir = TempDir::new().expect("test setup");
    let src = temp_dir.path().join("src");
    let dst = temp_dir.path().join("dst");
    fs::create_dir(&src).expect("test setup");
    fs::write(src.join("file.txt"), b"content").expect("test setup");
    symlink("file.txt", src.join("relative.txt")).expect("test setup");
    symlink(src.join("file.txt"), src.join("absolute.txt")).expect("test setup");

    copy_dir_all(&src, &dst).expect("copy");

    for link in ["relative.txt", "absolute.txt"] {
        let copied = dst.join(link);
        assert!(
            fs::symlink_metadata(&copied)?.is_symlink(),
            "{link} must stay a symlink, not become a copy of its target"
        );
        assert_eq!(fs::read_link(&copied)?, fs::read_link(src.join(link))?);
    }
    Ok(())
}

/// A broken link used to abort the whole recursion with `NotFound`, leaving a
/// half-copied tree under the destination's final name.
#[cfg(unix)]
#[test]
fn copy_dir_all_copies_broken_symlink() -> io::Result<()> {
    use std::os::unix::fs::symlink;

    let _lock = copy_step_mutex().lock().expect("copy step mutex");
    reset_forced_errors();

    let temp_dir = TempDir::new().expect("test setup");
    let src = temp_dir.path().join("src");
    let dst = temp_dir.path().join("dst");
    fs::create_dir(&src).expect("test setup");
    fs::write(src.join("keep.txt"), b"keep").expect("test setup");
    symlink("../gone/target.js", src.join("dangling")).expect("test setup");

    copy_dir_all(&src, &dst).expect("a broken link must not abort the copy");

    assert!(dst.join("keep.txt").exists());
    assert!(fs::symlink_metadata(dst.join("dangling"))?.is_symlink());
    assert_eq!(
        fs::read_link(dst.join("dangling"))?,
        PathBuf::from("../gone/target.js")
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn copy_dir_all_copies_directory_symlink() -> io::Result<()> {
    use std::os::unix::fs::symlink;

    let _lock = copy_step_mutex().lock().expect("copy step mutex");
    reset_forced_errors();

    let temp_dir = TempDir::new().expect("test setup");
    let src = temp_dir.path().join("src");
    let dst = temp_dir.path().join("dst");
    fs::create_dir_all(src.join("real")).expect("test setup");
    fs::write(src.join("real").join("inner.txt"), b"inner").expect("test setup");
    symlink("real", src.join("alias")).expect("test setup");

    copy_dir_all(&src, &dst).expect("a directory link must not abort the copy");

    assert!(fs::symlink_metadata(dst.join("alias"))?.is_symlink());
    assert_eq!(fs::read_link(dst.join("alias"))?, PathBuf::from("real"));
    assert!(dst.join("real").join("inner.txt").exists());
    Ok(())
}

/// A dangling symlink occupies its name as firmly as any file: every exclusive
/// create fails on it with `AlreadyExists`. `Path::exists` follows the link and
/// called the name free, so the transfer loop kept re-picking it and gave up
/// after 10 000 attempts instead of falling back to `report_1.txt`.
#[cfg(unix)]
#[test]
fn unique_destination_treats_a_dangling_symlink_as_a_taken_name() -> io::Result<()> {
    use std::ffi::OsStr;
    use std::os::unix::fs::symlink;

    let temp_dir = TempDir::new().expect("test setup");
    let taken = temp_dir.path().join("report.txt");
    symlink("nowhere-at-all", &taken).expect("test setup");
    assert!(!taken.exists(), "test setup: the link must be dangling");

    let dest = unique_destination(temp_dir.path(), OsStr::new("report.txt"))?;

    assert_eq!(dest, temp_dir.path().join("report_1.txt"));
    Ok(())
}
