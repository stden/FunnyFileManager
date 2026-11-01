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
}
