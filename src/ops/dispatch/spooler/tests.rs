//! Test service responses without starting a service.

#[cfg(windows)]
mod windows {
    use super::super::{spooler_program, start_print_spooler, start_with, SPOOLER_OVERRIDE};
    use std::{
        ffi::OsStr, io, os::windows::process::ExitStatusExt, path::Path, path::PathBuf,
        process::ExitStatus,
    };

    struct OverrideGuard;

    impl OverrideGuard {
        fn set(path: PathBuf) -> Self {
            SPOOLER_OVERRIDE.with(|slot| *slot.borrow_mut() = Some(path));
            Self
        }
    }

    impl Drop for OverrideGuard {
        fn drop(&mut self) {
            SPOOLER_OVERRIDE.with(|slot| *slot.borrow_mut() = None);
        }
    }

    #[test]
    fn requests_only_the_local_spooler_and_reports_the_service_result() {
        let program = Path::new(r"C:\Windows\System32\sc.exe");
        for (code, message) in [
            (0, "Print Spooler start requested"),
            (1056, "Print Spooler is already running"),
        ] {
            let result = start_with(program, |command| {
                assert_eq!(command.get_program(), program.as_os_str());
                assert!(command.get_args().eq(["start", "Spooler"]));
                Ok(ExitStatus::from_raw(code))
            });
            assert_eq!(result.unwrap(), message);
        }
    }

    #[test]
    fn preserves_service_and_launch_errors() {
        let program = Path::new("sc.exe");
        let denied = start_with(program, |_| Ok(ExitStatus::from_raw(5))).unwrap_err();
        assert_eq!(denied.kind(), io::ErrorKind::PermissionDenied);
        let missing = start_with(program, |_| Err(io::ErrorKind::NotFound.into())).unwrap_err();
        assert_eq!(missing.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn spooler_program_resolves_sc_exe_and_start_uses_override() {
        let resolved = spooler_program().unwrap();
        assert_eq!(resolved.file_name(), Some(OsStr::new("sc.exe")));
        assert!(resolved.parent().is_some_and(Path::is_dir));

        let temp = tempfile::tempdir().unwrap();
        let missing = temp.path().join("missing-sc.exe");
        let _guard = OverrideGuard::set(missing);
        assert_eq!(
            start_print_spooler().unwrap_err().kind(),
            io::ErrorKind::NotFound
        );
    }
}

#[test]
fn interpret_status_code_maps_known_codes_and_missing_exit_code() {
    assert_eq!(
        super::interpret_status_code(Some(0)).unwrap(),
        "Print Spooler start requested"
    );
    assert_eq!(
        super::interpret_status_code(Some(1056)).unwrap(),
        "Print Spooler is already running"
    );
    assert_eq!(
        super::interpret_status_code(Some(5))
            .unwrap_err()
            .raw_os_error(),
        Some(5)
    );
    assert!(super::interpret_status_code(None)
        .unwrap_err()
        .to_string()
        .contains("without an exit code"));
}

#[cfg(not(windows))]
#[test]
fn reports_an_unsupported_platform() {
    assert_eq!(
        super::start_print_spooler().unwrap_err().kind(),
        std::io::ErrorKind::Unsupported
    );
}
