//! Purpose: Tests for the platform wrappers, asked of the real kernel.

#[cfg(unix)]
mod unix {
    use std::io;
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    use std::os::unix::process::{CommandExt, ExitStatusExt};
    use std::process::{Child, Command};

    use tempfile::TempDir;

    use crate::platform::{check_write_access, effective_uid, is_root, kill_process_group};

    fn sleeper(own_group: bool) -> Child {
        let mut command = Command::new("sleep");
        command.arg("30");
        if own_group {
            command.process_group(0);
        }
        command.spawn().expect("spawn sleep")
    }

    #[test]
    fn effective_uid_owns_the_files_this_process_creates() {
        let dir = TempDir::new().expect("tempdir");
        let file = dir.path().join("mine");
        std::fs::write(&file, b"x").expect("write");
        let owner = std::fs::metadata(&file).expect("stat").uid();
        assert_eq!(owner, effective_uid());
    }

    #[test]
    fn is_root_means_effective_uid_zero() {
        assert_eq!(is_root(), effective_uid() == 0);
    }

    /// `/proc/self/status` lists the real, effective, saved and filesystem
    /// uids — an answer independent of the calls under test.
    #[cfg(target_os = "linux")]
    #[test]
    fn real_and_effective_uid_match_the_kernel_status() {
        let status = std::fs::read_to_string("/proc/self/status").expect("status");
        let ids: Vec<u32> = status
            .lines()
            .find_map(|line| line.strip_prefix("Uid:"))
            .expect("a Uid line")
            .split_whitespace()
            .map(|id| id.parse().expect("numeric uid"))
            .collect();
        assert_eq!(ids[..2], [crate::platform::real_uid(), effective_uid()]);
    }

    #[test]
    fn a_writable_file_passes_the_write_check() {
        let dir = TempDir::new().expect("tempdir");
        let file = dir.path().join("open.txt");
        std::fs::write(&file, b"x").expect("write");
        check_write_access(&file).expect("writable");
    }

    #[test]
    fn a_missing_file_fails_the_write_check_as_not_found() {
        let dir = TempDir::new().expect("tempdir");
        let error = check_write_access(&dir.path().join("absent")).expect_err("no file");
        assert_eq!(error.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn an_interior_nul_fails_the_write_check_as_invalid_input() {
        let path = std::path::Path::new("bad\0name");
        let error = check_write_access(path).expect_err("unrepresentable");
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    }

    /// Root's `CAP_DAC_OVERRIDE` writes past the mode bits, so the kernel's
    /// answer depends on who runs the suite; both answers are asserted.
    #[test]
    fn a_read_only_file_is_refused_unless_root() {
        let dir = TempDir::new().expect("tempdir");
        let file = dir.path().join("locked.txt");
        std::fs::write(&file, b"x").expect("write");
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o444)).expect("chmod");
        let answer = check_write_access(&file);
        if is_root() {
            answer.expect("root writes past the mode bits");
        } else {
            let error = answer.expect_err("read-only for this user");
            assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
        }
    }

    #[test]
    fn killing_a_group_leader_ends_it_with_sigkill() {
        let mut leader = sleeper(true);
        kill_process_group(&leader).expect("kill group");
        let status = leader.wait().expect("reap");
        assert_eq!(status.signal(), Some(9));
    }

    /// A child left in this process's group leads no group of its own: the
    /// call must fail rather than signal anyone, the test runner included.
    #[test]
    fn a_child_that_leads_no_group_is_not_signalled() {
        let mut child = sleeper(false);
        let error = kill_process_group(&child).expect_err("no such group");
        assert_eq!(error.raw_os_error(), Some(libc::ESRCH));
        assert!(child.try_wait().expect("poll").is_none(), "still running");
        child.kill().expect("cleanup");
        child.wait().expect("reap");
    }
}

#[cfg(not(unix))]
#[test]
fn is_root_is_never_claimed_off_unix() {
    assert!(!super::is_root());
}

/// The system drive is mounted by definition, so its bit must be set.
#[cfg(windows)]
#[test]
fn logical_drives_include_the_system_drive() {
    let drive = std::env::var("SystemDrive").unwrap_or_else(|_| String::from("C:"));
    let letter = drive
        .bytes()
        .next()
        .expect("a drive letter")
        .to_ascii_uppercase();
    let bit = u32::from(letter - b'A');
    assert_ne!(super::logical_drives() & (1 << bit), 0, "{drive} missing");
}

#[cfg(windows)]
#[test]
fn set_file_attributes_hides_and_restores_a_file() {
    use std::os::windows::fs::MetadataExt;
    use windows_sys::Win32::Storage::FileSystem::{FILE_ATTRIBUTE_HIDDEN, FILE_ATTRIBUTE_NORMAL};

    let dir = tempfile::TempDir::new().expect("tempdir");
    let file = dir.path().join("hide-me.txt");
    std::fs::write(&file, b"x").expect("write");
    let hidden = |path: &std::path::Path| {
        std::fs::metadata(path).expect("stat").file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0
    };
    super::set_file_attributes(&file, FILE_ATTRIBUTE_HIDDEN).expect("hide");
    assert!(hidden(&file));
    super::set_file_attributes(&file, FILE_ATTRIBUTE_NORMAL).expect("restore");
    assert!(!hidden(&file));
}

#[cfg(windows)]
#[test]
fn set_file_attributes_refuses_what_it_cannot_name() {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let missing = super::set_file_attributes(&dir.path().join("absent"), 0x80);
    assert_eq!(
        missing.expect_err("no file").kind(),
        std::io::ErrorKind::NotFound
    );
    let nul = super::set_file_attributes(std::path::Path::new("bad\0name"), 0x80);
    assert_eq!(
        nul.expect_err("interior NUL").kind(),
        std::io::ErrorKind::InvalidInput
    );
}
