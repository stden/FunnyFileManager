use super::{fixture::Server, RemoteError, RemoteFs, UnixMode, WriteMode};
use std::io::{self, Read, Write};

pub(super) fn check(remote: &mut dyn RemoteFs, server: &Server) {
    let missing = server.path("missing");
    assert!(remote.list(&missing).is_err());
    assert!(remote.download(&missing, &mut Vec::new()).is_err());
    assert!(remote
        .upload(
            &server.path("missing/file"),
            &mut b"data".as_slice(),
            WriteMode::CreateNew
        )
        .is_err());
    assert!(remote.chmod(&missing, UnixMode::from_mode(0o600)).is_err());
    assert!(remote.remove_file(&missing).is_err());
    assert!(remote.rmdir(&missing).is_err());
    assert!(remote
        .rename(&missing, &server.path("renamed-missing"))
        .is_err());
    assert!(remote
        .replace_file(&missing, &server.path("replacement-missing"))
        .is_err());
    assert!(matches!(
        remote.download(&server.path(""), &mut Vec::new()),
        Err(RemoteError::Unsupported("reading special files"))
    ));
    transfer_io_errors(remote, server);
    invalid_names(remote, server);
}

struct FailedIo;

impl Read for FailedIo {
    fn read(&mut self, _bytes: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::other("fixture input failed"))
    }
}

impl Write for FailedIo {
    fn write(&mut self, _bytes: &[u8]) -> io::Result<usize> {
        Err(io::Error::other("fixture output failed"))
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn transfer_io_errors(remote: &mut dyn RemoteFs, server: &Server) {
    let path = server.path("io-failure");
    assert!(remote
        .upload(&path, &mut FailedIo, WriteMode::CreateNew)
        .is_err());
    remote
        .upload(&path, &mut b"contents".as_slice(), WriteMode::Overwrite)
        .expect("seed input");
    assert!(remote.download(&path, &mut FailedIo).is_err());
    let too_long = server.path(&"x".repeat(300));
    assert!(matches!(
        remote.rename(&path, &too_long),
        Err(RemoteError::Failed(_))
    ));
    assert!(remote.stat(&path).is_ok(), "failed rename keeps source");
    remote.remove_file(&path).expect("cleanup");
}

fn invalid_names(remote: &mut dyn RemoteFs, server: &Server) {
    use std::os::unix::ffi::OsStringExt;
    let directory = server.local("invalid-names");
    std::fs::create_dir(&directory).expect("fixture directory");
    std::fs::write(
        directory.join(std::ffi::OsString::from_vec(vec![0xff])),
        b"data",
    )
    .expect("non-UTF-8 name");
    assert!(matches!(
        remote.list(&server.path("invalid-names")),
        Err(RemoteError::Failed("SFTP returned a non-UTF-8 file name"))
    ));
}
