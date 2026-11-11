//! Purpose: Cross-platform tests for `unnamed_stage_with` fallback and staged copy paths.

use super::super::*;
use tempfile::TempDir;

#[test]
fn unnamed_stage_with_returns_none_for_non_file_or_unavailable_unnamed_temp() {
    let dir = TempDir::new().unwrap();
    let src = dir.path().join("src.txt");
    fs::write(&src, b"payload").unwrap();
    let settings = CopyPrompt::for_purpose(Purpose::Copy, "src.txt", dir.path());

    assert!(stage::unnamed_stage_with(
        dir.path(),
        dir.path(),
        &settings,
        #[cfg(unix)]
        None,
        |_| panic!("not called"),
        |_, _| panic!("not called"),
    )
    .unwrap()
    .is_none());

    assert!(stage::unnamed_stage_with(
        &src,
        dir.path(),
        &settings,
        #[cfg(unix)]
        None,
        |_| None,
        |_, _| panic!("not called"),
    )
    .unwrap()
    .is_none());
}

#[test]
fn unnamed_stage_with_links_successful_unnamed_temp_file() {
    let dir = TempDir::new().unwrap();
    let src = dir.path().join("src.txt");
    fs::write(&src, b"payload").unwrap();
    let settings = CopyPrompt::for_purpose(Purpose::Copy, "src.txt", dir.path());

    let sink = dir.path().join("sink.tmp");
    let staged = stage::unnamed_stage_with(
        &src,
        dir.path(),
        &settings,
        #[cfg(unix)]
        None,
        |_| fs::File::create(&sink).ok(),
        |_, path| fs::copy(&sink, path).map(|_| ()),
    )
    .unwrap()
    .unwrap();
    assert_eq!(fs::read(staged.path()).unwrap(), b"payload");
}

#[test]
fn stage_with_replaces_or_keeps_destination_on_unnamed_temp_result() {
    let dir = TempDir::new().unwrap();
    let src = dir.path().join("src.txt");
    let dst = dir.path().join("dst.txt");
    fs::write(&src, b"payload").unwrap();
    fs::write(&dst, b"keep").unwrap();
    let settings = CopyPrompt::for_purpose(Purpose::Copy, "src.txt", dir.path());

    let sink2 = dir.path().join("sink2.tmp");
    let err = stage::stage_with(
        &src,
        &dst,
        &settings,
        |_| fs::File::create(&sink2).ok(),
        |_, _| Err(io::Error::other("link failed")),
    )
    .unwrap_err();
    assert_eq!(err.to_string(), "link failed");
    assert_eq!(fs::read(&dst).unwrap(), b"keep");

    let sink3 = dir.path().join("sink3.tmp");
    stage::stage_with(
        &src,
        &dst,
        &settings,
        |_| fs::File::create(&sink3).ok(),
        |_, path| fs::copy(&sink3, path).map(|_| ()),
    )
    .unwrap();
    assert_eq!(fs::read(&dst).unwrap(), b"payload");
}

/// Where no unnamed file can be made, the named staging still gives the
/// replacement the mode of the file it replaces, and leaves no entry behind.
#[cfg(unix)]
#[test]
fn named_staging_without_a_descriptor_view_keeps_the_replaced_mode() {
    use std::os::unix::fs::PermissionsExt;

    let dir = TempDir::new().unwrap();
    let src = dir.path().join("src.txt");
    let dst = dir.path().join("dst.txt");
    fs::write(&src, b"new").unwrap();
    fs::write(&dst, b"old").unwrap();
    fs::set_permissions(&src, fs::Permissions::from_mode(0o750)).unwrap();
    fs::set_permissions(&dst, fs::Permissions::from_mode(0o640)).unwrap();
    let settings = CopyPrompt::for_purpose(Purpose::Copy, "src.txt", dir.path());

    stage::stage_with(&src, &dst, &settings, |_| None, |_, _| panic!("not called")).unwrap();

    assert_eq!(fs::read(&dst).unwrap(), b"new");
    let mode = fs::metadata(&dst).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o640);
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 2);
}
