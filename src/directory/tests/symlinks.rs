//! Symlink, NTFS junction, and unreadable entry-metadata coverage for `directory`.

use super::*;

struct ForceMetadataErrorGuard;

impl ForceMetadataErrorGuard {
    fn arm() -> Self {
        FORCE_ENTRY_METADATA_ERROR.set(true);
        Self
    }
}

impl Drop for ForceMetadataErrorGuard {
    fn drop(&mut self) {
        FORCE_ENTRY_METADATA_ERROR.set(false);
    }
}

#[cfg(unix)]
fn link_directory(target: &Path, link: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(windows)]
fn link_directory(target: &Path, link: &Path) -> io::Result<()> {
    let output = std::process::Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(link)
        .arg(target)
        .output()?;
    assert!(
        output.status.success(),
        "mklink /J failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn unreadable_entry_metadata_skips_the_row_without_failing_the_directory_read() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("vanished.txt"), b"x")?;
    let _guard = ForceMetadataErrorGuard::arm();

    let listing = read(temp.path(), &[])?;

    assert_eq!(listing.entries, []);
    Ok(())
}

#[test]
fn links_classify_directory_and_file_targets() -> io::Result<()> {
    let temp = TempDir::new()?;
    let target_dir = temp.path().join("target_dir");
    let regular_file = temp.path().join("Makefile");
    fs::create_dir(&target_dir)?;
    fs::write(&regular_file, b"all:\n")?;
    let dir_link = temp.path().join("dir_link");
    link_directory(&target_dir, &dir_link)?;
    #[cfg(unix)]
    std::os::unix::fs::symlink(&regular_file, temp.path().join("file_link"))?;

    let listing = read(temp.path(), &[])?;
    let kind_of = |name: &str| {
        listing
            .entries
            .iter()
            .find(|entry| entry.name == name)
            .map(|entry| entry.kind)
    };
    assert_eq!(kind_of("dir_link"), Some(EntryKind::SymlinkToDirectory));
    #[cfg(unix)]
    assert_eq!(kind_of("file_link"), Some(EntryKind::SymlinkToFile));
    let link_meta = fs::symlink_metadata(&dir_link)?;
    let file_meta = fs::metadata(&regular_file)?;
    assert_eq!(
        entry_kind(&link_meta, Some(&file_meta)),
        EntryKind::SymlinkToFile
    );
    Ok(())
}

#[test]
fn removed_link_target_is_reported_as_broken_symlink() -> io::Result<()> {
    let temp = TempDir::new()?;
    let target_dir = temp.path().join("target_dir");
    fs::create_dir(&target_dir)?;
    link_directory(&target_dir, &temp.path().join("dir_link"))?;
    fs::remove_dir(&target_dir)?;
    let after_removal = read(temp.path(), &[])?;
    let broken = after_removal
        .entries
        .iter()
        .find(|entry| entry.name == "dir_link")
        .expect("broken link still listed");
    assert_eq!(broken.kind, EntryKind::BrokenSymlink);
    Ok(())
}

#[test]
fn decode_file_name_reuses_valid_utf8_and_preserves_raw_non_utf8() {
    let (valid, raw_none) = decode_file_name(OsString::from("plain.txt"));
    assert_eq!(valid, "plain.txt");
    assert_eq!(raw_none, None);

    #[cfg(unix)]
    let invalid = {
        use std::os::unix::ffi::OsStringExt;
        OsString::from_vec(vec![b'x', 0xFF])
    };
    #[cfg(windows)]
    let invalid = {
        use std::os::windows::ffi::OsStringExt;
        OsString::from_wide(&[u16::from(b'x'), 0xD800])
    };
    let (lossy, raw_some) = decode_file_name(invalid.clone());
    assert_eq!(lossy, "x\u{FFFD}");
    assert_eq!(raw_some, Some(invalid));
}
