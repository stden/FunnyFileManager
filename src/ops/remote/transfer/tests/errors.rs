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

struct NoChmodTarget;
impl RemoteFs for NoChmodTarget {
    fn list(&mut self, _dir: &RemotePath) -> Result<Vec<DirEntry>, RemoteError> {
        Ok(vec![DirEntry {
            name: "mode.txt".to_owned(),
            ..DirEntry::default()
        }])
    }

    fn download(
        &mut self,
        _path: &RemotePath,
        output: &mut dyn std::io::Write,
    ) -> Result<u64, RemoteError> {
        output.write_all(b"ok")?;
        Ok(2)
    }

    fn upload(
        &mut self,
        _path: &RemotePath,
        input: &mut dyn std::io::Read,
        _mode: crate::remote::WriteMode,
    ) -> Result<u64, RemoteError> {
        let mut sink = Vec::new();
        input.read_to_end(&mut sink)?;
        Ok(u64::try_from(sink.len()).unwrap_or(u64::MAX))
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
    let local_copy = temp.path().join("local_copy");
    copy_file(
        &FileLocation::Local(source.clone()),
        &FileLocation::Local(local_copy.clone()),
    )
    .expect("local copy_file");
    assert_eq!(fs::read(local_copy).expect("local copy"), b"original");
    assert_eq!(fs::read(source).expect("source remains"), b"original");
}

#[test]
fn failed_local_stage_flush_removes_created_destination_and_non_utf8_names_error() {
    let temp = tempfile::TempDir::new().expect("tempdir");
    let dest = temp.path().join("partial.bin");
    file::write_created_local(&dest, |_| Err(std::io::Error::other("disk full").into()))
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

/// A download claims its name exclusively: an existing local file is refused
/// before a byte is written, and the refusal does not remove it.
#[test]
fn a_download_never_removes_an_existing_destination() {
    let temp = tempfile::TempDir::new().expect("temporary directory");
    let path = temp.path().join("existing.txt");
    fs::write(&path, b"original").expect("existing destination");
    let written = file::write_created_local(&path, |_| panic!("must not write"));
    assert!(written.is_err());
    assert_eq!(
        fs::read(path).expect("existing destination survives"),
        b"original"
    );
}

#[test]
fn remote_chmod_and_missing_local_mode_errors_propagate() {
    let temp = tempfile::TempDir::new().expect("tempdir");
    #[cfg(unix)]
    assert!(location::set_local_mode(
        &temp.path().join("missing"),
        crate::remote::UnixMode::from_mode(0o644),
    )
    .is_err());

    let src_host = remote();
    let src_file = src_host.child("mode.txt").expect("src child");
    let local_src = temp.path().join("seed.txt");
    fs::write(&local_src, b"ok").expect("seed");
    transfer(&FileLocation::Local(local_src), &src_file, false).expect("seed upload");
    chmod(&src_file, crate::remote::UnixMode::from_mode(0o644)).expect("set mode");
    let dst_file = FileLocation::Remote {
        link: RemoteLink::new(Box::new(NoChmodTarget)),
        path: RemotePath::root(),
    }
    .child("mode.txt")
    .expect("dst child");
    assert_eq!(
        transfer(&src_file, &dst_file, false),
        Err(RemoteError::Unsupported("chmod"))
    );
}

#[test]
fn remote_download_without_unix_mode_skips_chmod() {
    let temp = tempfile::TempDir::new().expect("tempdir");
    let src_file = FileLocation::Remote {
        link: RemoteLink::new(Box::new(NoChmodTarget)),
        path: RemotePath::root(),
    }
    .child("mode.txt")
    .expect("src child");
    let dst_path = temp.path().join("modeless.txt");
    transfer(&src_file, &FileLocation::Local(dst_path.clone()), false).expect("download");
    assert_eq!(fs::read(&dst_path).expect("read downloaded"), b"ok");
}
