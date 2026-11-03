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
