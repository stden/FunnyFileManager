use super::*;
use crate::models::DirEntry;
use crate::remote::RemoteFs;

struct HostileNames;
impl RemoteFs for HostileNames {
    fn list(&mut self, _path: &RemotePath) -> Result<Vec<DirEntry>, RemoteError> {
        Ok(vec![DirEntry {
            name: "../outside".to_owned(),
            ..DirEntry::default()
        }])
    }
}

#[test]
fn malformed_remote_children_are_rejected_before_a_destination_is_created() {
    let source = FileLocation::Remote {
        link: RemoteLink::new(Box::new(HostileNames)),
        path: RemotePath::root(),
    };
    assert_eq!(
        children(&source),
        Err(RemoteError::InvalidInput("invalid remote file name"))
    );
}

#[cfg(unix)]
#[test]
fn non_utf8_local_children_are_rejected_without_copying_or_deleting_them() {
    use std::os::unix::ffi::OsStringExt;
    let temp = tempfile::TempDir::new().expect("temporary directory");
    let child = temp.path().join(std::ffi::OsString::from_vec(vec![0xff]));
    fs::write(&child, b"original").expect("non-UTF-8 filename");
    assert_eq!(
        children(&FileLocation::Local(temp.path().to_path_buf())),
        Err(RemoteError::InvalidInput(
            "cannot transfer a non-UTF-8 name"
        ))
    );
    assert_eq!(fs::read(child).expect("source remains"), b"original");
}

#[test]
fn metadata_and_directory_creation_errors_do_not_change_the_source() {
    let temp = tempfile::TempDir::new().expect("temporary directory");
    let missing = FileLocation::Local(temp.path().join("missing"));
    assert!(metadata(&missing).is_err());
    assert!(mkdir(&FileLocation::Local(temp.path().join("missing/nested"))).is_err());
    let source = temp.path().join("file");
    fs::write(&source, b"original").expect("source");
    assert!(copy_file(
        &FileLocation::Local(source.clone()),
        &FileLocation::Local(temp.path().join("missing/file"))
    )
    .is_err());
    assert_eq!(fs::read(source).expect("source remains"), b"original");
}

#[test]
fn failed_local_stage_flush_removes_created_destination_and_non_utf8_names_error() {
    let temp = tempfile::TempDir::new().expect("tempdir");
    let mut staged = tempfile::tempfile().expect("staged");
    let dest = temp.path().join("partial.bin");
    file::write_created_local(&dest, &mut staged, |_, _| {
        Err(std::io::Error::other("disk full"))
    })
    .expect_err("flush failure");
    assert!(
        !dest.exists(),
        "exclusively created file must be cleaned up"
    );

    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStringExt;
        assert_eq!(
            location::utf8_name(std::ffi::OsString::from_wide(&[0xD800])),
            Err(RemoteError::InvalidInput(
                "cannot transfer a non-UTF-8 name"
            ))
        );
    }
}
