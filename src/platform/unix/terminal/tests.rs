//! Purpose: Verify missing PTY file descriptors return an error rather than failing later.

use super::nonblocking_fd;
use std::io;

#[test]
fn missing_terminal_fd_returns_other_error() {
    let error = nonblocking_fd(None).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::Other);
}
