//! Purpose: Share real permission-denial fixtures across Rust packages, retaining LLVM profiles.
#![allow(
    clippy::unwrap_used,
    clippy::print_stdout,
    reason = "shared test-only permission fixture fails fast on setup errors and marks its child completion"
)]

use std::{
    fs,
    os::unix::{
        fs::{MetadataExt, PermissionsExt},
        process::CommandExt,
    },
    path::Path,
    process::Command,
};

pub fn run(check: impl FnOnce()) {
    if std::env::var("FFM_PERMISSION_TEST_CHILD").as_deref() == Ok("1") {
        check();
        println!("permission fixture completed");
        return;
    }
    let staging = tempfile::Builder::new()
        .prefix("ffm-permission-")
        .tempdir_in("/var/tmp")
        .unwrap();
    fs::set_permissions(staging.path(), fs::Permissions::from_mode(0o777)).unwrap();
    let profile = std::env::var_os("LLVM_PROFILE_FILE").map(std::path::PathBuf::from);
    let output = child(staging.path(), profile.as_deref()).output().unwrap();
    if let Some(directory) = profile.as_ref().and_then(|path| path.parent()) {
        forward_profiles(staging.path(), directory, &output.stderr);
    }
    assert!(
        output.status.success(),
        "permission test failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("permission fixture completed"));
}

fn child(staging: &Path, profile: Option<&Path>) -> Command {
    let thread = std::thread::current();
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", thread.name().unwrap(), "--nocapture"])
        .env("FFM_PERMISSION_TEST_CHILD", "1")
        .env("TMPDIR", staging)
        .env("TMP", staging)
        .env("TEMP", staging);
    if let Some(profile) = profile {
        command.env(
            "LLVM_PROFILE_FILE",
            staging.join(profile.file_name().unwrap()),
        );
    }
    if fs::metadata(staging).unwrap().uid() == 0 {
        command.gid(65534).uid(65534);
    }
    command
}

fn forward_profiles(staging: &Path, directory: &Path, stderr: &[u8]) {
    let mut copied = 0;
    for entry in fs::read_dir(staging).unwrap().flatten() {
        if entry
            .path()
            .extension()
            .is_some_and(|extension| extension == "profraw")
        {
            fs::copy(entry.path(), directory.join(entry.file_name())).unwrap();
            copied += 1;
        }
    }
    assert!(
        copied > 0,
        "permission child produced no profiles:\n{}",
        String::from_utf8_lossy(stderr)
    );
}
