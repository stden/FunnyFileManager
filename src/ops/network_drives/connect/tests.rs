//! Purpose: Direct `mount_one` error propagation on missing or invalid mount targets.

use crate::models::network_drive::{MountPoint, NetworkAddress, NetworkDrive, NetworkProtocol};

#[test]
fn mount_one_reports_failure_for_nonexistent_mount_point() {
    let temp = tempfile::TempDir::new().expect("temp");
    let target = if cfg!(windows) {
        "/mnt/not-a-drive-letter".to_owned()
    } else {
        temp.path().join("missing-mount").display().to_string()
    };
    let drive = NetworkDrive {
        path: MountPoint::try_from(target).expect("mount point"),
        remote: NetworkAddress::try_from("//127.0.0.1/missing".to_owned()).expect("remote"),
        protocol: NetworkProtocol::Smb,
    };

    assert!(super::mount_one(&drive).is_err());
}
