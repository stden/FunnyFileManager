//! Purpose: Error adapters preserve the original filesystem failure and its location.

use super::*;

#[test]
fn a_result_adapter_keeps_the_filesystem_error_kind_and_message() {
    let root = tempfile::TempDir::new().unwrap();
    let missing = root.path().join("missing");
    let error = std::fs::metadata(&missing).unwrap_err();
    let message = error.to_string();
    let wrapped = Err::<(), _>(error)
        .map_err(FailedAt::for_path(&missing))
        .unwrap_err();
    assert_eq!(wrapped.kind(), io::ErrorKind::NotFound);
    assert_eq!(wrapped.to_string(), message);
    assert_eq!(FailedAt::of(&wrapped).unwrap().path(), missing);
}

#[test]
fn an_outer_adapter_retains_the_inner_failure_location() {
    let inner = Path::new("source/locked");
    let outer = Path::new("destination");
    let original = FailedAt::wrap(inner, io::Error::from(io::ErrorKind::PermissionDenied));
    let error = FailedAt::for_path(outer)(original);
    assert_eq!(FailedAt::of(&error).unwrap().path(), inner);
    assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
}
