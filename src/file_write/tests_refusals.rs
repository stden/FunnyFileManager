//! Writable-file errors explain which permission boundary refused the write.

#[cfg(unix)]
#[test]
fn read_only_filesystems_and_permissions_have_distinct_messages() {
    use super::WriteRefusal;

    assert_eq!(
        WriteRefusal::PermissionDenied.to_string(),
        "permission denied"
    );
    assert_eq!(
        WriteRefusal::ReadOnlyFileSystem.to_string(),
        "read-only file system"
    );
    assert_eq!(
        super::refusal_for_errno(Some(libc::EROFS)),
        Some(WriteRefusal::ReadOnlyFileSystem)
    );
    assert_eq!(
        super::refusal_for_errno(Some(libc::EACCES)),
        Some(WriteRefusal::PermissionDenied)
    );
    assert_eq!(super::refusal_for_errno(Some(libc::ENOENT)), None);
    assert_eq!(super::refusal_for_errno(None), None);
}

#[test]
fn ensure_writable_and_refuse_unwritable_reject_readonly_file() {
    let dir = tempfile::TempDir::new().unwrap();
    let locked = dir.path().join("locked.yaml");
    std::fs::write(&locked, "v: 1\n").unwrap();
    let mut perms = std::fs::metadata(&locked).unwrap().permissions();
    perms.set_readonly(true);
    std::fs::set_permissions(&locked, perms).unwrap();

    let refusal = super::refuse_unwritable(&locked).unwrap_err();
    assert_eq!(refusal.kind(), std::io::ErrorKind::PermissionDenied);
    assert_ne!(refusal.to_string(), "");
    let err = super::ensure_writable(&locked).unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::PermissionDenied);
}
