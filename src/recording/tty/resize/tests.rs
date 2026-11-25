//! Purpose: Verify resize-pipe initialization failures are cloned cleanly for callers.

use super::borrow_reader;
use std::io;
use std::os::unix::net::UnixStream;

#[test]
fn borrow_reader_maps_cached_errors_and_keeps_streams() {
    let err = borrow_reader(&Err(io::Error::other("pipe failed"))).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::Other);
    assert!(err.to_string().contains("pipe failed"));

    let (stream, _peer) = UnixStream::pair().unwrap();
    assert!(borrow_reader(&Ok(stream)).is_ok());
}
