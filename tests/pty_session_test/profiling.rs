//! Profiling metadata follows the real binary through the hermetic PTY fixture.

use super::*;

#[test]
fn inherited_profile_reaches_pty_child() {
    if std::env::var_os("FFM_PTY_PROFILE_CHILD").is_some() {
        check_child_environment();
        return;
    }
    let sandbox = TempDir::new().expect("profile directory");
    let profile = std::env::var_os("LLVM_PROFILE_FILE")
        .unwrap_or_else(|| sandbox.path().join("child-%m-%p.profraw").into_os_string());
    let status = Command::new(std::env::current_exe().expect("test executable"))
        .args(["--exact", "profiling::inherited_profile_reaches_pty_child"])
        .env("FFM_PTY_PROFILE_CHILD", "1")
        .env("LLVM_PROFILE_FILE", profile)
        .status()
        .expect("isolated profile test");
    assert!(status.success());
}

#[expect(
    clippy::expect_used,
    reason = "test fixture: missing process metadata or an exit status must fail the profile regression"
)]
fn check_child_environment() {
    let mut ffm = Ffm::start();
    let variables =
        fs::read(format!("/proc/{}/environ", ffm.process.id())).expect("child environment");
    let expected = [
        b"LLVM_PROFILE_FILE=".as_slice(),
        std::env::var_os("LLVM_PROFILE_FILE")
            .expect("profile path")
            .as_bytes(),
    ]
    .concat();
    let inherited = variables
        .split(|byte| *byte == 0)
        .any(|entry| entry == expected);
    ffm.type_keys(b"\x03");
    assert!(ffm.wait_for_exit().expect("ffm exited").success());
    assert!(inherited, "PTY fixture discarded the coverage profile path");
}
