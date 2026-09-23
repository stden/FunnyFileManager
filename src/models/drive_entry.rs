//! Purpose: `DriveEntry` and the drive discovery behind the Alt+F1 / Alt+F2 menus:
//! the local drives, the S3 profiles, and the `NetBox` and Process list rows.

use std::path::PathBuf;

use super::App;

impl App {
    /// Get available drives (Windows) or mount points (Unix)
    #[cfg(windows)]
    pub fn get_drives() -> Vec<PathBuf> {
        windows_drive_paths_from_mask(crate::platform::logical_drives())
    }

    #[cfg(not(windows))]
    pub fn get_drives() -> Vec<PathBuf> {
        vec![PathBuf::from("/")]
    }

    /// Every row of the Change drive dialog: local drives, AWS S3 profiles,
    /// then the two plugin rows Far lists after them.
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

        entries.push(DriveEntry::NetBox);
        entries.push(DriveEntry::ProcessList);
        entries
    }
}

#[cfg(windows)]
pub(super) fn windows_drive_paths_from_mask(mask: u32) -> Vec<PathBuf> {
    let mut drives = Vec::new();
    for bit in 0..26_u8 {
        if (mask & (1 << bit)) != 0 {
            let drive_letter = char::from(b'A' + bit);
            drives.push(PathBuf::from(format!("{drive_letter}:\\")));
        }
    }
    drives
}

/// What a row of the Change drive dialog opens: a filesystem, or one of the
/// plugins Far lists under the rule.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DriveEntry {
    Local(PathBuf),
    S3Profile(String),
    /// `NetBox`'s stored-sessions panel.
    NetBox,
    /// The Task list dialog (`Ctrl+W`).
    ProcessList,
}

impl DriveEntry {
    #[must_use]
    pub fn display(&self) -> String {
        match self {
            Self::Local(path) => path.display().to_string(),
            Self::S3Profile(name) => format!("S3: {name}"),
            Self::NetBox => "NetBox".to_owned(),
            Self::ProcessList => "Process list".to_owned(),
        }
    }
}
