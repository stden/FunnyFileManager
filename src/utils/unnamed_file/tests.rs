//! Purpose: Unit tests for unnamed-file fallback behavior on non-Linux hosts.

#[test]
fn unnamed_file_fallback_returns_none_and_unsupported_on_non_linux() {
    let temp = tempfile::TempDir::new().unwrap();
    let sample = temp.path().join("sample.txt");
    let file = std::fs::File::create(&sample).unwrap();
    #[cfg(not(target_os = "linux"))]
    {
        assert!(super::unnamed_file_in(temp.path()).is_none());
        let err = super::name_unnamed(&file, &temp.path().join("out.txt")).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::Unsupported);
    }
    #[cfg(target_os = "linux")]
    drop(file);
}
