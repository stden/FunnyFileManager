//! Purpose: Unit tests for non-regular destination refusal when rewriting files in place.

#[cfg(windows)]
#[test]
fn rewrite_file_rejects_windows_character_device_destination() {
    let temp = tempfile::TempDir::new().unwrap();
    let src = temp.path().join("src.txt");
    std::fs::write(&src, b"data").unwrap();
    let input = std::fs::File::open(&src).unwrap();
    let err = super::rewrite_file(
        &input,
        std::path::Path::new("NUL"),
        crate::utils::CopyRules::PLAIN,
    )
    .unwrap_err();
    assert_ne!(err.kind(), std::io::ErrorKind::NotFound);
}

#[cfg(unix)]
#[test]
fn rewrite_file_rejects_unix_device_and_self_overwrite() {
    let temp = tempfile::TempDir::new().unwrap();
    let src = temp.path().join("src.txt");
    std::fs::write(&src, b"data").unwrap();
    let input = std::fs::File::open(&src).unwrap();
    let dev_err = super::rewrite_file(
        &input,
        std::path::Path::new("/dev/null"),
        crate::utils::CopyRules::PLAIN,
    )
    .unwrap_err();
    assert_eq!(dev_err.kind(), std::io::ErrorKind::InvalidInput);

    let self_err = super::rewrite_file(&input, &src, crate::utils::CopyRules::PLAIN).unwrap_err();
    assert_eq!(self_err.kind(), std::io::ErrorKind::InvalidInput);
    assert_eq!(std::fs::read(&src).unwrap(), b"data");
}

#[test]
fn rewrite_file_replaces_regular_file_under_both_source_and_fresh_rules() {
    use crate::utils::{CopyRules, FileMode, LinkPolicy, Times};

    let temp = tempfile::TempDir::new().unwrap();
    let src = temp.path().join("src.txt");
    let dst = temp.path().join("dst.txt");
    std::fs::write(&src, b"replacement").unwrap();
    std::fs::write(&dst, b"old").unwrap();
    let input = std::fs::File::open(&src).unwrap();
    super::rewrite_file(&input, &dst, CopyRules::MOVE).unwrap();
    assert_eq!(std::fs::read(&dst).unwrap(), b"replacement");

    let fresh_src = temp.path().join("src2.txt");
    std::fs::write(&fresh_src, b"fresh").unwrap();
    let fresh_input = std::fs::File::open(&fresh_src).unwrap();
    let fresh_rules = CopyRules {
        links: LinkPolicy::Recreate,
        modes: FileMode::Fresh,
        folder_modes: FileMode::Fresh,
        times: Times::Fresh,
    };
    super::rewrite_file(&fresh_input, &dst, fresh_rules).unwrap();
    assert_eq!(std::fs::read(&dst).unwrap(), b"fresh");
    assert_eq!(
        super::ensure_regular_file(false).unwrap_err().kind(),
        std::io::ErrorKind::InvalidInput
    );
}
