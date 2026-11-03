use super::{available_bytes, capacity, Capacity, DriveKind};
use std::path::Path;

/// The root filesystem is always mounted, so this is the one call that must
/// succeed on any machine the suite runs on.
#[test]
fn the_root_filesystem_reports_a_size() {
    let space = capacity(Path::new("/")).expect("the root filesystem is mounted");
    assert!(space.total > 0, "a mounted filesystem has a size");
    assert!(
        space.available <= space.total,
        "free {} exceeds total {}",
        space.available,
        space.total
    );
}

/// `available_bytes` is the footer's view of the same call; a second platform
/// query could disagree with the dialog's.
#[test]
fn the_footer_reads_the_free_half_of_the_same_answer() {
    let path = Path::new("/");
    let space = capacity(path).expect("the root filesystem is mounted");
    assert!(
        available_bytes(path).is_some(),
        "the footer uses the same call as the totals"
    );
    assert!(space.total >= space.available);
}

/// A path that cannot be resolved is `None`, not zero: an unmounted drive and
/// a full one are different facts.
#[test]
fn an_impossible_path_reports_nothing_rather_than_zero() {
    assert_eq!(capacity(Path::new("/no/such/mount/point/here")), None);
    assert_eq!(
        available_bytes(Path::new("/no/such/mount/point/here")),
        None
    );
}

/// An interior NUL cannot become a C string, and must not panic on the way.
#[test]
fn a_path_with_an_interior_nul_is_refused() {
    assert_eq!(capacity(Path::new("bad\0path")), None);
}

#[test]
fn every_kind_names_itself_except_the_unknown_one() {
    assert_eq!(DriveKind::Fixed.label(), "fixed");
    assert_eq!(DriveKind::Removable.label(), "removable");
    assert_eq!(DriveKind::Network.label(), "network");
    assert_eq!(DriveKind::CdRom.label(), "CD-ROM");
    assert_eq!(DriveKind::RamDisk.label(), "RAM disk");
    assert_eq!(DriveKind::Unknown.label(), "");
    assert_eq!(DriveKind::default(), DriveKind::Unknown);
}

/// Unix draws no distinction between mounts, so it says so consistently rather
/// than guessing from the path.
#[cfg(not(windows))]
#[test]
fn unix_calls_every_mount_fixed() {
    assert_eq!(super::drive_kind(Path::new("/")), DriveKind::Fixed);
    assert_eq!(
        super::drive_kind(Path::new("/no/such/place")),
        DriveKind::Fixed
    );
}

#[cfg(windows)]
#[test]
fn windows_classifies_every_win32_drive_type_and_queries_live_paths() {
    use windows_sys::Win32::System::WindowsProgramming::{
        DRIVE_CDROM, DRIVE_FIXED, DRIVE_NO_ROOT_DIR, DRIVE_RAMDISK, DRIVE_REMOTE, DRIVE_REMOVABLE,
        DRIVE_UNKNOWN,
    };

    for (raw, expected) in [
        (DRIVE_FIXED, DriveKind::Fixed),
        (DRIVE_REMOVABLE, DriveKind::Removable),
        (DRIVE_REMOTE, DriveKind::Network),
        (DRIVE_CDROM, DriveKind::CdRom),
        (DRIVE_RAMDISK, DriveKind::RamDisk),
        (DRIVE_UNKNOWN, DriveKind::Unknown),
        (DRIVE_NO_ROOT_DIR, DriveKind::Unknown),
    ] {
        assert_eq!(super::classify_drive_type(raw), expected);
    }
    assert_ne!(
        super::drive_kind(Path::new(r"C:\")),
        DriveKind::CdRom,
        "system volume query reaches GetDriveTypeW"
    );
    assert_eq!(
        super::drive_kind(Path::new("bad\0path")),
        DriveKind::Unknown
    );
}

#[test]
fn capacity_is_a_plain_value() {
    let space = Capacity {
        total: 10,
        available: 4,
    };
    assert_eq!(
        space,
        Capacity {
            total: 10,
            available: 4
        }
    );
    assert_ne!(
        space,
        Capacity {
            total: 10,
            available: 5
        }
    );
    assert!(format!("{space:?}").contains("available"));
}
