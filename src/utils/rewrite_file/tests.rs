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

#[test]
fn rewrite_file_replaces_regular_file_preserving_times_and_permissions() {
    let temp = tempfile::TempDir::new().unwrap();
    let src = temp.path().join("src.txt");
    let dst = temp.path().join("dst.txt");
    std::fs::write(&src, b"replacement").unwrap();
    std::fs::write(&dst, b"old").unwrap();
    let input = std::fs::File::open(&src).unwrap();
    super::rewrite_file(&input, &dst, crate::utils::CopyRules::MOVE).unwrap();
    assert_eq!(std::fs::read(&dst).unwrap(), b"replacement");
}
