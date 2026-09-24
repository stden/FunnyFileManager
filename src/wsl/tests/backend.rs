//! Purpose: Tests for opening a root and the system backend off Windows.

use super::*;

#[test]
fn opening_root_never_starts_a_process() {
    let name = distribution("Ubuntu");
    assert_eq!(
        execute_with(&WslOperation::Open(name.clone()), |_, _| panic!(
            "unexpected process"
        ))
        .unwrap(),
        WslOutput::Root(name.root_path())
    );
}

#[cfg(windows)]
#[test]
fn system_backend_can_resolve_a_root_without_executing_wsl() {
    let name = distribution("Debian");
    assert_eq!(
        SystemWslBackend
            .execute(&WslOperation::Open(name.clone()))
            .unwrap(),
        WslOutput::Root(name.root_path())
    );
}

#[cfg(not(windows))]
#[test]
fn system_backend_reports_unavailable_on_other_platforms() {
    for operation in [
        WslOperation::Refresh,
        WslOperation::Update,
        WslOperation::Open(distribution("Ubuntu")),
        WslOperation::UpdatePackages(distribution("Ubuntu")),
    ] {
        assert_eq!(
            SystemWslBackend.execute(&operation).unwrap_err(),
            "WSL is only available on Windows"
        );
    }
}
