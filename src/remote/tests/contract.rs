//! A minimal transport inherits safe, explicit failure defaults.

use crate::models::{DirEntry, EntryKind, FileSize, RemotePath};
use crate::remote::{RemoteError, RemoteFs, UnixMode, WriteMode};
use std::io::{self, ErrorKind};

struct Listing {
    fail: bool,
}

impl RemoteFs for Listing {
    fn list(&mut self, path: &RemotePath) -> Result<Vec<DirEntry>, RemoteError> {
        if self.fail {
            return Err(RemoteError::Failed("listing unavailable"));
        }
        assert_eq!(path.as_str(), "/dir");
        Ok(vec![DirEntry {
            name: "file".to_owned(),
            kind: EntryKind::File,
            size: FileSize::from_bytes(42),
            ..DirEntry::default()
        }])
    }
}

fn path(text: &str) -> RemotePath {
    RemotePath::parse(text).expect("valid path")
}

#[test]
fn default_stat_uses_parent_listing_and_reports_absent_entries() {
    let mut fs = Listing { fail: false };
    let found = fs.stat(&path("/dir/file")).expect("listed file");
    assert_eq!(found.kind, EntryKind::File);
    assert_eq!(found.size, FileSize::from_bytes(42));
    assert_eq!(
        fs.stat(&path("/dir/missing")),
        Err(RemoteError::NotFound(path("/dir/missing")))
    );
    assert_eq!(
        fs.stat(&RemotePath::root()),
        Err(RemoteError::Unsupported("stat root"))
    );
    fs.fail = true;
    assert_eq!(
        fs.stat(&path("/dir/file")),
        Err(RemoteError::Failed("listing unavailable"))
    );
}

#[test]
fn a_read_only_transport_rejects_mutating_and_streaming_operations() {
    let mut fs = Listing { fail: false };
    let (source, target) = (path("/dir/file"), path("/dir/other"));
    assert_eq!(
        fs.download(&source, &mut Vec::new()),
        Err(RemoteError::Unsupported("download"))
    );
    assert_eq!(
        fs.upload(&source, &mut b"data".as_slice(), WriteMode::CreateNew),
        Err(RemoteError::Unsupported("upload"))
    );
    assert_eq!(fs.mkdir(&target), Err(RemoteError::Unsupported("mkdir")));
    assert_eq!(
        fs.remove_file(&source),
        Err(RemoteError::Unsupported("remove file"))
    );
    assert_eq!(
        fs.rmdir(&source),
        Err(RemoteError::Unsupported("remove directory"))
    );
}

#[test]
fn a_read_only_transport_refuses_renames_and_permission_changes() {
    let mut fs = Listing { fail: false };
    let (source, target) = (path("/dir/file"), path("/dir/other"));
    assert_eq!(
        fs.rename(&source, &target),
        Err(RemoteError::Unsupported("rename"))
    );
    assert_eq!(
        fs.chmod(&source, UnixMode::from_mode(0o640)),
        Err(RemoteError::Unsupported("chmod"))
    );
    assert_eq!(
        fs.replace_file(&source, &target),
        Err(RemoteError::Unsupported("replacing a file by rename"))
    );
}

#[test]
fn io_errors_and_display_messages_never_forward_sensitive_error_text() {
    for (kind, expected) in [
        (
            ErrorKind::PermissionDenied,
            "remote operation: permission denied",
        ),
        (ErrorKind::TimedOut, "remote operation timed out"),
        (ErrorKind::WouldBlock, "remote operation timed out"),
        (ErrorKind::UnexpectedEof, "remote connection closed"),
        (ErrorKind::ConnectionReset, "remote connection closed"),
        (ErrorKind::Other, "remote file transfer failed"),
    ] {
        let error = RemoteError::from(io::Error::new(kind, "private details"));
        assert_eq!(error.to_string(), expected);
    }
    assert_eq!(
        RemoteError::AuthenticationRequired.to_string(),
        "enter the remote username and password"
    );
    assert_eq!(
        RemoteError::Conflict.to_string(),
        "the remote file changed since it was opened"
    );
    assert_eq!(
        RemoteError::HostKey("sha256:fingerprint".into()).to_string(),
        "SSH host key is not trusted: sha256:fingerprint"
    );
}
