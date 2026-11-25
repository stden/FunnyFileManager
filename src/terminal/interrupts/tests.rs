//! Purpose: Verify cached signal-handler installation failures map cleanly for callers.

use super::unix::borrow_flags;
use std::io;

#[test]
fn borrow_flags_maps_cached_installation_errors() {
    let err = borrow_flags(&Err(io::Error::other("signal blocked")))
        .err()
        .unwrap();
    assert_eq!(err.kind(), io::ErrorKind::Other);
    assert!(err.to_string().contains("signal blocked"));
}
