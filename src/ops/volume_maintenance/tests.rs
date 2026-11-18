//! Volume maintenance uses a simulated launcher. No test performs maintenance.

use super::*;
use std::{ffi::OsString, path::PathBuf};

fn target() -> VolumeTarget {
    VolumeTarget {
        mount: PathBuf::from(r"C:\mounted disks\data\"),
        id: OsString::from(r"\\?\Volume{123}\"),
    }
}

#[test]
fn commands_target_exactly_one_volume_with_the_requested_operation() {
    let target = target();
    let program = Path::new(r"C:\Windows\System32\defrag.exe");
    for (operation, flag) in [
        (VolumeOperation::Trim, "/L"),
        (VolumeOperation::Defragment, "/D"),
    ] {
        let command = command(program, operation, &target.mount);
        assert_eq!(command.get_program(), program);
        assert_eq!(
            command.get_args().collect::<Vec<_>>(),
            [target.mount.as_os_str(), flag.as_ref(), "/U".as_ref()]
        );
    }
}

#[test]
fn a_replaced_volume_cannot_run_under_the_old_confirmation() {
    let confirmed = target();
    let mut current = confirmed.clone();
    current.id = OsString::from(r"\\?\Volume{456}\");
    assert!(verify_target(&confirmed, &current).is_err());
    current.id = confirmed.id.clone();
    current.mount = PathBuf::from(r"C:\");
    assert!(verify_target(&confirmed, &current).is_err());
    assert!(verify_target(&confirmed, &confirmed).is_ok());
}

#[cfg(windows)]
fn fake_output(code: u32) -> std::process::Output {
    use std::{
        os::windows::process::ExitStatusExt as _,
        process::{ExitStatus, Output},
    };
    Output {
        status: ExitStatus::from_raw(code),
        stdout: "Диск C: 100%"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect(),
        stderr: "résultat".as_bytes().to_vec(),
    }
}

#[cfg(windows)]
#[test]
fn a_fake_native_process_reports_its_localized_output_and_exit_code() {
    let target = target();
    for code in [0, 5] {
        let result = windows::execute(
            VolumeOperation::Trim,
            &target,
            &target,
            Path::new("unused.exe"),
            |_| Ok(fake_output(code)),
        );
        if code == 0 {
            assert_eq!(result.unwrap(), "Диск C: 100%\nrésultat");
        } else {
            assert!(result
                .unwrap_err()
                .to_string()
                .contains("Диск C: 100%\nrésultat"));
        }
    }
}

#[test]
fn the_full_report_is_preserved_on_success_and_failure() {
    assert_eq!(
        report(true, "0", "  done\r\n", "note").unwrap(),
        "done\nnote"
    );
    let failed = report(false, "5", "processed 0", "Access denied")
        .unwrap_err()
        .to_string();
    for clue in ["5", "processed 0", "Access denied"] {
        assert!(failed.contains(clue));
    }
    assert_eq!(
        report(false, "5", "", "").unwrap_err().to_string(),
        "defrag.exe failed (5)"
    );
}

#[cfg(windows)]
#[test]
fn invalid_drive_paths_are_rejected_before_lookup() {
    for path in [
        "C:relative",
        r"\relative",
        r"\\host\share",
        r"\\?\UNC\host\share",
        r"\\.\C:\",
    ] {
        assert!(resolve(Path::new(path)).is_err(), "{path}");
    }
    let dir = tempfile::tempdir().unwrap();
    assert!(resolve(&dir.path().join("missing")).is_err());
    let resolved = resolve(dir.path()).unwrap();
    assert!(resolved.mount.is_absolute());
    assert!(!resolved.id.is_empty());
    assert_eq!(resolve(&resolved.mount).unwrap(), resolved);
}

#[cfg(windows)]
#[test]
fn native_utility_receives_a_dos_mount_path_without_losing_utf16_units() {
    use std::os::windows::ffi::OsStringExt;
    let odd = OsString::from_wide(&[0xd800]);
    let mut plain = OsString::from(r"C:\mounted disks\");
    plain.push(odd);
    plain.push(r"\");
    let mut extended = OsString::from(r"\\?\");
    extended.push(&plain);
    let target = VolumeTarget {
        mount: extended.into(),
        ..target()
    };
    let error = windows::execute(
        VolumeOperation::Trim,
        &target,
        &target,
        Path::new("unused.exe"),
        |cmd| {
            assert_eq!(cmd.get_args().next(), Some(plain.as_os_str()));
            Err(io::Error::other("simulated process"))
        },
    )
    .unwrap_err();
    assert_eq!(error.to_string(), "simulated process");
}

#[cfg(windows)]
#[test]
fn the_native_program_comes_from_the_windows_system_directory() {
    let system = crate::platform::system_directory().unwrap();
    assert!(system.is_absolute());
    assert!(system.is_dir());
}

#[cfg(not(windows))]
#[test]
fn volume_maintenance_is_unsupported_on_other_platforms() {
    assert_eq!(
        resolve(Path::new("/")).unwrap_err().kind(),
        io::ErrorKind::Unsupported
    );
    assert_eq!(
        run(VolumeOperation::Trim, &target()).unwrap_err().kind(),
        io::ErrorKind::Unsupported
    );
}
