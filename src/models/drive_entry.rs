//! Purpose: `DriveEntry` and the drive discovery behind the Alt+F1 / Alt+F2 menus.

use std::path::PathBuf;

use super::App;

#[cfg(windows)]
unsafe extern "system" {
    fn GetLogicalDrives() -> u32;
}

impl App {
    /// Get available drives (Windows) or mount points (Unix)
    #[cfg(windows)]
    pub fn get_drives() -> Vec<PathBuf> {
        let mask = unsafe { GetLogicalDrives() };
        windows_drive_paths_from_mask(mask)
    }

    #[cfg(not(windows))]
    pub fn get_drives() -> Vec<PathBuf> {
        vec![PathBuf::from("/")]
    }

    /// Get all available "drives" including local drives and AWS S3 profiles
    pub fn get_all_drives() -> Vec<DriveEntry> {
        #[cfg_attr(not(feature = "s3"), allow(unused_mut))]
        let mut entries: Vec<DriveEntry> = Self::get_drives()
            .into_iter()
            .map(DriveEntry::Local)
            .collect();

        #[cfg(feature = "s3")]
        for profile in crate::s3::list_aws_profiles() {
            entries.push(DriveEntry::S3Profile(profile.name));
        }

        entries
    }
}

#[cfg(windows)]
pub(super) fn windows_drive_paths_from_mask(mask: u32) -> Vec<PathBuf> {
    let mut drives = Vec::new();
    for bit in 0..26 {
        if (mask & (1 << bit)) != 0 {
            let drive_letter = (b'A' + bit as u8) as char;
            drives.push(PathBuf::from(format!("{drive_letter}:\\")));
        }
    }
    drives
}

/// Represents a drive entry (local or S3)
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum DriveEntry {
    Local(PathBuf),
    S3Profile(String),
}

impl DriveEntry {
    #[must_use]
    pub fn display(&self) -> String {
        match self {
            Self::Local(path) => path.display().to_string(),
            Self::S3Profile(name) => format!("S3: {name}"),
        }
    }
}
