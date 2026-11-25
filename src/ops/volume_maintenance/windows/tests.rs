//! Purpose: Unit tests for Windows volume target resolution, maintenance run wiring, and OEM output decoding.

use std::ffi::OsString;
use std::io;
use std::path::PathBuf;

use super::{decode, resolve, resolve_with, run, select_program, PROGRAM_OVERRIDE};
use crate::diskfree::DriveKind;
use crate::models::{VolumeOperation, VolumeTarget};
use crate::platform;

struct ProgramGuard;

impl ProgramGuard {
    fn set(path: PathBuf) -> Self {
        PROGRAM_OVERRIDE.with(|slot| *slot.borrow_mut() = Some(path));
        Self
    }
}

impl Drop for ProgramGuard {
    fn drop(&mut self) {
        PROGRAM_OVERRIDE.with(|slot| *slot.borrow_mut() = None);
    }
}

#[test]
fn resolve_with_rejects_non_local_and_non_fixed_mounts() {
    let dir = tempfile::tempdir().unwrap();
    let unc = resolve_with(
        dir.path(),
        |_| Ok(PathBuf::from(r"\\host\share\")),
        |_| DriveKind::Fixed,
        |_| Ok(OsString::new()),
    )
    .unwrap_err();
    assert!(
        unc.to_string().contains("local fixed or removable"),
        "{unc}"
    );

    let network = resolve_with(
        dir.path(),
        platform::volume_mount_point,
        |_| DriveKind::Network,
        |_| Ok(OsString::new()),
    )
    .unwrap_err();
    assert!(
        network.to_string().contains("local fixed or removable"),
        "{network}"
    );
}

#[test]
fn run_validates_target_before_launch_and_uses_program_override() {
    let dir = tempfile::tempdir().unwrap();
    let real = resolve(dir.path()).unwrap();
    let stale = VolumeTarget {
        id: OsString::from(r"\\?\Volume{00000000-0000-0000-0000-000000000001}\"),
        ..real.clone()
    };
    let changed = run(VolumeOperation::Trim, &stale).unwrap_err();
    assert!(changed.to_string().contains("changed"), "{changed}");

    assert!(select_program(PathBuf::from("defrag.exe")).ends_with("defrag.exe"));
    let _guard = ProgramGuard::set(dir.path().join("missing-defrag.exe"));
    assert_eq!(
        run(VolumeOperation::Trim, &real).unwrap_err().kind(),
        io::ErrorKind::NotFound
    );
}

#[test]
fn decode_falls_back_to_oem_for_non_utf8_single_byte_output() {
    let bytes = [0x80, 0x81, 0x82];
    assert_eq!(decode(&bytes), platform::decode_oem(&bytes));
}
