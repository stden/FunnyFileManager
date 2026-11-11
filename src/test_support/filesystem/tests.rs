//! Capability checks must not hide unrelated fixture failures.

use super::*;

#[test]
fn a_created_fixture_is_supported() {
    assert!(non_unicode_name(Ok(())).unwrap());
}

#[test]
fn an_encoding_rejection_is_unsupported() {
    assert!(!non_unicode_name(Err(io::Error::from_raw_os_error(libc::EILSEQ))).unwrap());
}

#[test]
fn unrelated_errors_still_fail_setup() {
    for code in [libc::EACCES, libc::ENOENT, libc::ENOSPC] {
        let error = non_unicode_name(Err(io::Error::from_raw_os_error(code))).unwrap_err();
        assert_eq!(error.raw_os_error(), Some(code));
    }
}
