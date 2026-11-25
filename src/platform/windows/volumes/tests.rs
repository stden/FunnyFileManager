//! Purpose: Unit tests for Win32 volume query failures, system directory limits, and OEM decoding fallbacks.

use std::path::Path;

use super::{decode_oem_with, system_directory_with, volume_id, PATH_CAPACITY};

#[test]
fn volume_id_reports_os_error_for_missing_mount_point() {
    let temp = tempfile::tempdir().unwrap();
    let missing = temp.path().join("missing\\");
    assert!(volume_id(&missing).is_err());
    assert!(volume_id(Path::new(
        r"\\?\Volume{00000000-0000-0000-0000-000000000000}\"
    ))
    .is_err());
}

#[test]
fn system_directory_with_handles_zero_and_overlong_lengths() {
    assert!(system_directory_with(|_| 0).is_err());
    let overlong = system_directory_with(|_| u32::from(PATH_CAPACITY)).unwrap_err();
    assert!(overlong.to_string().contains("path limit"), "{overlong}");
}

#[test]
fn decode_oem_with_falls_back_to_utf8_lossy_on_overflow_or_zero() {
    assert_eq!(decode_oem_with(b"abc", usize::MAX, |_, _, _| 0), "abc");
    assert_eq!(decode_oem_with(b"abc", 3, |_, _, _| 0), "abc");
}
