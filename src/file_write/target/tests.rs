//! Purpose: Unit tests for `DanglingLink` formatting and `io::Error` conversion.

use super::DanglingLink;
use std::{io, path::PathBuf};

#[test]
fn dangling_link_formats_target_and_converts_to_invalid_input() {
    let err = DanglingLink(PathBuf::from("broken.yaml")).into_io_error();
    assert_eq!(err.kind(), io::ErrorKind::NotFound);
    assert!(err.to_string().contains("broken.yaml"));
}
