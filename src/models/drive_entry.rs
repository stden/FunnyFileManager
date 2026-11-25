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
        let entries = Self::get_drives().into_iter().map(DriveEntry::Local);
        #[cfg(feature = "s3")]
        let entries = entries.chain(
            crate::s3::list_aws_profiles()
                .into_iter()
                .map(|profile| DriveEntry::S3Profile(profile.name)),
        );
        entries
            .chain([
                DriveEntry::NetBox,
                DriveEntry::FtpConnect,
                DriveEntry::ConnectNetworkDrives,
                DriveEntry::JiraIssues,
                DriveEntry::McpServices,
                DriveEntry::AiAssistant,
                DriveEntry::ProcessList,
            ])
            .collect()
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
    /// Prompt to connect to any FTP or FTPS server and save its profile.
    FtpConnect,
    /// Connect all user-configured `network_drives:` from `ffm.yml`.
    ConnectNetworkDrives,
    /// Open the Jira issue browser & exporter dialog.
    JiraIssues,
    /// Open the Streamable HTTP MCP services dialog.
    McpServices,
    /// Open the AI assistant dialog.
    AiAssistant,
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
            Self::FtpConnect => "FTP connect".to_owned(),
            Self::ConnectNetworkDrives => "Connect network drives".to_owned(),
            Self::JiraIssues => "Jira issues".to_owned(),
            Self::McpServices => "MCP services".to_owned(),
            Self::AiAssistant => "AI assistant".to_owned(),
            Self::ProcessList => "Process list".to_owned(),
        }
    }
}
