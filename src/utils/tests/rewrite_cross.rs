//! Purpose: Cross-platform unit tests for `rewrite_file` truncation, metadata rules and refusals.

use super::*;
use tempfile::TempDir;

#[test]
fn rewrite_file_truncates_longer_destination_and_preserves_requested_metadata() -> io::Result<()> {
    let dir = TempDir::new()?;
    let src = dir.path().join("src.txt");
    let dst = dir.path().join("dst.txt");
    fs::write(&src, b"short")?;
    fs::write(&dst, b"much longer previous content")?;

    let input = open_regular_file(&src)?;
    let rules = CopyRules {
        times: Times::Source,
        modes: FileMode::Source,
        ..CopyRules::PLAIN
    };
    rewrite_file(&input, &dst, rules)?;
    assert_eq!(fs::read(&dst)?, b"short");
    Ok(())
}

#[test]
fn rewrite_file_refuses_missing_destination_and_directory_on_every_platform() -> io::Result<()> {
    let dir = TempDir::new()?;
    let src = dir.path().join("src.txt");
    fs::write(&src, b"payload")?;
    let input = open_regular_file(&src)?;

    let missing = dir.path().join("missing.txt");
    assert!(rewrite_file(&input, &missing, CopyRules::PLAIN).is_err());

    let folder = dir.path().join("sub");
    fs::create_dir(&folder)?;
    assert!(rewrite_file(&input, &folder, CopyRules::PLAIN).is_err());
    Ok(())
}

#[test]
fn symlink_dir_links_existing_and_dangling_targets() -> io::Result<()> {
    let dir = TempDir::new()?;
    let target = dir.path().join("real_dir");
    fs::create_dir(&target)?;
    fs::write(target.join("inside.txt"), b"ok")?;

    let live_link = dir.path().join("live_link");
    #[cfg(windows)]
    if let Err(err) = symlink_dir(&target.to_string_lossy(), &live_link) {
        if err.kind() == io::ErrorKind::PermissionDenied || err.raw_os_error() == Some(1314) {
            return Ok(());
        }
        return Err(err);
    }
    #[cfg(not(windows))]
    symlink_dir(&target.to_string_lossy(), &live_link)?;
    assert!(fs::symlink_metadata(&live_link)?.file_type().is_symlink());
    assert_eq!(fs::read(live_link.join("inside.txt"))?, b"ok");

    let dangling_link = dir.path().join("dangling_link");
    symlink_dir("not_created_yet", &dangling_link)?;
    assert!(fs::symlink_metadata(&dangling_link)?
        .file_type()
        .is_symlink());
    Ok(())
}
